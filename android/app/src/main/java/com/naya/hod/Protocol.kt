package com.naya.hod

import java.util.UUID

// PC(src-tauri/src/codec.rs, ble.rs)와 맞춘 약속. 한쪽을 바꾸면 반대쪽도 바꿀 것.
//   [0] 버전 = 1, [1] 코덱 = 1 (μ-law 16kHz 모노), [2..4] 순번 u16 LE, [4..] μ-law 바이트
object Protocol {
    val SERVICE_UUID: UUID = UUID.fromString("5e7a0001-3c1b-4f6e-9d2a-7b1c0e5a9f10")
    val AUDIO_CHAR_UUID: UUID = UUID.fromString("5e7a0002-3c1b-4f6e-9d2a-7b1c0e5a9f10")
    // 알림 구독을 켜는 표준 디스크립터
    val CCCD_UUID: UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")

    // PC 광고의 제조사 데이터 = PC 고유 번호 4바이트 (ble.rs 의 MANUFACTURER_ID)
    const val MANUFACTURER_ID = 0xFFFF

    /** QR 링크 hearone://connect?id=1a2b3c4d 에서 번호(16진수 8자리)를 꺼냄 */
    fun parseLink(link: String?): String? {
        val m = Regex("""^hearone://connect\?id=([0-9a-fA-F]{8})$""").find(link?.trim() ?: return null)
        return m?.groupValues?.get(1)?.lowercase()
    }

    fun idBytes(hex: String): ByteArray = ByteArray(4) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }

    const val VERSION = 1
    const val CODEC_ULAW_16K_MONO = 1
    const val HEADER_LEN = 4
    const val SAMPLE_RATE = 16000

    class Packet(val seq: Int, val pcm: ShortArray)

    // G.711 μ-law → 16bit PCM
    private val ULAW = ShortArray(256) { i ->
        val u = i.inv() and 0xFF
        val exp = (u shr 4) and 0x07
        val s = ((((u and 0x0F) shl 3) + 0x84) shl exp) - 0x84
        (if (u and 0x80 != 0) -s else s).toShort()
    }

    /** 형식이 다르면 null (버전/코덱이 안 맞는 PC) */
    fun parse(data: ByteArray): Packet? {
        if (data.size < HEADER_LEN) return null
        if (data[0].toInt() != VERSION || data[1].toInt() != CODEC_ULAW_16K_MONO) return null
        val seq = (data[2].toInt() and 0xFF) or ((data[3].toInt() and 0xFF) shl 8)
        val pcm = ShortArray(data.size - HEADER_LEN) { ULAW[data[it + HEADER_LEN].toInt() and 0xFF] }
        return Packet(seq, pcm)
    }
}
