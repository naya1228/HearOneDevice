package com.naya.hod

import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

/**
 * 확인 통과 뒤 PC가 잠가 보낸 패킷을 푼다 (AES-256-GCM). 형식은 docs/PROTOCOL.md 6절, PC 쪽은 src-tauri/src/auth/seal.rs
 * audioKey = 이번 연결의 소리 열쇠 (Auth.audioKey), lastSeq = 통과 패킷의 긴 순번 (이 뒤로만 받음)
 */
class Unsealer(audioKey: ByteArray, private var lastSeq: Long) {

    companion object {
        const val TAG_LEN = 16
    }

    private val key = SecretKeySpec(audioKey, "AES")
    private val cipher = Cipher.getInstance("AES/GCM/NoPadding")

    /** 풀린 패킷 (헤더 + 원래 내용). 검사값이 틀리거나 지난 패킷(다시 틀기)이면 null */
    fun open(packet: Protocol.Packet): Protocol.Packet? {
        val raw = packet.data
        if (raw.size < Protocol.HEADER_LEN + TAG_LEN) return null
        val seq = longSeq(packet.seq) ?: return null
        val plain = try {
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(TAG_LEN * 8, nonce(seq)))
            cipher.updateAAD(raw, 0, Protocol.HEADER_LEN)
            cipher.doFinal(raw, Protocol.HEADER_LEN, raw.size - Protocol.HEADER_LEN)
        } catch (e: AEADBadTagException) {
            return null
        }
        lastSeq = seq
        return Protocol.Packet(packet.seq, packet.codec, raw.copyOfRange(0, Protocol.HEADER_LEN) + plain)
    }

    // 헤더의 16비트 순번 → 긴 순번. 마지막 것보다 앞으로 1~32767이면 그만큼 앞, 아니면 지난 패킷
    private fun longSeq(low: Int): Long? {
        val diff = (low - (lastSeq and 0xFFFF).toInt()) and 0xFFFF
        return if (diff in 1 until 0x8000) lastSeq + diff else null
    }

    // 00 00 00 00 + 긴 순번 u64 little-endian
    private fun nonce(seq: Long) = ByteArray(12).also { n ->
        for (i in 0 until 8) n[4 + i] = (seq ushr (8 * i)).toByte()
    }
}
