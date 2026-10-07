package com.naya.hod

import java.security.MessageDigest
import java.security.SecureRandom
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec

/**
 * 연결 한 번의 확인 절차 (폰 쪽 계산). PC가 낸 문제에 답하고, PC가 돌려준 증명을 검사한다.
 * 메시지 순서·형식은 docs/PROTOCOL.md 5절. PC 쪽은 src-tauri/src/auth/handshake.rs
 */
class Auth(private val key: ByteArray, private val phoneNonce: ByteArray = randomNonce()) {

    companion object {
        const val NONCE_LEN = 16
        const val PROOF_LEN = 32
        private const val REPLY_VERSION: Byte = 1
        private const val REPLY_TYPE: Byte = 1
        private val PHONE_LABEL = "hearone-phone".toByteArray(Charsets.US_ASCII)
        private val PC_LABEL = "hearone-pc".toByteArray(Charsets.US_ASCII)

        private fun randomNonce() = ByteArray(NONCE_LEN).also { SecureRandom().nextBytes(it) }
    }

    private var pcNonce: ByteArray? = null

    /** PC 문제(무작위 값 P)를 받아 오디오 특성에 쓸 답. P 길이가 틀리면 null */
    fun reply(challenge: ByteArray): ByteArray? {
        if (challenge.size != NONCE_LEN) return null
        pcNonce = challenge
        return byteArrayOf(REPLY_VERSION, REPLY_TYPE) + phoneNonce + proof(PHONE_LABEL, challenge)
    }

    /** PC 증명이 맞는지. 문제를 받기 전이면 false */
    fun checkPc(pcProof: ByteArray): Boolean {
        val p = pcNonce ?: return false
        // 비교에 걸리는 시간으로 답이 새지 않게 MessageDigest.isEqual로 비교
        return pcProof.size == PROOF_LEN && MessageDigest.isEqual(proof(PC_LABEL, p), pcProof)
    }

    /** 증명 = HMAC-SHA256(열쇠, 이름표 + PC 무작위 값 + 폰 무작위 값) */
    private fun proof(label: ByteArray, pc: ByteArray): ByteArray =
        Mac.getInstance("HmacSHA256").run {
            init(SecretKeySpec(key, "HmacSHA256"))
            update(label)
            update(pc)
            update(phoneNonce)
            doFinal()
        }
}
