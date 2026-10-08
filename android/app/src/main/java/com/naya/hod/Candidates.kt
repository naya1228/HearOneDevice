package com.naya.hod

/**
 * PC 후보 고르기. 광고로는 내 PC를 가릴 수 없어서(docs/PROTOCOL.md 1절) HearOne PC에 차례로 붙어 열쇠로 확인한다.
 * 이번 찾기에서 본 주소 중 마지막으로 통과한 주소 → 신호가 센 주소 순으로 고르고, 확인에 떨어진 주소는 건너뛴다.
 * 주소는 PC가 바꿀 수 있어서(Windows) 기억은 이번 듣기 동안만.
 */
class Candidates {
    private val rejected = mutableSetOf<String>()
    private val seen = mutableMapOf<String, Int>() // 이번 찾기에서 본 주소 → 가장 센 신호
    /** 마지막으로 확인을 통과한 주소 */
    var lastGood: String? = null
        private set
    /** 통과 없이 연속으로 떨어진 횟수 */
    var rejections = 0
        private set

    /** 새 찾기를 시작할 때 (이번에 본 주소만 비움) */
    fun newRound() = seen.clear()

    /** 광고를 봤을 때. 떨어진 주소는 무시 */
    fun offer(addr: String, rssi: Int) {
        if (addr in rejected) return
        seen[addr] = maxOf(rssi, seen[addr] ?: Int.MIN_VALUE)
    }

    /** 다음에 붙어 볼 주소. 아직 없으면 null */
    fun pick(): String? = lastGood?.takeIf { it in seen } ?: seen.maxByOrNull { it.value }?.key

    /** 확인 통과 */
    fun accept(addr: String) {
        lastGood = addr
        rejections = 0
    }

    /** 확인에 떨어짐 (남의 PC이거나 열쇠가 다름) */
    fun reject(addr: String) {
        rejected += addr
        seen.remove(addr)
        rejections++
        if (addr == lastGood) lastGood = null
    }

    /** 확인 전에 연결이 안 됨. 그 주소는 이미 바뀌었을 수 있음 */
    fun unreachable(addr: String) {
        seen.remove(addr)
        if (addr == lastGood) lastGood = null
    }

    /** 다른 PC로 바꾸거나 새로 들을 때 */
    fun reset() {
        rejected.clear()
        seen.clear()
        lastGood = null
        rejections = 0
    }
}
