package com.naya.hod

import android.media.MediaCodec
import android.media.MediaFormat
import android.util.Log
import java.nio.ByteBuffer
import java.nio.ByteOrder

// 코덱 번호 목록. PC(src-tauri/src/codec/mod.rs 의 Codec)와 번호가 같아야 함. 표는 docs/CODECS.md
object Codecs {
    const val OPUS_128K_STEREO = 1
    const val OPUS_64K_STEREO = 2
    const val ADPCM_32K_STEREO = 3
    const val ADPCM_48K_STEREO = 4
    const val ULAW_16K_MONO = 5

    interface Decoder {
        val id: Int
        val name: String
        val sampleRate: Int
        /** 1 = 모노, 2 = 스테레오 */
        val channels: Int
        /** 코덱 데이터(헤더 뗀 것) → 16bit PCM (스테레오면 L R L R ...) */
        fun decode(data: ByteArray, offset: Int): ShortArray
        /** 다 쓰면 호출 (디코더가 시스템 자원을 잡고 있는 경우) */
        fun close() {}
    }

    /** 패킷 헤더의 코덱 번호 → 디코더. 이 앱이 모르는 번호면 null (앱 업데이트 필요) */
    fun decoder(id: Int): Decoder? = when (id) {
        ULAW_16K_MONO -> Ulaw16kMono
        ADPCM_32K_STEREO -> AdpcmStereo(ADPCM_32K_STEREO, "ADPCM 32kHz 스테레오", 32000)
        ADPCM_48K_STEREO -> AdpcmStereo(ADPCM_48K_STEREO, "ADPCM 48kHz 스테레오", 48000)
        OPUS_128K_STEREO -> OpusStereo(OPUS_128K_STEREO, "Opus 48kHz 스테레오 128kbps")
        OPUS_64K_STEREO -> OpusStereo(OPUS_64K_STEREO, "Opus 48kHz 스테레오 64kbps")
        else -> null
    }

    // 코덱 5: G.711 μ-law → 16bit PCM (바이트 1개 = 샘플 1개)
    object Ulaw16kMono : Decoder {
        override val id = ULAW_16K_MONO
        override val name = "μ-law 16kHz 모노"
        override val sampleRate = 16000
        override val channels = 1

        private val TABLE = ShortArray(256) { i ->
            val u = i.inv() and 0xFF
            val exp = (u shr 4) and 0x07
            val s = ((((u and 0x0F) shl 3) + 0x84) shl exp) - 0x84
            (if (u and 0x80 != 0) -s else s).toShort()
        }

        override fun decode(data: ByteArray, offset: Int) =
            ShortArray(data.size - offset) { TABLE[data[it + offset].toInt() and 0xFF] }
    }

    // 코덱 3·4: IMA ADPCM 스테레오 (PC src-tauri/src/codec/adpcm.rs 와 같은 형식)
    // 프레임 = [L 예측값 i16 LE, L 인덱스, 0, R 예측값 i16 LE, R 인덱스, 0] + 샘플마다 1바이트(아래 4bit L, 위 4bit R)
    // 프레임마다 시작 상태가 있어서 앞 패킷을 잃어도 이 프레임만으로 풀린다
    class AdpcmStereo(override val id: Int, override val name: String, override val sampleRate: Int) : Decoder {
        override val channels = 2

        override fun decode(data: ByteArray, offset: Int): ShortArray {
            if (data.size < offset + STATE_LEN) return ShortArray(0)
            val pred = IntArray(2) { c -> ((data[offset + c * 4].toInt() and 0xFF) or (data[offset + c * 4 + 1].toInt() shl 8)) }
            val index = IntArray(2) { c -> (data[offset + c * 4 + 2].toInt() and 0xFF).coerceIn(0, 88) }
            val start = offset + STATE_LEN
            val out = ShortArray((data.size - start) * 2)
            for (i in start until data.size) {
                val b = data[i].toInt() and 0xFF
                for (c in 0..1) {
                    val code = if (c == 0) b and 0x0F else b shr 4
                    val step = STEP[index[c]]
                    var delta = step shr 3
                    if (code and 4 != 0) delta += step
                    if (code and 2 != 0) delta += step shr 1
                    if (code and 1 != 0) delta += step shr 2
                    pred[c] = (if (code and 8 != 0) pred[c] - delta else pred[c] + delta).coerceIn(-32768, 32767)
                    index[c] = (index[c] + INDEX[code and 7]).coerceIn(0, 88)
                    out[(i - start) * 2 + c] = pred[c].toShort()
                }
            }
            return out
        }

