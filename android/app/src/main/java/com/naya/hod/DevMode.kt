package com.naya.hod

import android.content.Context

// docs/FEATURES.md: 폰 화면
object DevMode {
    private const val PREFS = "dev"
    private const val KEY_ON = "on"
    private const val TAPS = 7
    private const val TAP_GAP_MS = 1000L

    private var taps = 0
    private var lastTapAt = 0L

    fun isOn(ctx: Context): Boolean =
        ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean(KEY_ON, false)

    fun onTap(ctx: Context): Boolean? {
        val now = System.currentTimeMillis()
        taps = if (now - lastTapAt <= TAP_GAP_MS) taps + 1 else 1
        lastTapAt = now
        if (taps < TAPS) return null
        taps = 0
        val on = !isOn(ctx)
        ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putBoolean(KEY_ON, on).apply()
        return on
    }
}
