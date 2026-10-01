// 캡처 조각 → 16kHz 모노 μ-law 로 줄이고, BLE 패킷에 헤더를 붙인다.
//
// 패킷 형식 (android/.../Protocol.kt 와 같아야 함):
//   [0] 버전   = 1
//   [1] 코덱   = 1 (μ-law, 16kHz, 모노)
//   [2..4] 순번 u16 little-endian (청취자별, 손실 감지용)
//   [4..]  μ-law 바이트 (바이트 1개 = 샘플 1개)

use crate::audio::AudioChunk;

pub const PROTOCOL_VERSION: u8 = 1;
pub const CODEC_ULAW_16K_MONO: u8 = 1;
pub const HEADER_LEN: usize = 4;
pub const OUT_RATE: u32 = 16000;

/// 모노 다운믹스 + 16kHz 리샘플 + μ-law. 조각 경계를 넘는 상태를 유지한다.
pub struct Encoder {
    pending: Vec<f32>, // 아직 출력으로 못 바꾼 모노 샘플
    pos: f64,          // pending 안의 다음 출력 시작 위치
    in_rate: u32,
}

impl Encoder {
    pub fn new() -> Self {
        Self { pending: Vec::new(), pos: 0.0, in_rate: 0 }
    }

    pub fn encode(&mut self, chunk: &AudioChunk) -> Vec<u8> {
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
        let step = self.in_rate as f64 / OUT_RATE as f64;
        let mut out = Vec::with_capacity((self.pending.len() as f64 / step) as usize + 1);
        while self.pos + step <= self.pending.len() as f64 {
            let a = self.pos as usize;
            let b = ((self.pos + step) as usize).max(a + 1);
            let avg = self.pending[a..b].iter().sum::<f32>() / (b - a) as f32;
            out.push(linear_to_ulaw((avg.clamp(-1.0, 1.0) * 32767.0) as i16));
            self.pos += step;
        }
        let used = self.pos as usize;
        self.pending.drain(..used);
        self.pos -= used as f64;
        out
    }
}

/// 청취자별로 순번을 매겨 MTU에 맞는 패킷으로 자른다.
pub struct Packetizer {
    seq: u16,
    max_payload: usize,
}

impl Packetizer {
    /// max_packet: 한 번에 보낼 수 있는 최대 바이트 (헤더 포함)
    pub fn new(max_packet: usize) -> Self {
        Self { seq: 0, max_payload: max_packet.saturating_sub(HEADER_LEN).max(1) }
    }

    pub fn packets(&mut self, ulaw: &[u8]) -> Vec<Vec<u8>> {
        ulaw.chunks(self.max_payload)
            .map(|part| {
                let mut p = Vec::with_capacity(HEADER_LEN + part.len());
                p.push(PROTOCOL_VERSION);
                p.push(CODEC_ULAW_16K_MONO);
                p.extend_from_slice(&self.seq.to_le_bytes());
                p.extend_from_slice(part);
                self.seq = self.seq.wrapping_add(1);
                p
            })
            .collect()
    }
}

// G.711 μ-law
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

    fn chunk(rate: u32, frames: usize) -> AudioChunk {
        AudioChunk { sample_rate: rate, channels: 2, samples: vec![0.0; frames * 2] }
    }

    #[test]
    fn ulaw_known_values() {
        assert_eq!(linear_to_ulaw(0), 0xFF);
        assert_eq!(linear_to_ulaw(-1), 0x7F);
        assert_eq!(linear_to_ulaw(32767), 0x80);
        assert_eq!(linear_to_ulaw(-32768), 0x00);
    }

    #[test]
    fn resample_48k() {
        let mut e = Encoder::new();
        assert_eq!(e.encode(&chunk(48000, 960)).len(), 320);
    }

    #[test]
    fn resample_44k1_keeps_rate_across_chunks() {
        let mut e = Encoder::new();
        // 1초 분량을 불규칙한 조각으로
        let total: usize = [441, 1000, 7, 42652]
            .iter()
            .map(|&n| e.encode(&chunk(44100, n)).len())
            .sum();
        assert!((15999..=16000).contains(&total), "{total}");
    }

    #[test]
    fn packets_have_header_and_fit() {
        let mut p = Packetizer::new(100);
        let pk = p.packets(&[7u8; 250]);
        assert_eq!(pk.len(), 3);
        assert!(pk.iter().all(|x| x.len() <= 100));
        assert_eq!(&pk[2][..4], &[1, 1, 2, 0]);
        assert_eq!(pk.iter().map(|x| x.len() - HEADER_LEN).sum::<usize>(), 250);
    }
}
