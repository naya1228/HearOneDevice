package com.naya.hod

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.os.Handler
import android.os.Looper
import kotlin.math.PI
import kotlin.math.exp
import kotlin.math.sin

/**
 * 연결·해제 알림음. 연결은 올라가는 두 음, 해제는 내려가는 두 음.
 * 미디어 소리로 내서 이어폰으로 PC 소리와 함께 들린다 (음량도 미디어 음량을 따름).
 */
object Chime {
    private const val RATE = 44100
    private const val NOTE_MS = 90
    private const val LOW = 880.0
    private const val HIGH = 1320.0

    private val handler = Handler(Looper.getMainLooper())
    private val connected by lazy { notes(LOW, HIGH) }
    private val disconnected by lazy { notes(HIGH, LOW) }

    fun connected() = play(connected)
    fun disconnected() = play(disconnected)

    private fun play(pcm: ShortArray) {
        runCatching {
            val track = AudioTrack.Builder()
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                        .build()
                )
                .setAudioFormat(
                    AudioFormat.Builder()
                        .setSampleRate(RATE)
                        .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                        .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                        .build()
                )
                .setTransferMode(AudioTrack.MODE_STATIC)
                .setBufferSizeInBytes(pcm.size * 2)
                .build()
            track.write(pcm, 0, pcm.size)
            track.play()
            // 다 울린 뒤 정리 (서비스가 먼저 끝나도 소리는 끝까지)
            handler.postDelayed({ track.release() }, NOTE_MS * 2L + 200)
        }
    }

    private fun notes(first: Double, second: Double): ShortArray {
        val n = RATE * NOTE_MS / 1000
        return ShortArray(n * 2) { i ->
            val t = (i % n).toDouble() / RATE
            val freq = if (i < n) first else second
            // 짧게 올라갔다가 부드럽게 줄어드는 "띵" (딸깍 소리 방지)
            val end = NOTE_MS / 1000.0
            val envelope = minOf(1.0, t / 0.005, (end - t) / 0.005) * exp(-t * 30)
            (sin(2 * PI * freq * t) * envelope * 0.3 * Short.MAX_VALUE).toInt().toShort()
        }
    }
}
