// docs/FEATURES.md: 인코딩
use crate::audio::AudioChunk;

pub struct Resampler {
    out_rate: u32,
    out_channels: usize,
    pending_samples: Vec<f32>,
    pos_frames: f64,
    in_rate: u32,
}

impl Resampler {
    pub fn new(out_rate: u32, out_channels: usize) -> Self {
        assert!(matches!(out_channels, 1 | 2));
        Self { out_rate, out_channels, pending_samples: Vec::new(), pos_frames: 0.0, in_rate: 0 }
    }

    pub fn process(&mut self, chunk: &AudioChunk) -> Vec<f32> {
        if chunk.sample_rate != self.in_rate {
            self.in_rate = chunk.sample_rate;
            self.pending_samples.clear();
            self.pos_frames = 0.0;
        }
        let ch = chunk.channels.max(1) as usize;
        let oc = self.out_channels;
        for f in chunk.samples.chunks_exact(ch) {
            if oc == 1 {
                self.pending_samples.push(f.iter().sum::<f32>() / ch as f32);
            } else {
                self.pending_samples.push(f[0]);
                self.pending_samples.push(f[1.min(ch - 1)]);
            }
        }

        let frames = self.pending_samples.len() / oc;
        let step = self.in_rate as f64 / self.out_rate as f64;
        let mut out = Vec::with_capacity(((frames as f64 / step) as usize + 1) * oc);
        while self.pos_frames + step <= frames as f64 {
            let a = self.pos_frames as usize;
            let b = ((self.pos_frames + step) as usize).max(a + 1);
            for c in 0..oc {
                let sum: f32 = (a..b).map(|i| self.pending_samples[i * oc + c]).sum();
                out.push((sum / (b - a) as f32).clamp(-1.0, 1.0));
            }
            self.pos_frames += step;
        }
        let used = self.pos_frames as usize;
        self.pending_samples.drain(..used * oc);
        self.pos_frames -= used as f64;
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
        let mut r = Resampler::new(16000, 1);
        assert_eq!(r.process(&chunk(48000, 960)).len(), 320);
    }

    #[test]
    fn resample_44k1_keeps_rate_across_chunks() {
        let mut r = Resampler::new(16000, 1);
        let total: usize = [441, 1000, 7, 42652]
            .iter()
            .map(|&n| r.process(&chunk(44100, n)).len())
            .sum();
        assert!((15999..=16000).contains(&total), "{total}");
    }

    #[test]
    fn stereo_keeps_left_right() {
        let mut r = Resampler::new(48000, 2);
        let c = AudioChunk { sample_rate: 48000, channels: 2, samples: [0.5, -0.5].repeat(10) };
        assert_eq!(r.process(&c), [0.5, -0.5].repeat(10));
    }
}
