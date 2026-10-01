package com.naya.hod

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

// PC의 codec.rs 테스트와 같은 값으로 양쪽 형식이 맞는지 확인
class ProtocolTest {
    @Test
    fun parsesHeaderAndDecodesUlaw() {
        val p = Protocol.parse(byteArrayOf(1, 1, 0x34, 0x12, 0xFF.toByte(), 0x80.toByte(), 0x00))!!
        assertEquals(0x1234, p.seq)
        assertEquals(0, p.pcm[0].toInt())        // 0xFF = 무음
        assertEquals(32124, p.pcm[1].toInt())    // 0x80 = 최대 +
        assertEquals(-32124, p.pcm[2].toInt())   // 0x00 = 최대 -
    }

    @Test
    fun rejectsOtherVersion() {
        assertNull(Protocol.parse(byteArrayOf(2, 1, 0, 0, 0)))
        assertNull(Protocol.parse(byteArrayOf(1, 1, 0)))
    }
}
