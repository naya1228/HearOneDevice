package com.naya.hod

import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.ParcelUuid
import android.os.PowerManager
import android.util.Log

// docs/FEATURES.md: 폰 찾기·연결, 재생
@SuppressLint("MissingPermission")
class AudioService : Service() {

    companion object {
        const val ACTION_START = "com.naya.hod.START"
        const val ACTION_STOP = "com.naya.hod.STOP"
        const val EXTRA_ID = "id"
        private const val CHANNEL_ID = "playback"
        private const val NOTIFICATION_ID = 1
        private const val AUTH_TIMEOUT_MS = 10_000L
        private const val COLLECT_MS = 1000L
        private const val MAX_REJECTIONS = 5
        private const val RETRY_MS = 3000L
        private const val RESCAN_MS = 1000L
        private const val STATS_EVERY_MS = 1000L
        private const val MAX_SEQ_GAP = 1000

        @Volatile var isRunning = false; private set
        @Volatile var status = "정지됨"; private set
        @Volatile var onStatus: ((String) -> Unit)? = null
        @Volatile var stats: String? = null; private set
        @Volatile var onStats: ((String?) -> Unit)? = null
        val tap = AudioTap()
    }

    private val handler = Handler(Looper.getMainLooper())
    private val playLock = Any()
    @Volatile private var decoder: Codecs.Decoder? = null
    @Volatile private var player: JitterPlayer? = null
    private var gatt: BluetoothGatt? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var scanning = false
    private var targetId: String? = null
    private val candidates = Candidates()
    private val seenDevices = mutableMapOf<String, BluetoothDevice>()
    private var connectingAddr: String? = null
    private var passed = false
    private var leavingRejected = false
    @Volatile private var auth: Auth? = null
    @Volatile private var unsealer: Unsealer? = null
    private var unsealFailed = 0

    private var mtu = 23
    private var lastSeq = -1
    private var lost = 0
    private var unknownCodec = 0
    private var bytesThisSecond = 0
    private var playing = false
        set(value) {
            if (value != field) if (value) Chime.connected() else Chime.disconnected()
            field = value
        }

