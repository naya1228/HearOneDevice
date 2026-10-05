package com.naya.hod

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack

/**
 * 무선으로 불규칙하게 도착하는 조각을 잠깐 모았다가 일정한 속도로 재생한다.
 * - prebuffer 만큼 모이기 전엔 재생을 시작하지 않음 (끊김 방지)
 * - 1초 동안의 최저 수위가 target 보다 높으면 그만큼 버림 (지연 누적·시계 차이 보정)
 * - max 를 넘게 쌓이면 즉시 오래된 조각을 버림 (몰려온 데이터 대비)
 */
class JitterPlayer(private val rate: Int, private val channels: Int = 1) {
    // 아래 수치와 queued는 샘플 수 (스테레오면 L·R 각각 하나씩 = 프레임당 2)
    private val perMs = rate * channels / 1000
    private val prebuffer = perMs * 60  // 60ms
    private val target = perMs * 80     // 80ms
    private val max = perMs * 200       // 200ms

    private val lock = Object()
    private val queue = ArrayDeque<ShortArray>()
    private var queued = 0 // 샘플 수

    @Volatile private var running = false
    private var thread: Thread? = null

    // 상태 표시용 통계
    @Volatile var underruns = 0; private set
    @Volatile var droppedMs = 0; private set
    val bufferedMs: Int get() = synchronized(lock) { queued / perMs }
    /** AudioTrack 내부 버퍼 (지터 버퍼 뒤에 추가로 붙는 지연) */
    @Volatile var trackMs = 0; private set

    fun push(pcm: ShortArray) {
        if (pcm.isEmpty()) return
        synchronized(lock) {
            queue.addLast(pcm)
            queued += pcm.size
            while (queued > max && queue.size > 1) {
                val old = queue.removeFirst()
                queued -= old.size
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
        // 기기가 주는 최소 버퍼(이 폰은 120ms)는 크다 → 실제로 쓰는 양만 40ms로 줄임 (기기가 더 작게는 안 줄여줌)
        track.setBufferSizeInFrames(rate * 40 / 1000)
        track.play()
        trackMs = track.bufferSizeInFrames * 1000 / rate

        thread = Thread({
            var primed = false
            var windowMin = Int.MAX_VALUE
            var windowStart = System.nanoTime()
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
                            windowMin = minOf(windowMin, queued)
                            if (System.nanoTime() - windowStart > 1_000_000_000L) {
                                // 스테레오면 L·R 짝이 안 깨지게 채널 수의 배수로
                                if (windowMin > target) trim((windowMin - target) / channels * channels)
                                windowMin = Int.MAX_VALUE
                                windowStart = System.nanoTime()
                            }
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

    // 앞(오래된 쪽)에서 n 샘플을 버림. lock 안에서 호출. 마지막 조각 하나는 남김
    private fun trim(n: Int) {
        var left = n
        while (left > 0 && queue.size > 1) {
            val head = queue.removeFirst()
            if (head.size <= left) {
                left -= head.size
                queued -= head.size
            } else {
                queue.addFirst(head.copyOfRange(left, head.size))
                queued -= left
                left = 0
            }
        }
        droppedMs += (n - left) / perMs
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
