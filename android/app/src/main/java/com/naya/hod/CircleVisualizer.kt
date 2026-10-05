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

/**
 * 원 둘레에 막대 BARS개가 바깥으로 뻗는 스펙트럼. 위쪽부터 시계 방향으로 저음 → 고음.
 * 소리는 AudioService.tap 에서 읽는다. 화면에 보일 때만 그려지므로 화면이 꺼지면 계산도 멈춘다.
 */
class CircleVisualizer(context: Context) : View(context) {

    private companion object {
        const val BARS = 64
        const val FFT_SIZE = 1024
    }

    private val spectrum = Spectrum(FFT_SIZE, BARS)
    private val samples = FloatArray(FFT_SIZE)
    private val target = FloatArray(BARS)
    private val level = FloatArray(BARS) // 화면에 그리는 값: 올라갈 땐 바로, 내려갈 땐 천천히
    private val density = resources.displayMetrics.density
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.rgb(0xFD, 0x60, 0x00)
        strokeCap = Paint.Cap.ROUND
    }

    override fun onDraw(canvas: Canvas) {
        if (!AudioService.tap.read(samples)) target.fill(0f)
        else spectrum.compute(samples, AudioService.tap.sampleRate, target)
        for (i in 0 until BARS) {
            level[i] = if (target[i] > level[i]) target[i] else level[i] * 0.88f + target[i] * 0.12f
        }

        val cx = width / 2f
        val cy = height / 2f
        val r = min(width, height) / 2f
        val inner = r * 0.42f
        val maxLen = r * 0.55f
        paint.strokeWidth = (2 * PI * inner / BARS * 0.55).toFloat()
        for (i in 0 until BARS) {
            val a = -PI / 2 + 2 * PI * i / BARS
            val c = cos(a).toFloat()
            val s = sin(a).toFloat()
            val len = 3 * density + level[i] * maxLen
            canvas.drawLine(cx + c * inner, cy + s * inner, cx + c * (inner + len), cy + s * (inner + len), paint)
        }
        postInvalidateOnAnimation()
    }
}
