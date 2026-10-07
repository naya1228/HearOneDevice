package com.naya.hod

import java.net.URLDecoder
import java.util.UUID

// PC와 맞춘 약속 (UUID·광고·패킷·제어 메시지). 값과 규칙은 docs/PROTOCOL.md
object Protocol {
    val SERVICE_UUID: UUID = UUID.fromString("5e7a0001-3c1b-4f6e-9d2a-7b1c0e5a9f10")
    val AUDIO_CHAR_UUID: UUID = UUID.fromString("5e7a0002-3c1b-4f6e-9d2a-7b1c0e5a9f10")
    // 알림 구독을 켜는 표준 디스크립터
    val CCCD_UUID: UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")

    /** QR 링크에서 꺼낸 PC 번호(16진수 8자리)와 이름 (옛 QR엔 이름이 없음) */
    data class Link(val id: String, val name: String?)

    /** hearone://connect?id=1a2b3c4d&name=... */
    fun parseLink(link: String?): Link? {
        val m = Regex("""^hearone://connect\?id=([0-9a-fA-F]{8})(?:&name=([^&]*))?$""").find(link?.trim() ?: return null)
            ?: return null
        val name = m.groupValues[2].takeIf { it.isNotEmpty() }?.let { runCatching { URLDecoder.decode(it, "UTF-8") }.getOrNull() }
        return Link(m.groupValues[1].lowercase(), name)
    }

    fun idBytes(hex: String): ByteArray = ByteArray(4) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }

    const val VERSION = 1
    const val HEADER_LEN = 4

    const val CONTROL = 0
    const val CONTROL_STOP = 1

    /** codec = 코덱 번호 (Codecs.kt), 코덱 데이터는 data\[HEADER_LEN..] */
    class Packet(val seq: Int, val codec: Int, val data: ByteArray)

    /** 헤더가 깨졌거나 버전이 다르면 null. 코덱 번호 확인은 받는 쪽에서 */
    fun parse(data: ByteArray): Packet? {
        if (data.size < HEADER_LEN || data[0].toInt() != VERSION) return null
        val seq = (data[2].toInt() and 0xFF) or ((data[3].toInt() and 0xFF) shl 8)
        return Packet(seq, data[1].toInt() and 0xFF, data)
    }
}
