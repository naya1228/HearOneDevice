package com.naya.hod

import java.security.MessageDigest
import java.security.SecureRandom
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec

// docs/PROTOCOL.md: 연결 확인, 잠그기
class Auth(private val key: ByteArray, private val phoneNonce: ByteArray = randomNonce()) {

    companion object {
        const val NONCE_LEN = 16
        const val PROOF_LEN = 32
        private const val REPLY_VERSION: Byte = 1
        private const val REPLY_TYPE: Byte = 1
        private val PHONE_LABEL = "hearone-phone".toByteArray(Charsets.US_ASCII)
        private val PC_LABEL = "hearone-pc".toByteArray(Charsets.US_ASCII)
        private val AUDIO_LABEL = "hearone-audio".toByteArray(Charsets.US_ASCII)

        private fun randomNonce() = ByteArray(NONCE_LEN).also { SecureRandom().nextBytes(it) }
    }

    private var pcNonce: ByteArray? = null

    fun reply(challenge: ByteArray): ByteArray? {
        if (challenge.size != NONCE_LEN) return null
        pcNonce = challenge
        return byteArrayOf(REPLY_VERSION, REPLY_TYPE) + phoneNonce + proof(PHONE_LABEL, challenge)
    }

    fun checkPc(pcProof: ByteArray): Boolean {
        val p = pcNonce ?: return false
        return pcProof.size == PROOF_LEN && MessageDigest.isEqual(proof(PC_LABEL, p), pcProof)
    }

    fun audioKey(): ByteArray? = pcNonce?.let { proof(AUDIO_LABEL, it) }

    private fun proof(label: ByteArray, pc: ByteArray): ByteArray =
        Mac.getInstance("HmacSHA256").run {
            init(SecretKeySpec(key, "HmacSHA256"))
            update(label)
            update(pc)
            update(phoneNonce)
            doFinal()
        }
}
