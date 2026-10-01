// 코덱 공통: 코덱 번호 목록 + 인코더 고르기 + BLE 패킷 헤더.
// 어떤 코덱으로 보낼지는 lib.rs 의 CODEC. 코덱 표는 docs/CODECS.md.
//
// 패킷 형식 (android/.../Protocol.kt 와 같아야 함):
//   [0] 버전   = 1
//   [1] 코덱 번호 (아래 Codec, android/.../Codecs.kt 와 같아야 함)
//   [2..4] 순번 u16 little-endian (청취자별, 손실 감지용)
//   [4..]  코덱 데이터

mod resample;
mod ulaw;

use crate::audio::AudioChunk;

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 4;

/// 코덱 번호. 새 코덱은 번호를 하나 추가하고 encoder()에 연결한다 (번호는 재사용 금지).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Codec {
    /// μ-law, 16kHz 모노 (16KB/s)
    Ulaw16kMono = 1,
}

impl Codec {
    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn name(self) -> &'static str {
        match self {
            Codec::Ulaw16kMono => "μ-law 16kHz 모노",
        }
    }
}

/// 캡처 조각 → 코덱 데이터. 조각 경계를 넘는 상태를 가질 수 있다.
pub trait Encoder: Send {
    fn encode(&mut self, chunk: &AudioChunk) -> Vec<u8>;
}

pub fn encoder(codec: Codec) -> Box<dyn Encoder> {
    match codec {
        Codec::Ulaw16kMono => Box::new(ulaw::UlawEncoder::new(16000)),
    }
}

/// 청취자별로 순번을 매겨 MTU에 맞는 패킷으로 자른다.
/// 바이트 단위로 잘라도 되는 코덱 기준 (μ-law, PCM). 프레임 단위 코덱(Opus 등)은 따로 처리해야 함.
pub struct Packetizer {
    codec: u8,
    seq: u16,
    max_payload: usize,
}

impl Packetizer {
    /// max_packet: 한 번에 보낼 수 있는 최대 바이트 (헤더 포함)
    pub fn new(max_packet: usize, codec: Codec) -> Self {
        Self { codec: codec.id(), seq: 0, max_payload: max_packet.saturating_sub(HEADER_LEN).max(1) }
    }

    pub fn packets(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
        data.chunks(self.max_payload)
            .map(|part| {
                let mut p = Vec::with_capacity(HEADER_LEN + part.len());
                p.push(PROTOCOL_VERSION);
                p.push(self.codec);
                p.extend_from_slice(&self.seq.to_le_bytes());
                p.extend_from_slice(part);
                self.seq = self.seq.wrapping_add(1);
                p
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packets_have_header_and_fit() {
        let mut p = Packetizer::new(100, Codec::Ulaw16kMono);
        let pk = p.packets(&[7u8; 250]);
        assert_eq!(pk.len(), 3);
        assert!(pk.iter().all(|x| x.len() <= 100));
        assert_eq!(&pk[2][..4], &[1, 1, 2, 0]);
        assert_eq!(pk.iter().map(|x| x.len() - HEADER_LEN).sum::<usize>(), 250);
    }
}
