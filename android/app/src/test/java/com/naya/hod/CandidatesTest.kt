package com.naya.hod

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CandidatesTest {
    @Test
    fun picksStrongestThenSkipsRejected() {
        val c = Candidates()
        c.offer("A", -80)
        c.offer("B", -50)
        c.offer("A", -70)
        assertEquals("B", c.pick())
        c.reject("B")
        assertEquals("A", c.pick())
        c.offer("B", -40) // 떨어진 주소는 다시 와도 무시
        assertEquals("A", c.pick())
        assertEquals(1, c.rejections)
    }

    @Test
    fun lastGoodFirstWhenSeen() {
        val c = Candidates()
        c.offer("A", -90)
        c.accept("A")
        c.newRound()
        c.offer("B", -40)
        assertEquals("B", c.pick()) // 이번에 안 보이면 다른 후보
        c.offer("A", -90)
        assertEquals("A", c.pick())
        assertEquals(0, c.rejections)
    }

    @Test
    fun unreachableForgetsLastGood() {
        val c = Candidates()
        c.offer("A", -50)
        c.accept("A")
        c.unreachable("A")
        assertNull(c.lastGood)
        assertNull(c.pick())
    }

    @Test
    fun resetClearsEverything() {
        val c = Candidates()
        c.offer("A", -50)
        c.reject("A")
        c.reset()
        c.offer("A", -50)
        assertEquals("A", c.pick())
        assertEquals(0, c.rejections)
    }
}
