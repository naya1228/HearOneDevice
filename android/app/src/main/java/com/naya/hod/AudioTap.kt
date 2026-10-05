package com.naya.hod

/**
 * 재생 직전의 소리를 최근 size개만 모노로 보관한다 (비주얼라이저용).
 * 재생 스레드(JitterPlayer)가 쓰고 화면 스레드(CircleVisualizer)가 읽는다.
 */
class AudioTap(private val size: Int = 2048) {
    private val buf = FloatArray(size)
    private var pos = 0
    @Volatile private var lastWrite = 0L
    @Volatile var sampleRate = 48000; private set

    /** pcm: 16bit, 스테레오면 L R L R ... → 모노로 섞어 보관 */
    fun write(pcm: ShortArray, channels: Int, rate: Int) {
        sampleRate = rate
        synchronized(buf) {
            var i = 0
            while (i + channels <= pcm.size) {
                var sum = 0
                for (c in 0 until channels) sum += pcm[i + c]
                buf[pos] = sum / (channels * 32768f)
                pos = (pos + 1) % size
                i += channels
            }
        }
        lastWrite = System.nanoTime()
    }

    /** 가장 최근 out.size개(오래된 것부터)를 채운다. 0.2초 넘게 새 소리가 없으면 false */
    fun read(out: FloatArray): Boolean {
        if (System.nanoTime() - lastWrite > 200_000_000L) return false
        synchronized(buf) {
            val n = out.size
            for (k in 0 until n) out[k] = buf[(pos - n + k + size) % size]
        }
        return true
    }
}
