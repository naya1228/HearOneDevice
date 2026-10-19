package com.naya.hod

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

// docs/FEATURES.md: 업데이트
class UpdaterTest {
    @Test
    fun comparesEachNumber() {
        assertTrue(Updater.newer("0.5.0", "0.4.0"))
        assertTrue(Updater.newer("0.10.0", "0.9.1"))
        assertTrue(Updater.newer("1.0", "0.99.99"))
    }

    @Test
    fun sameOrOlderIsNotNewer() {
        assertFalse(Updater.newer("0.4.0", "0.4.0"))
        assertFalse(Updater.newer("0.4", "0.4.0"))
        assertFalse(Updater.newer("0.3.9", "0.4.0"))
    }
}
