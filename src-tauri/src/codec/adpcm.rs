// 코덱 2·3: IMA ADPCM 스테레오 (샘플 하나 = 4bit). 앱 쪽은 android/.../Codecs.kt 의 AdpcmStereo.
//
// 프레임 하나 = 10ms. 패킷을 잃어도 다음 프레임부터 바로 회복되도록 프레임마다 시작 상태를 싣는다:
//   [0..2] L 예측값 i16 LE, [2] L 스텝 인덱스, [3] 0
//   [4..6] R 예측값 i16 LE, [6] R 스텝 인덱스, [7] 0
//   [8..]  샘플마다 1바이트: 아래 4bit = L, 위 4bit = R

use super::resample::Resampler;
use super::Encoder;
use crate::audio::AudioChunk;

const STATE_LEN: usize = 8;

const STEP: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408,
    449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066,
    2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630,
    9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
    32767,
];
const INDEX: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

/// 채널 하나의 ADPCM 상태
#[derive(Clone, Copy, Default)]
struct Channel {
    predictor: i32,
    index: i32,
}

impl Channel {
    fn encode(&mut self, sample: i16) -> u8 {
        let mut step = STEP[self.index as usize];
        let mut diff = sample as i32 - self.predictor;
        let mut code = 0u8;
        if diff < 0 {
            code = 8;
            diff = -diff;
        }
        // 디코더와 똑같이 delta를 쌓아야 예측값이 어긋나지 않는다
        let mut delta = step >> 3;
        for bit in [4u8, 2, 1] {
            if diff >= step {
                code |= bit;
                diff -= step;
                delta += step;
            }
            step >>= 1;
        }
        self.apply(code, delta);
        code
    }

    fn apply(&mut self, code: u8, delta: i32) {
        self.predictor += if code & 8 != 0 { -delta } else { delta };
        self.predictor = self.predictor.clamp(-32768, 32767);
        self.index = (self.index + INDEX[(code & 7) as usize]).clamp(0, 88);
    }

    fn write_state(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&(self.predictor as i16).to_le_bytes());
        out.push(self.index as u8);
        out.push(0);
    }
}

pub struct AdpcmEncoder {
    resampler: Resampler,
    frame_len: usize,   // 프레임 하나의 샘플 수 (채널당)
    pending: Vec<f32>,  // 아직 프레임을 못 채운 스테레오 샘플
    ch: [Channel; 2],
}

impl AdpcmEncoder {
    pub fn new(rate: u32) -> Self {
        Self {
            resampler: Resampler::new(rate, 2),
            frame_len: rate as usize / 100, // 10ms
            pending: Vec::new(),
            ch: [Channel::default(); 2],
        }
    }
}

impl Encoder for AdpcmEncoder {
    fn encode(&mut self, chunk: &AudioChunk) -> Vec<Vec<u8>> {
        self.pending.extend(self.resampler.process(chunk));
        let need = self.frame_len * 2;
        let mut frames = Vec::new();
        while self.pending.len() >= need {
            let mut f = Vec::with_capacity(STATE_LEN + self.frame_len);
            self.ch[0].write_state(&mut f);
            self.ch[1].write_state(&mut f);
            for lr in self.pending[..need].chunks_exact(2) {
                let l = self.ch[0].encode((lr[0] * 32767.0) as i16);
                let r = self.ch[1].encode((lr[1] * 32767.0) as i16);
                f.push(l | (r << 4));
            }
            self.pending.drain(..need);
            frames.push(f);
        }
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 앱(Codecs.kt)과 같은 디코딩. 인코더 검증용
    fn decode(frame: &[u8]) -> Vec<i16> {
        let mut ch = [0, 4].map(|o| Channel {
            predictor: i16::from_le_bytes([frame[o], frame[o + 1]]) as i32,
            index: frame[o + 2] as i32,
        });
        let mut out = Vec::new();
        for &b in &frame[STATE_LEN..] {
            for (c, code) in [(0, b & 0x0F), (1, b >> 4)] {
                let step = STEP[ch[c].index as usize];
                let mut delta = step >> 3;
                if code & 4 != 0 {
                    delta += step;
                }
                if code & 2 != 0 {
                    delta += step >> 1;
                }
                if code & 1 != 0 {
                    delta += step >> 2;
                }
                ch[c].apply(code, delta);
                out.push(ch[c].predictor as i16);
            }
        }
        out
    }

    fn sine(rate: u32, frames: usize) -> AudioChunk {
        let samples = (0..frames)
            .flat_map(|i| {
                let t = i as f32 / rate as f32;
                [0.5 * (t * 440.0 * 6.283).sin(), 0.3 * (t * 1000.0 * 6.283).sin()]
            })
            .collect();
        AudioChunk { sample_rate: rate, channels: 2, samples }
    }

    #[test]
    fn frames_are_10ms_and_fixed_size() {
        let mut e = AdpcmEncoder::new(32000);
        let frames = e.encode(&sine(48000, 4800)); // 100ms
        assert_eq!(frames.len(), 10);
        assert!(frames.iter().all(|f| f.len() == STATE_LEN + 320));
    }

    #[test]
    fn round_trip_is_close() {
        let mut e = AdpcmEncoder::new(48000);
        let c = sine(48000, 4800);
        let decoded: Vec<i16> = e.encode(&c).iter().flat_map(|f| decode(f)).collect();
        // 처음 몇 ms는 예측값이 따라오는 중이라 제외
        let err: f64 = c.samples[960..]
            .iter()
            .zip(&decoded[960..])
            .map(|(&a, &b)| ((a * 32767.0) as f64 - b as f64).abs())
            .sum::<f64>()
            / (decoded.len() - 960) as f64;
        assert!(err < 300.0, "평균 오차 {err}");
    }

    #[test]
    fn frame_decodes_alone() {
        // 앞 프레임을 잃어도 이 프레임만으로 같은 값이 나와야 한다
        let mut e = AdpcmEncoder::new(32000);
        let frames = e.encode(&sine(48000, 4800));
        let all: Vec<i16> = frames.iter().flat_map(|f| decode(f)).collect();
        let n = 32000 / 100 * 2;
        assert_eq!(decode(&frames[5]), all[5 * n..6 * n]);
    }
}
