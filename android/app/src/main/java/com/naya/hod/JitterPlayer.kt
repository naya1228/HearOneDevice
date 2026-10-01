package com.naya.hod

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack

/**
 * 무선으로 불규칙하게 도착하는 조각을 잠깐 모았다가 일정한 속도로 재생한다.
 * - PREBUFFER 만큼 모이기 전엔 재생을 시작하지 않음 (끊김 방지)
 * - MAX 를 넘게 쌓이면 오래된 조각을 버림 (지연 누적 방지)
 */
class JitterPlayer {
    private val rate = Protocol.SAMPLE_RATE
    private val prebuffer = rate * 100 / 1000 // 100ms
    private val max = rate * 300 / 1000       // 300ms

    private val lock = Object()
    private val queue = ArrayDeque<ShortArray>()
    private var queued = 0 // 샘플 수

    @Volatile private var running = false
    private var thread: Thread? = null

    // 상태 표시용 통계
    @Volatile var underruns = 0; private set
    @Volatile var droppedMs = 0; private set
    val bufferedMs: Int get() = synchronized(lock) { queued * 1000 / rate }

    fun push(pcm: ShortArray) {
        if (pcm.isEmpty()) return
        synchronized(lock) {
            queue.addLast(pcm)
            queued += pcm.size
            while (queued > max && queue.size > 1) {
                val old = queue.removeFirst()
                queued -= old.size
                droppedMs += old.size * 1000 / rate
            }
            lock.notifyAll()
        }
    }

    fun start() {
        if (running) return
        running = true
        underruns = 0
        droppedMs = 0
        val minBuf = AudioTrack.getMinBufferSize(
            rate, AudioFormat.CHANNEL_OUT_MONO, AudioFormat.ENCODING_PCM_16BIT
        )
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
                    .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                    .build()
            )
            .setBufferSizeInBytes(minBuf)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
            .build()
        track.play()

        thread = Thread({
            var primed = false
            while (running) {
                val chunk = synchronized(lock) {
                    when {
                        !primed && queued < prebuffer -> { lock.wait(20); null }
                        queue.isEmpty() -> {
                            // 다 써버림 → 다시 PREBUFFER 만큼 모일 때까지 대기
                            primed = false
                            underruns++
                            lock.wait(20)
                            null
                        }
                        else -> {
                            primed = true
                            queue.removeFirst().also { queued -= it.size }
                        }
                    }
                } ?: continue
                track.write(chunk, 0, chunk.size) // 재생 버퍼가 차면 여기서 기다림
            }
            track.stop()
            track.release()
        }, "JitterPlayer").apply { start() }
    }

    fun stop() {
        running = false
        synchronized(lock) {
            queue.clear()
            queued = 0
            lock.notifyAll()
        }
        thread?.join(500)
        thread = null
    }
}
