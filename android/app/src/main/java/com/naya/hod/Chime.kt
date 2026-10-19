package com.naya.hod

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.os.Handler
import android.os.Looper
import kotlin.math.PI
import kotlin.math.exp
import kotlin.math.sin

// docs/FEATURES.md: 알림음
object Chime {
    private const val RATE = 44100
    private const val NOTE_MS = 90
    private const val LOW = 880.0
    private const val HIGH = 1320.0
    private const val VOLUME = 0.3
    private const val RAMP_S = 0.005
    private const val DECAY = 30.0
    private const val RELEASE_AFTER_MS = NOTE_MS * 2L + 200

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
            handler.postDelayed({ track.release() }, RELEASE_AFTER_MS)
        }
    }

    private fun notes(first: Double, second: Double): ShortArray {
        val n = RATE * NOTE_MS / 1000
        return ShortArray(n * 2) { i ->
            val t = (i % n).toDouble() / RATE
            val freq = if (i < n) first else second
            val end = NOTE_MS / 1000.0
            val envelope = minOf(1.0, t / RAMP_S, (end - t) / RAMP_S) * exp(-t * DECAY)
            (sin(2 * PI * freq * t) * envelope * VOLUME * Short.MAX_VALUE).toInt().toShort()
        }
    }
}
