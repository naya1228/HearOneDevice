package com.naya.hod

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack

// docs/FEATURES.md: 재생
class JitterPlayer(
    private val rate: Int,
    private val channels: Int = 1,
    private val tap: AudioTap? = null,
) {
    private val perMs = rate * channels / 1000
    private val prebufferSamples = perMs * PREBUFFER_MS
    private val targetSamples = perMs * TARGET_MS
    private val maxSamples = perMs * MAX_MS

    private val lock = Object()
    private val queue = ArrayDeque<ShortArray>()
    private var queuedSamples = 0

    @Volatile private var running = false
    private var thread: Thread? = null

    @Volatile var underruns = 0; private set
    @Volatile var droppedMs = 0; private set
    val bufferedMs: Int get() = synchronized(lock) { queuedSamples / perMs }
    @Volatile var trackMs = 0; private set

    fun push(pcm: ShortArray) {
        if (pcm.isEmpty()) return
        synchronized(lock) {
            queue.addLast(pcm)
            queuedSamples += pcm.size
            while (queuedSamples > maxSamples && queue.size > 1) {
                val old = queue.removeFirst()
                queuedSamples -= old.size
                droppedMs += old.size / perMs
            }
            lock.notifyAll()
        }
    }

    fun start() {
        if (running) return
        running = true
        underruns = 0
        droppedMs = 0
        val mask = if (channels == 2) AudioFormat.CHANNEL_OUT_STEREO else AudioFormat.CHANNEL_OUT_MONO
        val minBuf = AudioTrack.getMinBufferSize(rate, mask, AudioFormat.ENCODING_PCM_16BIT)
        val track = AudioTrack.Builder()
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                    .build()
            )
            .setAudioFormat(
                AudioFormat.Builder()
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                    .setSampleRate(rate)
                    .setChannelMask(mask)
                    .build()
            )
            .setBufferSizeInBytes(minBuf)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
            .build()
        track.setBufferSizeInFrames(rate * TRACK_MS / 1000)
        track.play()
        trackMs = track.bufferSizeInFrames * 1000 / rate

        thread = Thread({
            var primed = false
            var windowMinSamples = Int.MAX_VALUE
            var windowStart = System.nanoTime()
            while (running) {
                val chunk = synchronized(lock) {
                    when {
                        !primed && queuedSamples < prebufferSamples -> { lock.wait(WAIT_MS); null }
                        queue.isEmpty() -> {
                            primed = false
                            underruns++
                            lock.wait(WAIT_MS)
                            null
                        }
                        else -> {
                            primed = true
                            windowMinSamples = minOf(windowMinSamples, queuedSamples)
                            if (System.nanoTime() - windowStart > TRIM_WINDOW_NS) {
                                if (windowMinSamples > targetSamples) trimLocked((windowMinSamples - targetSamples) / channels * channels)
                                windowMinSamples = Int.MAX_VALUE
                                windowStart = System.nanoTime()
                            }
                            queue.removeFirst().also { queuedSamples -= it.size }
                        }
                    }
                } ?: continue
                tap?.write(chunk, channels, rate)
                track.write(chunk, 0, chunk.size)
            }
            track.stop()
            track.release()
        }, "JitterPlayer").apply { start() }
    }

    private fun trimLocked(n: Int) {
        var left = n
        while (left > 0 && queue.size > 1) {
            val head = queue.removeFirst()
            if (head.size <= left) {
                left -= head.size
                queuedSamples -= head.size
            } else {
                queue.addFirst(head.copyOfRange(left, head.size))
                queuedSamples -= left
                left = 0
            }
        }
        droppedMs += (n - left) / perMs
    }

    fun stop() {
        running = false
        synchronized(lock) {
            queue.clear()
            queuedSamples = 0
            lock.notifyAll()
        }
        thread?.join(STOP_JOIN_MS)
        thread = null
    }

    private companion object {
        const val PREBUFFER_MS = 60
        const val TARGET_MS = 80
        const val MAX_MS = 200
        const val TRACK_MS = 40
        const val TRIM_WINDOW_NS = 1_000_000_000L
        const val WAIT_MS = 20L
        const val STOP_JOIN_MS = 500L
    }
}
