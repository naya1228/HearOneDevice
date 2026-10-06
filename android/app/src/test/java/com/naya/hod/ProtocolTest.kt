package com.naya.hod

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

// PC의 codec.rs 테스트와 같은 값으로 양쪽 형식이 맞는지 확인
class ProtocolTest {
    @Test
    fun parsesHeader() {
        val p = Protocol.parse(byteArrayOf(1, 2, 0x34, 0x12, 7, 8))!!
        assertEquals(0x1234, p.seq)
        assertEquals(Codecs.OPUS_128K_STEREO, p.codec)
        assertEquals(listOf<Byte>(7, 8), p.data.drop(Protocol.HEADER_LEN))
    }

    @Test
    fun rejectsOtherVersion() {
        assertNull(Protocol.parse(byteArrayOf(2, 1, 0, 0, 0)))
        assertNull(Protocol.parse(byteArrayOf(1, 1, 0)))
    }

    @Test
    fun unknownCodecHasNoDecoder() {
        assertEquals(99, Protocol.parse(byteArrayOf(1, 99, 0, 0, 0))!!.codec)
        assertNull(Codecs.decoder(99))
    }

    @Test
    fun parsesQrLink() {
        assertEquals(Protocol.Link("00c0ffee", null), Protocol.parseLink("hearone://connect?id=00C0FFEE"))
        // PC의 link.rs 테스트와 같은 값
        assertEquals(Protocol.Link("00c0ffee", "내 PC"), Protocol.parseLink("hearone://connect?id=00c0ffee&name=%EB%82%B4%20PC"))
        assertNull(Protocol.parseLink("https://example.com"))
        assertNull(Protocol.parseLink("hearone://connect?id=123"))
        val b = Protocol.idBytes("00c0ffee")
        assertEquals(listOf(0x00, 0xc0, 0xff, 0xee), b.map { it.toInt() and 0xFF })
    }

    @Test
    fun savedPcsRoundTrip() {
        val list = listOf(SavedPcs.Pc("00c0ffee", "노트\t북", 2), SavedPcs.Pc("1a2b3c4d", "", 1))
        val back = SavedPcs.decode(SavedPcs.encode(list))
        assertEquals(listOf(SavedPcs.Pc("00c0ffee", "노트 북", 2), SavedPcs.Pc("1a2b3c4d", "", 1)), back)
        assertEquals("1a2b3c4d", back[1].label)
    }
}
