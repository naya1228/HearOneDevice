// docs/CODECS.md: 구현된 코덱
use super::resample::Resampler;
use super::Encoder;
use crate::audio::AudioChunk;
use ::opus::{Application, Bitrate, Channels};

const RATE: u32 = 48000;
const FRAME_LEN: usize = RATE as usize / 50;
const MAX_FRAME_BYTES: usize = 480;

pub struct OpusEncoder {
    resampler: Resampler,
    enc: ::opus::Encoder,
    pending_samples: Vec<f32>,
}

impl OpusEncoder {
    pub fn new(bitrate: i32) -> Self {
        let mut enc = ::opus::Encoder::new(RATE, Channels::Stereo, Application::Audio)
            .expect("Opus 인코더 생성 실패");
        enc.set_bitrate(Bitrate::Bits(bitrate)).expect("Opus 비트레이트 설정 실패");
        Self { resampler: Resampler::new(RATE, 2), enc, pending_samples: Vec::new() }
    }
}

impl Encoder for OpusEncoder {
    fn encode(&mut self, chunk: &AudioChunk) -> Vec<Vec<u8>> {
        self.pending_samples.extend(self.resampler.process(chunk));
        let need = FRAME_LEN * 2;
        let mut frames = Vec::new();
        while self.pending_samples.len() >= need {
            match self.enc.encode_vec_float(&self.pending_samples[..need], MAX_FRAME_BYTES) {
                Ok(f) => frames.push(f),
                Err(e) => println!("[opus] 인코딩 실패: {e}"),
            }
            self.pending_samples.drain(..need);
        }
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frames: usize) -> AudioChunk {
        let samples = (0..frames)
            .flat_map(|i| {
                let t = i as f32 / RATE as f32;
                [0.5 * (t * 440.0 * 6.283).sin(), 0.3 * (t * 1000.0 * 6.283).sin()]
            })
            .collect();
        AudioChunk { sample_rate: RATE, channels: 2, samples }
    }

    #[test]
    fn frames_are_20ms_and_fit_packet() {
        let mut e = OpusEncoder::new(128_000);
        let frames = e.encode(&sine(RATE as usize / 10));
        assert_eq!(frames.len(), 5);
        assert!(frames.iter().all(|f| !f.is_empty() && f.len() <= MAX_FRAME_BYTES));
    }

    #[test]
    fn decodes_back() {
        let mut e = OpusEncoder::new(64_000);
        let mut d = ::opus::Decoder::new(RATE, Channels::Stereo).unwrap();
        let mut out = vec![0i16; FRAME_LEN * 2];
        let mut total = 0;
        let mut energy = 0f64;
        for f in e.encode(&sine(48000)) {
            let n = d.decode(&f, &mut out, false).unwrap();
            total += n;
            energy += out[..n * 2].iter().map(|&s| (s as f64).powi(2)).sum::<f64>();
        }
        assert_eq!(total, 48000);
        assert!(energy / (total * 2) as f64 > 1e6, "{energy}");
    }
}
