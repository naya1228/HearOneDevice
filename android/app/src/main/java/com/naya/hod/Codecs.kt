package com.naya.hod

// 코덱 번호 목록. PC(src-tauri/src/codec/mod.rs 의 Codec)와 번호가 같아야 함. 표는 docs/CODECS.md
object Codecs {
    const val ULAW_16K_MONO = 1

    /** 이 앱이 받을 코덱. PC lib.rs 의 CODEC 과 같은 번호로 맞출 것 */
    const val ACTIVE = ULAW_16K_MONO

    interface Decoder {
        val id: Int
        val name: String
        val sampleRate: Int
        /** 코덱 데이터(헤더 뗀 것) → 16bit 모노 PCM */
        fun decode(data: ByteArray, offset: Int): ShortArray
    }

    fun decoder(id: Int): Decoder? = when (id) {
        ULAW_16K_MONO -> Ulaw16kMono
        else -> null
    }

    val active: Decoder = decoder(ACTIVE) ?: error("Codecs.ACTIVE=$ACTIVE 에 해당하는 디코더 없음")

    // 코덱 1: G.711 μ-law → 16bit PCM (바이트 1개 = 샘플 1개)
    object Ulaw16kMono : Decoder {
        override val id = ULAW_16K_MONO
        override val name = "μ-law 16kHz 모노"
        override val sampleRate = 16000

        private val TABLE = ShortArray(256) { i ->
            val u = i.inv() and 0xFF
            val exp = (u shr 4) and 0x07
            val s = ((((u and 0x0F) shl 3) + 0x84) shl exp) - 0x84
            (if (u and 0x80 != 0) -s else s).toShort()
        }

        override fun decode(data: ByteArray, offset: Int) =
            ShortArray(data.size - offset) { TABLE[data[it + offset].toInt() and 0xFF] }
    }
}
