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
        /** 연결할 PC 번호 (16진수 8자리). 없으면 처음 발견한 PC */
        const val EXTRA_ID = "id"
        private const val CHANNEL_ID = "playback"
        private const val NOTIFICATION_ID = 1

        @Volatile var isRunning = false; private set
        @Volatile var status = "정지됨"; private set
        /** 화면에 상태를 보여주기 위한 콜백 (MainActivity가 등록) */
        @Volatile var onStatus: ((String) -> Unit)? = null
    }

    private val handler = Handler(Looper.getMainLooper())
    private val decoder = Codecs.active
    private val player = JitterPlayer(decoder.sampleRate)
    private var gatt: BluetoothGatt? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var scanning = false
    private var targetId: String? = null

    // 통계
    private var mtu = 23
    private var lastSeq = -1
    private var lost = 0
    private var wrongCodec = 0 // PC가 다른 코덱으로 보내면 그 번호
    private var bytesThisSecond = 0
    private var playing = false

    private val bluetooth by lazy { getSystemService(BluetoothManager::class.java).adapter }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> start(intent.getStringExtra(EXTRA_ID))
            ACTION_STOP -> stopAll()
        }
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        stopAll()
        super.onDestroy()
    }

    private fun start(id: String?) {
        if (isRunning) {
            // 재생 중에 다른 PC의 QR을 찍으면 그 PC로 갈아탐
            if (id != null && id != targetId) {
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
        player.start()
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
        player.stop()
        wakeLock?.let { if (it.isHeld) it.release() }
        wakeLock = null
        playing = false
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
        wrongCodec = 0
        val id = targetId
        setStatus(if (id != null) "PC $id 찾는 중..." else "PC 찾는 중...")
        val filter = ScanFilter.Builder().setServiceUuid(ParcelUuid(Protocol.SERVICE_UUID)).apply {
            // QR로 받은 번호를 광고에 싣고 있는 PC만
            if (id != null) setManufacturerData(Protocol.MANUFACTURER_ID, Protocol.idBytes(id))
        }.build()
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
            // 광고에 실린 PC 번호를 기억 → 다음엔 "듣기 시작"만 눌러도 이 PC로
            result.scanRecord?.getManufacturerSpecificData(Protocol.MANUFACTURER_ID)
                ?.takeIf { it.size == 4 }
                ?.joinToString("") { "%02x".format(it) }
                ?.let { targetId = it; MainActivity.saveLastPc(this@AudioService, it) }
            setStatus("PC ${targetId ?: ""} 발견, 연결 중...")
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

        override fun onDescriptorWrite(g: BluetoothGatt, d: BluetoothGattDescriptor, status: Int) {
            playing = status == BluetoothGatt.GATT_SUCCESS
            setStatus(if (playing) "PC ${targetId ?: ""} 연결됨 (MTU $mtu)" else "구독 실패 (코드 $status)")
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
        if (packet.codec != decoder.id) {
            wrongCodec = packet.codec
            return
        }
        if (lastSeq >= 0) {
            val gap = (packet.seq - lastSeq - 1) and 0xFFFF
            if (gap in 1..1000) lost += gap
        }
        lastSeq = packet.seq
        bytesThisSecond += data.size
        player.push(decoder.decode(packet.data, Protocol.HEADER_LEN))
    }

    // ---------- 상태 표시 ----------

    private val statsTick = object : Runnable {
        override fun run() {
            if (wrongCodec != 0) {
                setStatus("코덱 불일치: PC ${wrongCodec}번, 앱 ${decoder.id}번 (docs/CODECS.md)", updateNotification = false)
            } else if (playing) {
                val kb = bytesThisSecond / 1024.0
                setStatus(
                    "PC ${targetId ?: "?"} 재생 중 · 코덱 ${decoder.id} ${decoder.name} · MTU $mtu · %.1f KB/s\n버퍼 %dms (+재생장치 %dms) · 손실 %d · 끊김 %d회 · 버림 %dms".format(
                        kb, player.bufferedMs, player.trackMs, lost, player.underruns, player.droppedMs
                    ),
                    updateNotification = false,
                )
            }
            bytesThisSecond = 0
            handler.postDelayed(this, 1000)
        }
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
