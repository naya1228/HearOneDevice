// 모노 다운믹스 + 리샘플. 여러 코덱이 같이 쓴다.

use crate::audio::AudioChunk;

/// 캡처 조각을 out_rate 모노(-1.0..1.0)로 바꾼다. 조각 경계를 넘는 상태를 유지한다.
pub struct Resampler {
    out_rate: u32,
    pending: Vec<f32>, // 아직 출력으로 못 바꾼 모노 샘플
    pos: f64,          // pending 안의 다음 출력 시작 위치
    in_rate: u32,
}

impl Resampler {
    pub fn new(out_rate: u32) -> Self {
        Self { out_rate, pending: Vec::new(), pos: 0.0, in_rate: 0 }
    }

    pub fn process(&mut self, chunk: &AudioChunk) -> Vec<f32> {
        if chunk.sample_rate != self.in_rate {
            self.in_rate = chunk.sample_rate;
            self.pending.clear();
            self.pos = 0.0;
        }
        let ch = chunk.channels.max(1) as usize;
        self.pending.extend(
            chunk.samples.chunks_exact(ch).map(|f| f.iter().sum::<f32>() / ch as f32),
        );

        // 출력 샘플 하나 = 입력 step개 구간의 평균 (간이 저역통과 겸 다운샘플)
        let step = self.in_rate as f64 / self.out_rate as f64;
        let mut out = Vec::with_capacity((self.pending.len() as f64 / step) as usize + 1);
        while self.pos + step <= self.pending.len() as f64 {
            let a = self.pos as usize;
            let b = ((self.pos + step) as usize).max(a + 1);
            let avg = self.pending[a..b].iter().sum::<f32>() / (b - a) as f32;
            out.push(avg.clamp(-1.0, 1.0));
            self.pos += step;
        }
        let used = self.pos as usize;
        self.pending.drain(..used);
        self.pos -= used as f64;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(rate: u32, frames: usize) -> AudioChunk {
        AudioChunk { sample_rate: rate, channels: 2, samples: vec![0.0; frames * 2] }
    }

    #[test]
    fn resample_48k() {
        let mut r = Resampler::new(16000);
        assert_eq!(r.process(&chunk(48000, 960)).len(), 320);
    }

    #[test]
    fn resample_44k1_keeps_rate_across_chunks() {
        let mut r = Resampler::new(16000);
        // 1초 분량을 불규칙한 조각으로
        let total: usize = [441, 1000, 7, 42652]
            .iter()
            .map(|&n| r.process(&chunk(44100, n)).len())
            .sum();
        assert!((15999..=16000).contains(&total), "{total}");
    }
}
