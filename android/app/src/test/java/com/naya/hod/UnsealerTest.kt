package com.naya.hod

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

// docs/PROTOCOL.md: 시험 값
class UnsealerTest {
    private val audioKey = Protocol.hexBytes("90de82fa19fcc35801587b8582a6fb07730eefba845edac03b027ff24a417245")
    private val sealed = Protocol.hexBytes("02017011" + "38a121539dec3e5259d7bb98e1229d66d650a06dc5afd1cbd8d4")

    private fun packet(bytes: ByteArray) = Protocol.parse(bytes)!!

    @Test
    fun opensNextPacket() {
        val p = Unsealer(audioKey, 69999).open(packet(sealed))!!
        assertEquals(1, p.codec)
        assertArrayEquals(ByteArray(10) { 5 }, p.data.copyOfRange(Protocol.HEADER_LEN, p.data.size))
    }

    @Test
    fun countsWrapOfHeaderSeq() {
        assertEquals(1, Unsealer(audioKey, 65530).open(packet(sealed))?.codec)
        assertNull(Unsealer(audioKey, 4000).open(packet(sealed)))
    }

    @Test
    fun rejectsReplayAndTamper() {
        val u = Unsealer(audioKey, 69999)
        u.open(packet(sealed))!!
        assertNull(u.open(packet(sealed)))
        val tampered = sealed.copyOf().also { it[4] = (it[4] + 1).toByte() }
        assertNull(Unsealer(audioKey, 69999).open(packet(tampered)))
        val otherHeader = sealed.copyOf().also { it[1] = 2 }
        assertNull(Unsealer(audioKey, 69999).open(packet(otherHeader)))
    }
}