        private companion object {
            const val STATE_LEN = 8
            val STEP = intArrayOf(
                7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
                73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408,
                449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066,
                2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630,
                9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
                32767,
            )
            val INDEX = intArrayOf(-1, -1, -1, -1, 2, 4, 6, 8)
        }
    }

    // 코덱 1·2: Opus 48kHz 스테레오 (PC src-tauri/src/codec/opus.rs). 비트레이트는 PC 쪽 설정이라 디코더는 같다.
    // 안드로이드 내장 디코더(MediaCodec, Android 5.0+)를 쓴다. 패킷 하나 = 20ms 프레임 하나, 컨테이너 없음.
    class OpusStereo(override val id: Int, override val name: String) : Decoder {
        override val sampleRate = 48000
        override val channels = 2

        private val codec = MediaCodec.createDecoderByType(MediaFormat.MIMETYPE_AUDIO_OPUS)
        private val info = MediaCodec.BufferInfo()
        private var ptsUs = 0L

        init {
            val format = MediaFormat.createAudioFormat(MediaFormat.MIMETYPE_AUDIO_OPUS, sampleRate, channels)
            // MediaCodec 문서의 Opus 설정값: csd-0 = OpusHead(RFC 7845), csd-1 = pre-skip ns, csd-2 = seek pre-roll ns
            format.setByteBuffer("csd-0", opusHead())
            format.setByteBuffer("csd-1", nanos(0))
            format.setByteBuffer("csd-2", nanos(80_000_000))
            codec.configure(format, null, null, 0)
            codec.start()
        }

        override fun decode(data: ByteArray, offset: Int): ShortArray {
            val inIndex = codec.dequeueInputBuffer(10_000)
            if (inIndex >= 0) {
                val len = data.size - offset
                codec.getInputBuffer(inIndex)!!.apply { clear(); put(data, offset, len) }
                codec.queueInputBuffer(inIndex, 0, len, ptsUs, 0)
                ptsUs += 20_000
            }
            // 나온 만큼 꺼낸다. 첫 출력은 조금 기다려 줌 (안 기다리면 출력이 한 패킷씩 밀림)
            var out = ShortArray(0)
            var wait = 5_000L
            while (true) {
                val outIndex = codec.dequeueOutputBuffer(info, wait)
                wait = 0
                when {
                    outIndex >= 0 -> {
                        val buf = codec.getOutputBuffer(outIndex)!!
                        buf.position(info.offset).limit(info.offset + info.size)
                        val pcm = ShortArray(info.size / 2)
                        buf.order(ByteOrder.nativeOrder()).asShortBuffer().get(pcm)
                        codec.releaseOutputBuffer(outIndex, false)
                        out += pcm
                    }
                    outIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED -> {
                        val f = codec.outputFormat
                        val rate = f.getInteger(MediaFormat.KEY_SAMPLE_RATE)
                        val ch = f.getInteger(MediaFormat.KEY_CHANNEL_COUNT)
                        if (rate != sampleRate || ch != channels) Log.w("HOD", "Opus 출력 형식이 예상과 다름: ${rate}Hz ${ch}ch")
                    }
                    else -> break // 더 나올 게 없음
                }
            }
            return out
        }

        override fun close() {
            runCatching { codec.stop() }
            codec.release()
        }

        // RFC 7845 5.1: "OpusHead", 버전 1, 채널 수, pre-skip u16, 원래 샘플레이트 u32, 게인 i16, 매핑 0 = 19바이트
        private fun opusHead(): ByteBuffer = ByteBuffer.allocate(19).order(ByteOrder.LITTLE_ENDIAN).apply {
            put("OpusHead".toByteArray(Charsets.US_ASCII))
            put(1)
            put(channels.toByte())
            putShort(0)
            putInt(sampleRate)
            putShort(0)
            put(0)
            flip()
        }

        private fun nanos(ns: Long): ByteBuffer =
            ByteBuffer.allocate(8).order(ByteOrder.nativeOrder()).putLong(ns).apply { flip() }
    }
}
