package com.naya.hod

import java.net.URLDecoder
import java.util.UUID

// docs/PROTOCOL.md
object Protocol {
    val SERVICE_UUID: UUID = UUID.fromString("5e7a0001-3c1b-4f6e-9d2a-7b1c0e5a9f10")
    val AUDIO_CHAR_UUID: UUID = UUID.fromString("5e7a0002-3c1b-4f6e-9d2a-7b1c0e5a9f10")
    val CCCD_UUID: UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")

    data class Link(val id: String, val key: String?, val name: String?)

    fun parseLink(link: String?): Link? {
        val m = Regex("""^hearone://connect\?(.*)$""").find(link?.trim() ?: return null) ?: return null
        val q = m.groupValues[1].split("&").mapNotNull { kv ->
            kv.split("=", limit = 2).takeIf { it.size == 2 }?.let { it[0] to it[1] }
        }.toMap()
        val id = q["id"]?.takeIf { it.matches(Regex("[0-9a-fA-F]{8}")) }?.lowercase() ?: return null
        val key = q["key"]?.takeIf { it.matches(Regex("[0-9a-fA-F]{64}")) }?.lowercase()
        val name = q["name"]?.takeIf { it.isNotEmpty() }?.let { runCatching { URLDecoder.decode(it, "UTF-8") }.getOrNull() }
        return Link(id, key, name)
    }

    fun hexBytes(hex: String): ByteArray = ByteArray(hex.length / 2) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }

    const val MTU_REQUEST = 517

    const val VERSION = 2
    const val HEADER_LEN = 4

    const val CONTROL = 0
    const val CONTROL_STOP = 1
    const val CONTROL_CHALLENGE = 2
    const val CONTROL_AUTH_OK = 3
    const val CONTROL_AUTH_FAIL = 4

    class Packet(val seq: Int, val codec: Int, val data: ByteArray)

    fun parse(data: ByteArray): Packet? {
        if (data.size < HEADER_LEN || data[0].toInt() != VERSION) return null
        val seq = (data[2].toInt() and 0xFF) or ((data[3].toInt() and 0xFF) shl 8)
        return Packet(seq, data[1].toInt() and 0xFF, data)
    }
}
