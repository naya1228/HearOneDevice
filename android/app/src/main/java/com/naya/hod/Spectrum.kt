package com.naya.hod

import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.log10
import kotlin.math.max
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt

// docs/FEATURES.md: 폰 화면
class Spectrum(private val n: Int = 1024, private val bands: Int = 64) {
    private companion object {
        const val LOW_HZ = 60.0
        const val HIGH_HZ = 16000.0
        const val FLOOR_DB = 60f
    }

    init {
        require(n > 0 && n and (n - 1) == 0) { "n은 2의 거듭제곱" }
    }

    private val re = FloatArray(n)
    private val im = FloatArray(n)
    private val window = FloatArray(n) { (0.5 - 0.5 * cos(2 * PI * it / (n - 1))).toFloat() }

    fun compute(samples: FloatArray, rate: Int, out: FloatArray) {
        for (i in 0 until n) {
            re[i] = samples[i] * window[i]
            im[i] = 0f
        }
        fft()
        val lo = LOW_HZ
        val hi = minOf(HIGH_HZ, rate / 2.0)
        for (b in 0 until bands) {
            val f0 = lo * (hi / lo).pow(b.toDouble() / bands)
            val f1 = lo * (hi / lo).pow((b + 1.0) / bands)
            val k0 = (f0 * n / rate).toInt().coerceIn(1, n / 2 - 1)
            val k1 = (f1 * n / rate).toInt().coerceIn(k0 + 1, n / 2)
            var peak = 0f
            for (k in k0 until k1) peak = max(peak, re[k] * re[k] + im[k] * im[k])
            val amp = sqrt(peak) / (n / 4f)
            val db = 20 * log10(amp + 1e-9f)
            out[b] = ((db + FLOOR_DB) / FLOOR_DB).coerceIn(0f, 1f)
        }
    }

    private fun fft() {
        var j = 0
        for (i in 1 until n) {
            var bit = n shr 1
            while (j and bit != 0) {
                j = j xor bit
                bit = bit shr 1
            }
            j = j xor bit
            if (i < j) {
                re[i] = re[j].also { re[j] = re[i] }
                im[i] = im[j].also { im[j] = im[i] }
            }
        }
        var len = 2
        while (len <= n) {
            val ang = -2 * PI / len
            val wr = cos(ang).toFloat()
            val wi = sin(ang).toFloat()
            for (i in 0 until n step len) {
                var cr = 1f
                var ci = 0f
                for (k in 0 until len / 2) {
                    val a = i + k
                    val b = a + len / 2
                    val tr = re[b] * cr - im[b] * ci
                    val ti = re[b] * ci + im[b] * cr
                    re[b] = re[a] - tr
                    im[b] = im[a] - ti
                    re[a] += tr
                    im[a] += ti
                    val ncr = cr * wr - ci * wi
                    ci = cr * wi + ci * wr
                    cr = ncr
                }
            }
            len = len shl 1
        }
    }
}
