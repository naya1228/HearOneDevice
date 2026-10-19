package com.naya.hod

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ProtocolTest {
    // docs/PROTOCOL.md: 패킷
    @Test
    fun parsesHeader() {
        val p = Protocol.parse(byteArrayOf(2, 2, 0x34, 0x12, 7, 8))!!
        assertEquals(0x1234, p.seq)
        assertEquals(Codecs.OPUS_128K_STEREO, p.codec)
        assertEquals(listOf<Byte>(7, 8), p.data.drop(Protocol.HEADER_LEN))
    }

    @Test
    fun rejectsOtherVersion() {
        assertNull(Protocol.parse(byteArrayOf(1, 1, 0, 0, 0)))
        assertNull(Protocol.parse(byteArrayOf(2, 1, 0)))
    }

    @Test
    fun unknownCodecHasNoDecoder() {
        assertEquals(99, Protocol.parse(byteArrayOf(2, 99, 0, 0, 0))!!.codec)
        assertNull(Codecs.decoder(99))
    }

    // docs/PROTOCOL.md: 찾기, 시험 값
    @Test
    fun parsesQrLink() {
        val key = "ab".repeat(32)
        assertEquals(
            Protocol.Link("00c0ffee", key, "Dell-Pro-14"),
            Protocol.parseLink("hearone://connect?id=00c0ffee&key=$key&name=Dell-Pro-14"),
        )
        assertEquals(
            Protocol.Link("00c0ffee", key, "내 PC"),
            Protocol.parseLink("hearone://connect?id=00C0FFEE&key=$key&name=%EB%82%B4%20PC"),
        )
        assertEquals(
            Protocol.Link("00c0ffee", key, null),
            Protocol.parseLink("hearone://connect?id=00c0ffee&addr=98fe3e000001&key=$key"),
        )
        assertEquals(Protocol.Link("00c0ffee", null, null), Protocol.parseLink("hearone://connect?id=00c0ffee"))
        assertNull(Protocol.parseLink("https://example.com"))
        assertNull(Protocol.parseLink("hearone://connect?id=123"))
    }

    // docs/FEATURES.md: PC 기록
    @Test
    fun savedPcsRoundTrip() {
        val list = listOf(
            SavedPcs.Pc("00c0ffee", "노트\t북", 2, "ab".repeat(32)),
            SavedPcs.Pc("1a2b3c4d", "", 1),
        )
        val back = SavedPcs.decode(SavedPcs.encode(list))
        assertEquals(listOf(list[0].copy(name = "노트 북"), list[1]), back)
        assertEquals("1a2b3c4d", back[1].label)
        assertEquals(listOf(true, false), back.map { it.canConnect })
    }

    @Test
    fun savedPcsReadsOldLines() {
        assertEquals(listOf(SavedPcs.Pc("00c0ffee", "노트북", 5)), SavedPcs.decode("00c0ffee\t5\t노트북"))
        val key = "ab".repeat(32)
        assertEquals(
            listOf(SavedPcs.Pc("00c0ffee", "노트북", 5, key)),
            SavedPcs.decode("00c0ffee\t5\t98:FE:3E:00:00:01\t$key\t노트북"),
        )
    }
}