    private val bluetooth by lazy { getSystemService(BluetoothManager::class.java).adapter }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> intent.getStringExtra(EXTRA_ID)?.let { start(it) }
            ACTION_STOP -> stopAll()
        }
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        stopAll()
        super.onDestroy()
    }

    private fun start(id: String) {
        if (isRunning) {
            if (id != targetId) {
                targetId = id
                candidates.reset()
                stopScan()
                gatt?.disconnect()
                if (gatt == null) scan()
            }
            return
        }
        targetId = id
        candidates.reset()
        isRunning = true
        startForegroundCompat(notification("연결 준비 중"))
        wakeLock = getSystemService(PowerManager::class.java)
            .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "HearOneDevice:playback")
            .apply { acquire() }
        handler.post(statsTick)
        scan()
    }

    private fun stopAll() {
        if (!isRunning) return
        isRunning = false
        handler.removeCallbacksAndMessages(null)
        stopScan()
        gatt?.let { it.disconnect(); it.close() }
        gatt = null
        synchronized(playLock) {
            player?.stop()
            player = null
            decoder?.close()
            decoder = null
        }
        wakeLock?.let { if (it.isHeld) it.release() }
        wakeLock = null
        resetAuth()
        playing = false
        setStats(null)
        setStatus("정지됨")
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    // docs/PROTOCOL.md: 찾기
    private fun scan() {
        if (!isRunning || scanning) return
        val scanner = bluetooth?.bluetoothLeScanner
        if (scanner == null || !bluetooth.isEnabled) {
            setStatus("블루투스가 꺼져 있음. 켜면 자동으로 다시 시도")
            handler.postDelayed({ scan() }, RETRY_MS)
            return
        }
        lastSeq = -1
        unknownCodec = 0
        unsealFailed = 0
        if (targetId == null) return
        setStatus("${pcLabel()} 찾는 중...")
        candidates.newRound()
        seenDevices.clear()
        val filter = ScanFilter.Builder().setServiceUuid(ParcelUuid(Protocol.SERVICE_UUID)).build()
        val settings = ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()
        scanner.startScan(listOf(filter), settings, scanCallback)
        scanning = true
        handler.postDelayed(pickCandidate, COLLECT_MS)
    }

    private fun stopScan() {
        handler.removeCallbacks(pickCandidate)
        if (!scanning) return
        scanning = false
        runCatching { bluetooth?.bluetoothLeScanner?.stopScan(scanCallback) }
    }

    private val pickCandidate: Runnable = Runnable {
        if (!scanning) return@Runnable
        val device = candidates.pick()?.let { seenDevices[it] }
        if (device == null) {
            handler.postDelayed(pickCandidate, COLLECT_MS)
            return@Runnable
        }
        connectTo(device)
    }

    private fun connectTo(device: BluetoothDevice) {
        stopScan()
        connectingAddr = device.address
        passed = false
        setStatus("${pcLabel()} 후보 발견, 연결 중...")
        gatt = device.connectGatt(this, false, gattCallback, BluetoothDevice.TRANSPORT_LE)
    }

    private fun rejectCurrent() {
        connectingAddr?.let { candidates.reject(it) }
        resetAuth()
        if (candidates.rejections >= MAX_REJECTIONS) {
            stopWith("${pcLabel()}을(를) 찾지 못했어요. 계속 안 되면 PC의 QR을 다시 찍어 주세요")
            return
        }
        leavingRejected = true
        setStatus("다른 PC였어요. 다시 찾는 중...")
        val g = gatt
        if (g != null) g.disconnect() else scan()
    }

    private val scanCallback = object : ScanCallback() {
        override fun onScanResult(callbackType: Int, result: ScanResult) {
            if (!scanning) return
            val addr = result.device.address
            seenDevices[addr] = result.device
            candidates.offer(addr, result.rssi)
            if (addr == candidates.lastGood) connectTo(result.device)
        }

        override fun onScanFailed(errorCode: Int) {
            scanning = false
            setStatus("스캔 실패 (코드 $errorCode). 다시 시도")
            handler.postDelayed({ scan() }, RETRY_MS)
        }
    }

    // docs/PROTOCOL.md: 연결
    private val gattCallback = object : BluetoothGattCallback() {
        override fun onConnectionStateChange(g: BluetoothGatt, status: Int, newState: Int) {
            if (newState == BluetoothProfile.STATE_CONNECTED) {
                setStatus("연결됨, 설정 중...")
                g.requestConnectionPriority(BluetoothGatt.CONNECTION_PRIORITY_HIGH)
                g.requestMtu(Protocol.MTU_REQUEST)
            } else if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                g.close()
                if (gatt == g) gatt = null
                resetAuth()
                playing = false
                handler.post {
                    if (!isRunning) return@post
                    val addr = connectingAddr
                    if (addr != null && !passed && !leavingRejected) candidates.unreachable(addr)
                    if (!leavingRejected) setStatus("끊김, 다시 찾는 중...")
                    leavingRejected = false
                    handler.postDelayed({ scan() }, RESCAN_MS)
                }
            }
        }

        override fun onMtuChanged(g: BluetoothGatt, newMtu: Int, status: Int) {
            mtu = newMtu
            g.discoverServices()
        }

        override fun onServicesDiscovered(g: BluetoothGatt, status: Int) {
            val ch = g.getService(Protocol.SERVICE_UUID)?.getCharacteristic(Protocol.AUDIO_CHAR_UUID)
            val desc = ch?.getDescriptor(Protocol.CCCD_UUID)
            if (ch == null || desc == null) {
                handler.post { rejectCurrent() }
                return
            }
            val key = targetId?.let { SavedPcs.find(this@AudioService, it) }?.key?.ifEmpty { null }
            if (key == null) {
                handler.post { stopWith("이 PC는 QR을 다시 찍어야 해요") }
                return
            }
            auth = Auth(Protocol.hexBytes(key))
            g.setCharacteristicNotification(ch, true)
            val enable = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
            if (Build.VERSION.SDK_INT >= 33) {
                g.writeDescriptor(desc, enable)
            } else {
                @Suppress("DEPRECATION")
                desc.value = enable
                @Suppress("DEPRECATION")
                g.writeDescriptor(desc)
            }
        }

        override fun onDescriptorWrite(g: BluetoothGatt, d: BluetoothGattDescriptor, status: Int) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                setStatus("구독 실패 (코드 $status)")
                return
            }
            handler.post {
                setStatus("${pcLabel()} 확인 중...")
                handler.postDelayed(authTimeout, AUTH_TIMEOUT_MS)
            }
        }

        override fun onCharacteristicChanged(
            g: BluetoothGatt, ch: BluetoothGattCharacteristic, value: ByteArray
        ) = onAudio(value)

        @Deprecated("Android 12 이하용")
        @Suppress("DEPRECATION")
        override fun onCharacteristicChanged(g: BluetoothGatt, ch: BluetoothGattCharacteristic) =
            onAudio(ch.value)
    }

    // docs/PROTOCOL.md: 패킷, 잠그기
    private fun onAudio(data: ByteArray) {
        val raw = Protocol.parse(data) ?: return
        val open = unsealer
        val packet = if (open == null) raw else open.open(raw) ?: run { unsealFailed++; return }
        if (packet.codec == Protocol.CONTROL) {
            onControl(packet, sealed = open != null)
            return
        }
        if (open == null) return
        if (lastSeq >= 0) {
            val gap = (packet.seq - lastSeq - 1) and 0xFFFF
            if (gap in 1..MAX_SEQ_GAP) lost += gap
        }
        lastSeq = packet.seq
        bytesThisSecond += data.size
        synchronized(playLock) {
            if (!isRunning) return
            val dec = decoder?.takeIf { it.id == packet.codec } ?: switchCodec(packet.codec) ?: return
            player?.push(dec.decode(packet.data, Protocol.HEADER_LEN))
        }
    }

    // docs/PROTOCOL.md: 제어 메시지, 연결 확인
    private fun onControl(packet: Protocol.Packet, sealed: Boolean) {
        val cmd = packet.data.getOrNull(Protocol.HEADER_LEN)?.toInt() ?: return
        val body = packet.data.copyOfRange(Protocol.HEADER_LEN + 1, packet.data.size)
        if (sealed) {
            if (cmd == Protocol.CONTROL_STOP) handler.post { stopWith("PC에서 공유를 중지했어요") }
            return
        }
        when (cmd) {
            Protocol.CONTROL_CHALLENGE -> answer(body)
            Protocol.CONTROL_AUTH_OK -> {
                val a = auth
                val key = a?.takeIf { it.checkPc(body) }?.audioKey()
                if (key == null) {
                    handler.post { rejectCurrent() }
                    return
                }
                unsealer = Unsealer(key, packet.seq.toLong())
                handler.post {
                    handler.removeCallbacks(authTimeout)
                    passed = true
                    connectingAddr?.let { candidates.accept(it) }
                    playing = true
                    setStatus("${pcLabel()} 연결됨")
                }
            }
            Protocol.CONTROL_AUTH_FAIL -> handler.post { rejectCurrent() }
        }
    }

    private fun answer(challenge: ByteArray) {
        val g = gatt ?: return
        val reply = auth?.reply(challenge) ?: return
        val ch = g.getService(Protocol.SERVICE_UUID)?.getCharacteristic(Protocol.AUDIO_CHAR_UUID) ?: return
        val type = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
        if (Build.VERSION.SDK_INT >= 33) {
            g.writeCharacteristic(ch, reply, type)
        } else {
            @Suppress("DEPRECATION")
            ch.value = reply
            ch.writeType = type
            @Suppress("DEPRECATION")
            g.writeCharacteristic(ch)
        }
    }

    private val authTimeout = Runnable { rejectCurrent() }

    private fun resetAuth() {
        handler.removeCallbacks(authTimeout)
        auth = null
        unsealer = null
    }

    private fun stopWith(message: String) {
        stopAll()
        setStatus(message)
    }

    // docs/CODECS.md: 코덱 바꾸는 법
    private fun switchCodec(id: Int): Codecs.Decoder? {
        val dec = runCatching { Codecs.decoder(id) }.onFailure { Log.e("HOD", "디코더 $id 열기 실패", it) }.getOrNull()
        if (dec == null) {
            unknownCodec = id
            return null
        }
        unknownCodec = 0
        if (dec.sampleRate != decoder?.sampleRate || dec.channels != decoder?.channels) {
            player?.stop()
            player = JitterPlayer(dec.sampleRate, dec.channels, tap).also { it.start() }
        }
        decoder?.close()
        decoder = dec
        return dec
    }

    // docs/FEATURES.md: 폰 화면
    private val statsTick = object : Runnable {
        override fun run() {
            val dec = decoder
            val p = player
            var detail: String? = null
            if (unknownCodec != 0) {
                setStatus("PC가 보낸 코덱 ${unknownCodec}번을 이 앱이 모름. 앱 업데이트 필요 (docs/CODECS.md)", updateNotification = false)
            } else if (playing && dec != null && p != null) {
                setStatus("${pcLabel()} 재생 중", updateNotification = false)
                val kb = bytesThisSecond / 1024.0
                detail = "코덱 ${dec.id} ${dec.name} · MTU $mtu · %.1f KB/s\n버퍼 %dms (+재생장치 %dms) · 손실 %d · 끊김 %d회 · 버림 %dms · 풀기 실패 %d".format(
                    kb, p.bufferedMs, p.trackMs, lost, p.underruns, p.droppedMs, unsealFailed
                )
            }
            setStats(detail)
            bytesThisSecond = 0
            handler.postDelayed(this, STATS_EVERY_MS)
        }
    }

    private fun pcLabel(): String {
        val id = targetId ?: return "PC"
        return SavedPcs.find(this, id)?.name?.ifBlank { null } ?: "PC $id"
    }

    private fun setStats(text: String?) {
        if (text == stats) return
        stats = text
        onStats?.invoke(text)
    }

    private fun setStatus(text: String, updateNotification: Boolean = true) {
        status = text
        onStatus?.invoke(text)
        if (updateNotification && isRunning) {
            getSystemService(NotificationManager::class.java)
                .notify(NOTIFICATION_ID, notification(text.lineSequence().first()))
        }
    }

    private fun notification(text: String): Notification {
        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(
            NotificationChannel(CHANNEL_ID, "재생", NotificationManager.IMPORTANCE_LOW)
        )
        val open = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE
        )
        val stop = PendingIntent.getService(
            this, 1, Intent(this, AudioService::class.java).setAction(ACTION_STOP),
            PendingIntent.FLAG_IMMUTABLE
        )
        return Notification.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle("HearOneDevice")
            .setContentText(text)
            .setContentIntent(open)
            .addAction(Notification.Action.Builder(null, "정지", stop).build())
            .setOngoing(true)
            .build()
    }

    private fun startForegroundCompat(n: Notification) {
        if (Build.VERSION.SDK_INT >= 29) {
            startForeground(
                NOTIFICATION_ID, n,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE or
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK
            )
        } else {
            startForeground(NOTIFICATION_ID, n)
        }
    }
}
