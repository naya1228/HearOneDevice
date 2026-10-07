package com.naya.hod

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

// PC의 auth/handshake.rs 테스트와 같은 값 (열쇠 7×32, P = 0..15, F = 9×16)
class AuthTest {
    private val key = ByteArray(32) { 7 }
    private val pcNonce = ByteArray(16) { it.toByte() }
    private val phoneNonce = ByteArray(16) { 9 }
    private val phoneProof = Protocol.hexBytes("9beffad1415a5c8a6753ab385c6705c48e7e55360551650d2aee0ced3c03ae31")
    private val pcProof = Protocol.hexBytes("2bf2e7dbe786aad5e2980fe4816e0ba6b991034e44dbbf698966c29b5c7ac527")

    @Test
    fun replyHasVersionTypeNonceProof() {
        val reply = Auth(key, phoneNonce).reply(pcNonce)!!
        assertArrayEquals(byteArrayOf(1, 1) + phoneNonce + phoneProof, reply)
    }

    @Test
    fun checksPcProof() {
        val auth = Auth(key, phoneNonce)
        assertFalse(auth.checkPc(pcProof)) // 문제를 받기 전
        auth.reply(pcNonce)
        assertTrue(auth.checkPc(pcProof))
        assertFalse(auth.checkPc(pcProof.copyOf().also { it[0] = 0 }))
        assertFalse(auth.checkPc(pcProof.copyOf(31)))
    }

    @Test
    fun audioKeyAfterChallenge() {
        val auth = Auth(key, phoneNonce)
        assertNull(auth.audioKey())
        auth.reply(pcNonce)
        assertArrayEquals(Protocol.hexBytes("90de82fa19fcc35801587b8582a6fb07730eefba845edac03b027ff24a417245"), auth.audioKey())
    }

    @Test
    fun wrongChallengeLength() {
        assertNull(Auth(key, phoneNonce).reply(ByteArray(15)))
    }
}
