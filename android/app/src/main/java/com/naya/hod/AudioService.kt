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

/**
 * PC를 찾아 BLE로 연결하고, 받은 소리를 JitterPlayer로 재생한다.
 * 포그라운드 서비스(알림바 표시)라서 화면이 꺼져도 안드로이드가 멈추지 않는다.
 * 권한은 MainActivity에서 받은 뒤에만 시작된다.
 */
@SuppressLint("MissingPermission")
class AudioService : Service() {

    companion object {
        const val ACTION_START = "com.naya.hod.START"
        const val ACTION_STOP = "com.naya.hod.STOP"
        /** 연결할 PC 번호 (16진수 8자리). 없으면 시작하지 않음 (기록에 없는 PC는 찾지 않는다) */
        const val EXTRA_ID = "id"
        private const val CHANNEL_ID = "playback"
        private const val NOTIFICATION_ID = 1
        // PC는 5초 안에 답이 없으면 실패로 봄 (docs/PROTOCOL.md 5절). 폰은 여유를 두고 기다림
        private const val AUTH_TIMEOUT_MS = 10_000L

        @Volatile var isRunning = false; private set
        @Volatile var status = "정지됨"; private set
        /** 화면에 상태를 보여주기 위한 콜백 (MainActivity가 등록) */
        @Volatile var onStatus: ((String) -> Unit)? = null
        /** 재생 중 상세 숫자 (코덱·MTU·버퍼·손실 등). 재생 중이 아니면 null. 개발 모드에서만 화면에 보임 */
        @Volatile var stats: String? = null; private set
        @Volatile var onStats: ((String?) -> Unit)? = null
        /** 재생 중인 소리 (MainActivity의 비주얼라이저가 읽음) */
        val tap = AudioTap()
    }

    private val handler = Handler(Looper.getMainLooper())
    // PC가 보내는 코덱에 맞춰 바뀐다 (switchCodec). 첫 패킷이 오기 전엔 null
    private val playLock = Any()
    @Volatile private var decoder: Codecs.Decoder? = null
    @Volatile private var player: JitterPlayer? = null
    private var gatt: BluetoothGatt? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var scanning = false
    private var targetId: String? = null
    // 연결 확인 (docs/PROTOCOL.md 5절). 통과하기 전에 온 소리는 버림
    private var auth: Auth? = null
    @Volatile private var authed = false

