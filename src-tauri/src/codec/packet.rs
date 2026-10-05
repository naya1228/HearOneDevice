// BLE 패킷 헤더 + MTU에 맞게 자르기. 형식·제어 메시지는 docs/PROTOCOL.md

use super::Codec;

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 4;

/// 제어 메시지 (docs/PROTOCOL.md 4절)
pub const CONTROL: u8 = 0;
pub const CONTROL_STOP: u8 = 1;

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
        parts.into_iter().map(|part| self.packet(codec.id(), part)).collect()
    }

    /// 제어 메시지 패킷 (cmd = CONTROL_STOP 등)
    pub fn control(&mut self, cmd: u8) -> Vec<u8> {
        self.packet(CONTROL, &[cmd])
    }

    fn packet(&mut self, codec_id: u8, part: &[u8]) -> Vec<u8> {
        let mut p = Vec::with_capacity(HEADER_LEN + part.len());
        p.push(PROTOCOL_VERSION);
        p.push(codec_id);
        p.extend_from_slice(&self.seq.to_le_bytes());
        p.extend_from_slice(part);
        self.seq = self.seq.wrapping_add(1);
        p
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

    #[test]
    fn control_packet_continues_seq() {
        let mut p = Packetizer::new(100);
        p.packets(Codec::Ulaw16kMono, &[0u8; 10]);
        assert_eq!(p.control(CONTROL_STOP), vec![1, CONTROL, 1, 0, CONTROL_STOP]);
    }
}
