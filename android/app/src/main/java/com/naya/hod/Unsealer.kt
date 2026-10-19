package com.naya.hod

import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

// docs/PROTOCOL.md: 잠그기
class Unsealer(audioKey: ByteArray, private var lastSeq: Long) {

    companion object {
        const val TAG_LEN = 16
    }

    private val key = SecretKeySpec(audioKey, "AES")
    private val cipher = Cipher.getInstance("AES/GCM/NoPadding")

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

    private fun longSeq(low: Int): Long? {
        val diff = (low - (lastSeq and 0xFFFF).toInt()) and 0xFFFF
        return if (diff in 1 until 0x8000) lastSeq + diff else null
    }

    private fun nonce(seq: Long) = ByteArray(12).also { n ->
        for (i in 0 until 8) n[4 + i] = (seq ushr (8 * i)).toByte()
    }
}
