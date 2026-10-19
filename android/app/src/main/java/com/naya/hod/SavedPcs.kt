package com.naya.hod

import android.content.Context

// docs/FEATURES.md: PC 기록
object SavedPcs {
    data class Pc(val id: String, val name: String, val lastUsed: Long, val key: String = "") {
        val label: String get() = name.ifBlank { id }
        val canConnect: Boolean get() = key.isNotEmpty()
    }

    private const val PREFS = "pcs"
    private const val KEY_LIST = "list"
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

    fun remember(ctx: Context, link: Protocol.Link) {
        val list = all(ctx)
        val old = list.find { it.id == link.id }
        val pc = Pc(
            link.id, old?.name?.ifBlank { null } ?: link.name.orEmpty(), System.currentTimeMillis(),
            link.key.orEmpty(),
        )
        save(ctx, listOf(pc) + list.filter { it.id != link.id })
    }

    fun touch(ctx: Context, id: String) {
        val list = all(ctx)
        val pc = list.find { it.id == id } ?: return
        save(ctx, listOf(pc.copy(lastUsed = System.currentTimeMillis())) + list.filter { it.id != id })
    }

    fun rename(ctx: Context, id: String, name: String) =
        save(ctx, all(ctx).map { if (it.id == id) it.copy(name = name.trim()) else it })

    fun remove(ctx: Context, id: String) = save(ctx, all(ctx).filter { it.id != id })

    private fun save(ctx: Context, list: List<Pc>) =
        ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putString(KEY_LIST, encode(list)).apply()

    fun encode(list: List<Pc>): String = list.joinToString("\n") {
        "${it.id}\t${it.lastUsed}\t${it.key}\t${it.name.replace(Regex("[\t\n\r]"), " ")}"
    }

    fun decode(text: String): List<Pc> = text.lines().mapNotNull { line ->
        val parts = line.split("\t", limit = 5)
        val time = parts.getOrNull(1)?.toLongOrNull() ?: 0
        when (parts.size) {
            5 -> Pc(parts[0], parts[4], time, parts[3])
            4 -> Pc(parts[0], parts[3], time, parts[2])
            3 -> Pc(parts[0], parts[2], time)
            else -> null
        }
    }.sortedByDescending { it.lastUsed }
}