    // 통계
    private var mtu = 23
    private var lastSeq = -1
    private var lost = 0
    private var unknownCodec = 0 // PC가 이 앱이 모르는 코덱으로 보내면 그 번호
    private var bytesThisSecond = 0
    // 연결(구독 성공)·해제 순간에 알림음
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
            // 재생 중에 다른 PC의 QR을 찍으면 그 PC로 갈아탐
            if (id != targetId) {
                targetId = id
                stopScan()
                gatt?.disconnect() // 끊김 처리(onConnectionStateChange)에서 새 번호로 다시 찾음
                if (gatt == null) scan()
            }
            return
        }
        targetId = id
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

    // ---------- 1. PC 찾기 ----------

    private fun scan() {
        if (!isRunning || scanning) return
        val scanner = bluetooth?.bluetoothLeScanner
        if (scanner == null || !bluetooth.isEnabled) {
            setStatus("블루투스가 꺼져 있음. 켜면 자동으로 다시 시도")
            handler.postDelayed({ scan() }, 3000)
            return
        }
        lastSeq = -1
        unknownCodec = 0
        val id = targetId ?: return
        setStatus("${pcLabel()} 찾는 중...")
        // QR로 받은 주소에서 우리 서비스를 광고하는 PC만 (docs/PROTOCOL.md 1절)
        val addr = SavedPcs.find(this, id)?.addr?.ifEmpty { null }
            ?: return stopWith("이 PC는 QR을 다시 찍어야 해요")
        val filter = ScanFilter.Builder()
            .setDeviceAddress(addr)
            .setServiceUuid(ParcelUuid(Protocol.SERVICE_UUID))
            .build()
        val settings = ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()
        scanner.startScan(listOf(filter), settings, scanCallback)
        scanning = true
    }

    private fun stopScan() {
        if (!scanning) return
        scanning = false
        runCatching { bluetooth?.bluetoothLeScanner?.stopScan(scanCallback) }
    }

    private val scanCallback = object : ScanCallback() {
        override fun onScanResult(callbackType: Int, result: ScanResult) {
            if (!scanning) return
            stopScan()
            setStatus("${pcLabel()} 발견, 연결 중...")
            gatt = result.device.connectGatt(
                this@AudioService, false, gattCallback, BluetoothDevice.TRANSPORT_LE
            )
        }

        override fun onScanFailed(errorCode: Int) {
            scanning = false
            setStatus("스캔 실패 (코드 $errorCode). 다시 시도")
            handler.postDelayed({ scan() }, 3000)
        }
    }

    // ---------- 2. 연결 → MTU → 서비스 찾기 → 구독 ----------

    private val gattCallback = object : BluetoothGattCallback() {
        override fun onConnectionStateChange(g: BluetoothGatt, status: Int, newState: Int) {
            if (newState == BluetoothProfile.STATE_CONNECTED) {
                setStatus("연결됨, 설정 중...")
                g.requestConnectionPriority(BluetoothGatt.CONNECTION_PRIORITY_HIGH)
                // 한 번에 보낼 수 있는 크기를 최대로 요청 (실제 값은 onMtuChanged)
                g.requestMtu(517)
            } else if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                g.close()
                if (gatt == g) gatt = null
                resetAuth()
                playing = false
                if (isRunning) {
                    setStatus("끊김, 다시 찾는 중...")
                    handler.postDelayed({ scan() }, 1000)
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
                setStatus("PC 앱 버전이 맞지 않음")
                g.disconnect()
                return
            }
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

        // 구독 성공 → PC가 확인 문제를 보내옴 (onControl)
        override fun onDescriptorWrite(g: BluetoothGatt, d: BluetoothGattDescriptor, status: Int) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                setStatus("구독 실패 (코드 $status)")
                return
            }
            // 확인 상태는 메인 스레드에서만 만짐 (onControl과 같은 곳)
            handler.post {
                val key = targetId?.let { SavedPcs.find(this@AudioService, it) }?.key?.ifEmpty { null }
                    ?: return@post stopWith("이 PC는 QR을 다시 찍어야 해요")
                auth = Auth(Protocol.hexBytes(key))
                setStatus("${pcLabel()} 확인 중...")
                handler.postDelayed(authTimeout, AUTH_TIMEOUT_MS)
            }
        }

        // ---------- 3. 소리 받기 ----------

        // Android 13+
        override fun onCharacteristicChanged(
            g: BluetoothGatt, ch: BluetoothGattCharacteristic, value: ByteArray
        ) = onAudio(value)

        // Android 12 이하
        @Deprecated("Android 12 이하용")
        @Suppress("DEPRECATION")
        override fun onCharacteristicChanged(g: BluetoothGatt, ch: BluetoothGattCharacteristic) =
            onAudio(ch.value)
    }

    private fun onAudio(data: ByteArray) {
        val packet = Protocol.parse(data) ?: return
        if (packet.codec == Protocol.CONTROL) {
            onControl(packet)
            return
        }
        if (!authed) return
        if (lastSeq >= 0) {
            val gap = (packet.seq - lastSeq - 1) and 0xFFFF
            if (gap in 1..1000) lost += gap
        }
        lastSeq = packet.seq
        bytesThisSecond += data.size
        synchronized(playLock) {
            if (!isRunning) return
            val dec = decoder?.takeIf { it.id == packet.codec } ?: switchCodec(packet.codec) ?: return
            player?.push(dec.decode(packet.data, Protocol.HEADER_LEN))
        }
    }

    // PC가 보낸 제어 메시지 (docs/PROTOCOL.md 4절). 멈추는 경우는 정지 버튼과 똑같이 (다시 찾지 않음)
    private fun onControl(packet: Protocol.Packet) {
        val cmd = packet.data.getOrNull(Protocol.HEADER_LEN)?.toInt() ?: return
        val body = packet.data.copyOfRange(Protocol.HEADER_LEN + 1, packet.data.size)
        handler.post {
            when (cmd) {
                Protocol.CONTROL_STOP -> stopWith("PC에서 공유를 중지했어요")
                Protocol.CONTROL_CHALLENGE -> answer(body)
                Protocol.CONTROL_AUTH_OK -> {
                    if (auth?.checkPc(body) == true) {
                        handler.removeCallbacks(authTimeout)
                        authed = true
                        playing = true
                        setStatus("${pcLabel()} 연결됨")
                    } else {
                        stopWith("PC 확인에 실패해 끊었어요 (진짜 PC가 아닐 수 있음)")
                    }
                }
                Protocol.CONTROL_AUTH_FAIL -> stopWith("열쇠가 맞지 않아요. PC의 QR을 다시 찍어 주세요")
            }
        }
    }

    // PC 문제에 답을 오디오 특성에 씀 (응답 있는 쓰기)
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

    // 정해진 시간 안에 확인이 안 끝나면 옛 PC 앱이거나 문제가 있는 것
    private val authTimeout = Runnable { stopWith("PC가 연결 확인에 답하지 않아요. PC 앱을 업데이트해 주세요") }

    private fun resetAuth() {
        handler.removeCallbacks(authTimeout)
        auth = null
        authed = false
    }

    private fun stopWith(message: String) {
        stopAll()
        setStatus(message)
    }

    // PC에서 코덱을 바꾸면 패킷 헤더의 번호가 바뀐다 → 디코더를 바꾸고, 샘플레이트·채널이 다르면 재생기도 새로.
    // playLock 안에서 호출
    private fun switchCodec(id: Int): Codecs.Decoder? {
        // 디코더를 못 여는 경우(기기에 Opus 디코더가 없음 등)도 "모르는 코덱"으로 표시. 원인은 logcat
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

    // ---------- 상태 표시 ----------

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
                detail = "코덱 ${dec.id} ${dec.name} · MTU $mtu · %.1f KB/s\n버퍼 %dms (+재생장치 %dms) · 손실 %d · 끊김 %d회 · 버림 %dms".format(
                    kb, p.bufferedMs, p.trackMs, lost, p.underruns, p.droppedMs
                )
            }
            setStats(detail)
            bytesThisSecond = 0
            handler.postDelayed(this, 1000)
        }
    }

    // 화면에 보일 PC 이름. 기록에 이름이 있으면 이름만 (번호는 감춤)
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
