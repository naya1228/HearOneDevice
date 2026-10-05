// 코덱 1: G.711 μ-law (바이트 1개 = 샘플 1개)

use super::resample::Resampler;
use super::Encoder;
use crate::audio::AudioChunk;

pub struct UlawEncoder {
    resampler: Resampler,
}

impl UlawEncoder {
    pub fn new(rate: u32) -> Self {
        Self { resampler: Resampler::new(rate, 1) }
    }
}

impl Encoder for UlawEncoder {
    fn encode(&mut self, chunk: &AudioChunk) -> Vec<Vec<u8>> {
        let data = self
            .resampler
            .process(chunk)
            .into_iter()
            .map(|s| linear_to_ulaw((s * 32767.0) as i16))
            .collect();
        vec![data]
    }
}

fn linear_to_ulaw(sample: i16) -> u8 {
    const BIAS: i32 = 0x84;
    const CLIP: i32 = 32635;
    let mut s = sample as i32;
    let sign = if s < 0 {
        s = -s;
        0x80
    } else {
        0
    };
    s = s.min(CLIP) + BIAS;
    let mut exp = 7;
    let mut mask = 0x4000;
    while exp > 0 && s & mask == 0 {
        exp -= 1;
        mask >>= 1;
    }
    let mantissa = (s >> (exp + 3)) & 0x0F;
    !((sign | (exp << 4) | mantissa) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulaw_known_values() {
        assert_eq!(linear_to_ulaw(0), 0xFF);
        assert_eq!(linear_to_ulaw(-1), 0x7F);
        assert_eq!(linear_to_ulaw(32767), 0x80);
        assert_eq!(linear_to_ulaw(-32768), 0x00);
    }

    #[test]
    fn one_byte_per_sample() {
        let mut e = UlawEncoder::new(16000);
        let chunk = AudioChunk { sample_rate: 48000, channels: 2, samples: vec![0.0; 960 * 2] };
        assert_eq!(e.encode(&chunk), vec![vec![0xFF; 320]]);
    }
}
