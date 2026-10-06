package com.naya.hod

import android.content.Context

/**
 * 연결했던 PC 기록. PC 번호가 열쇠이고 이름은 표시용 (QR로 받거나 사용자가 바꿈).
 * 최근에 연결한 PC가 앞에 온다.
 */
object SavedPcs {
    data class Pc(val id: String, val name: String, val lastUsed: Long) {
        /** 화면에 보일 이름. 이름이 없으면 번호 */
        val label: String get() = name.ifBlank { id }
    }

    private const val PREFS = "pcs"
    private const val KEY_LIST = "list"
    // 예전 버전이 "마지막 PC" 하나만 저장하던 곳 → 처음 읽을 때 기록으로 옮김
    private const val OLD_PREFS = "hod"
    private const val OLD_KEY = "last_pc"

    fun all(ctx: Context): List<Pc> {
        val prefs = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val text = prefs.getString(KEY_LIST, null)
        if (text != null) return decode(text)
        val old = ctx.getSharedPreferences(OLD_PREFS, Context.MODE_PRIVATE).getString(OLD_KEY, null)
        return listOfNotNull(old?.let { Pc(it, "", 0) }).also { save(ctx, it) }
    }

    fun last(ctx: Context): Pc? = all(ctx).firstOrNull()

    fun find(ctx: Context, id: String): Pc? = all(ctx).find { it.id == id }

    /** 연결할 때 호출. 없으면 추가, 있으면 맨 앞으로. 이름은 비어 있을 때만 채움 (사용자가 바꾼 이름 유지) */
    fun remember(ctx: Context, id: String, name: String?) {
        val list = all(ctx)
        val old = list.find { it.id == id }
        val pc = Pc(id, old?.name?.ifBlank { null } ?: name.orEmpty(), System.currentTimeMillis())
        save(ctx, listOf(pc) + list.filter { it.id != id })
    }

    fun rename(ctx: Context, id: String, name: String) =
        save(ctx, all(ctx).map { if (it.id == id) it.copy(name = name.trim()) else it })

    fun remove(ctx: Context, id: String) = save(ctx, all(ctx).filter { it.id != id })

    private fun save(ctx: Context, list: List<Pc>) =
        ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putString(KEY_LIST, encode(list)).apply()

    // 한 줄에 PC 하나: 번호 \t 마지막 연결 시각 \t 이름 (이름의 탭·줄바꿈은 공백으로)
    fun encode(list: List<Pc>): String =
        list.joinToString("\n") { "${it.id}\t${it.lastUsed}\t${it.name.replace(Regex("[\t\n\r]"), " ")}" }

    fun decode(text: String): List<Pc> = text.lines().mapNotNull { line ->
        val parts = line.split("\t", limit = 3)
        if (parts.size < 3) return@mapNotNull null
        Pc(parts[0], parts[2], parts[1].toLongOrNull() ?: 0)
    }.sortedByDescending { it.lastUsed }
}
