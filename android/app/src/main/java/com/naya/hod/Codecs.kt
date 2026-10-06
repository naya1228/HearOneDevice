package com.naya.hod

import android.media.MediaCodec
import android.media.MediaFormat
import android.util.Log
import java.nio.ByteBuffer
import java.nio.ByteOrder

// 코덱 번호 목록. PC(src-tauri/src/codec/mod.rs 의 Codec)와 번호가 같아야 함. 표는 docs/CODECS.md
object Codecs {
    const val OPUS_64K_STEREO = 1
    const val OPUS_128K_STEREO = 2

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
        OPUS_64K_STEREO -> OpusStereo(OPUS_64K_STEREO, "Opus 48kHz 스테레오 64kbps")
        OPUS_128K_STEREO -> OpusStereo(OPUS_128K_STEREO, "Opus 48kHz 스테레오 128kbps")
        else -> null
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
