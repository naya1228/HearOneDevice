package com.naya.hod

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.view.View
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin

// docs/FEATURES.md: 폰 화면
class CircleVisualizer(context: Context) : View(context) {

    private companion object {
        const val BARS = 64
        const val FFT_SIZE = 1024
        const val FALL = 0.88f
        const val INNER = 0.42f
        const val MAX_LEN = 0.55f
        const val BAR_FILL = 0.55
        const val MIN_LEN_DP = 3
    }

    private val spectrum = Spectrum(FFT_SIZE, BARS)
    private val samples = FloatArray(FFT_SIZE)
    private val target = FloatArray(BARS)
    private val level = FloatArray(BARS)
    private val density = resources.displayMetrics.density
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(0xFD, 0x60, 0x00)
        strokeCap = Paint.Cap.ROUND
    }

    override fun onDraw(canvas: Canvas) {
        if (!AudioService.tap.read(samples)) target.fill(0f)
        else spectrum.compute(samples, AudioService.tap.sampleRate, target)
        for (i in 0 until BARS) {
            level[i] = if (target[i] > level[i]) target[i] else level[i] * FALL + target[i] * (1 - FALL)
        }

        val cx = width / 2f
        val cy = height / 2f
        val r = min(width, height) / 2f
        val inner = r * INNER
        val maxLen = r * MAX_LEN
        paint.strokeWidth = (2 * PI * inner / BARS * BAR_FILL).toFloat()
        for (i in 0 until BARS) {
            val a = -PI / 2 + 2 * PI * i / BARS
            val c = cos(a).toFloat()
            val s = sin(a).toFloat()
            val len = MIN_LEN_DP * density + level[i] * maxLen
            canvas.drawLine(cx + c * inner, cy + s * inner, cx + c * (inner + len), cy + s * (inner + len), paint)
        }
        postInvalidateOnAnimation()
    }
}
