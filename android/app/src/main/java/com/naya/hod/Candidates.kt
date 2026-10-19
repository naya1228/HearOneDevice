package com.naya.hod

// docs/PROTOCOL.md: 찾기
class Candidates {
    private val rejected = mutableSetOf<String>()
    private val seen = mutableMapOf<String, Int>()
    var lastGood: String? = null
        private set
    var rejections = 0
        private set

    fun newRound() = seen.clear()

    fun offer(addr: String, rssi: Int) {
        if (addr in rejected) return
        seen[addr] = maxOf(rssi, seen[addr] ?: Int.MIN_VALUE)
    }

    fun pick(): String? = lastGood?.takeIf { it in seen } ?: seen.maxByOrNull { it.value }?.key

    fun accept(addr: String) {
        lastGood = addr
        rejections = 0
    }

    fun reject(addr: String) {
        rejected += addr
        seen.remove(addr)
        rejections++
        if (addr == lastGood) lastGood = null
    }

    fun unreachable(addr: String) {
        seen.remove(addr)
        if (addr == lastGood) lastGood = null
    }

    fun reset() {
        rejected.clear()
        seen.clear()
        lastGood = null
        rejections = 0
    }
}
