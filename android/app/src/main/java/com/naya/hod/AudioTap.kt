package com.naya.hod

// docs/FEATURES.md: 폰 화면
class AudioTap(private val size: Int = 2048) {
    private companion object {
        const val SILENCE_NS = 200_000_000L
    }

    private val buf = FloatArray(size)
    private var pos = 0
    @Volatile private var lastWrite = 0L
    @Volatile var sampleRate = 48000; private set

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

    fun read(out: FloatArray): Boolean {
        if (System.nanoTime() - lastWrite > SILENCE_NS) return false
        synchronized(buf) {
            val n = out.size
            for (k in 0 until n) out[k] = buf[(pos - n + k + size) % size]
        }
        return true
    }
}
