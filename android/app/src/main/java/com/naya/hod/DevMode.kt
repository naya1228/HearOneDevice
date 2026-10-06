package com.naya.hod

import android.content.Context

/**
 * 숨은 개발 모드. 켜면 화면에 재생 상세 숫자(AudioService.stats)가 보인다.
 * 제목을 빠르게 7번 누르면 켜지고/꺼지고, 앱을 다시 켜도 기억한다.
 */
object DevMode {
    private const val PREFS = "dev"
    private const val KEY_ON = "on"
    private const val TAPS = 7
    private const val TAP_GAP_MS = 1000L // 이보다 늦게 누르면 처음부터 다시 셈

    private var taps = 0
    private var lastTapAt = 0L

    fun isOn(ctx: Context): Boolean =
        ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean(KEY_ON, false)

    /** 제목을 누를 때마다 호출. 7번째에 모드를 바꾸고 새 상태를 돌려준다. 아직이면 null */
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
