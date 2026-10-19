package com.naya.hod

import android.media.MediaCodec
import android.media.MediaFormat
import android.util.Log
import java.nio.ByteBuffer
import java.nio.ByteOrder

// docs/CODECS.md
object Codecs {
    const val OPUS_64K_STEREO = 1
    const val OPUS_128K_STEREO = 2

    interface Decoder {
        val id: Int
        val name: String
        val sampleRate: Int
        val channels: Int
        fun decode(data: ByteArray, offset: Int): ShortArray
        fun close() {}
    }

    fun decoder(id: Int): Decoder? = when (id) {
        OPUS_64K_STEREO -> OpusStereo(OPUS_64K_STEREO, "Opus 64kbps (기본)")
        OPUS_128K_STEREO -> OpusStereo(OPUS_128K_STEREO, "Opus 128kbps (고음질)")
        else -> null
    }

    class OpusStereo(override val id: Int, override val name: String) : Decoder {
        private companion object {
            const val FRAME_US = 20_000L
            const val INPUT_WAIT_US = 10_000L
            const val FIRST_OUTPUT_WAIT_US = 5_000L
            const val PRE_SKIP_NS = 0L
            const val PRE_ROLL_NS = 80_000_000L
        }

        override val sampleRate = 48000
        override val channels = 2

        private val codec = MediaCodec.createDecoderByType(MediaFormat.MIMETYPE_AUDIO_OPUS)
        private val info = MediaCodec.BufferInfo()
        private var ptsUs = 0L

        init {
            val format = MediaFormat.createAudioFormat(MediaFormat.MIMETYPE_AUDIO_OPUS, sampleRate, channels)
            format.setByteBuffer("csd-0", opusHead())
            format.setByteBuffer("csd-1", nanos(PRE_SKIP_NS))
            format.setByteBuffer("csd-2", nanos(PRE_ROLL_NS))
            codec.configure(format, null, null, 0)
            codec.start()
        }

        override fun decode(data: ByteArray, offset: Int): ShortArray {
            val inIndex = codec.dequeueInputBuffer(INPUT_WAIT_US)
            if (inIndex >= 0) {
                val len = data.size - offset
                codec.getInputBuffer(inIndex)!!.apply { clear(); put(data, offset, len) }
                codec.queueInputBuffer(inIndex, 0, len, ptsUs, 0)
                ptsUs += FRAME_US
            }
            var out = ShortArray(0)
            var wait = FIRST_OUTPUT_WAIT_US
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
                    else -> break
                }
            }
            return out
        }

        override fun close() {
            runCatching { codec.stop() }
            codec.release()
        }

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
