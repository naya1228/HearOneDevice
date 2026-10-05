// BLE 패킷 헤더 + MTU에 맞게 자르기.
//
// 패킷 형식 (android/.../Protocol.kt 와 같아야 함):
//   [0] 버전   = 1
//   [1] 코덱 번호 (codec/mod.rs 의 Codec, android/.../Codecs.kt 와 같아야 함)
//   [2..4] 순번 u16 little-endian (청취자별, 손실 감지용)
//   [4..]  코덱 데이터

use super::Codec;

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 4;

/// 청취자별로 순번을 매겨 MTU에 맞는 패킷으로 만든다.
pub struct Packetizer {
    seq: u16,
    max_payload: usize,
}

impl Packetizer {
    /// max_packet: 한 번에 보낼 수 있는 최대 바이트 (헤더 포함)
    pub fn new(max_packet: usize) -> Self {
        Self { seq: 0, max_payload: max_packet.saturating_sub(HEADER_LEN).max(1) }
    }

    /// framed 코덱은 조각 하나를 패킷 하나로 (MTU보다 크면 못 보내므로 버림),
    /// 아니면 MTU에 맞게 잘라서 여러 패킷으로.
    pub fn packets(&mut self, codec: Codec, data: &[u8]) -> Vec<Vec<u8>> {
        let parts: Vec<&[u8]> = if codec.framed() {
            if data.len() > self.max_payload {
                println!(
                    "[packet] 프레임 {}B가 MTU 여유 {}B보다 커서 버림",
                    data.len(),
                    self.max_payload
                );
                return Vec::new();
            }
            vec![data]
        } else {
            data.chunks(self.max_payload).collect()
        };
        parts
            .into_iter()
            .map(|part| {
                let mut p = Vec::with_capacity(HEADER_LEN + part.len());
                p.push(PROTOCOL_VERSION);
                p.push(codec.id());
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
        let mut p = Packetizer::new(100);
        let pk = p.packets(Codec::Ulaw16kMono, &[7u8; 250]);
        assert_eq!(pk.len(), 3);
        assert!(pk.iter().all(|x| x.len() <= 100));
        assert_eq!(&pk[2][..4], &[1, 5, 2, 0]);
        assert_eq!(pk.iter().map(|x| x.len() - HEADER_LEN).sum::<usize>(), 250);
    }
}
