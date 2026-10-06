// BLE 패킷 헤더 붙이기. 형식·제어 메시지는 docs/PROTOCOL.md

use super::Codec;

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 4;

/// 제어 메시지 (docs/PROTOCOL.md 4절)
pub const CONTROL: u8 = 0;
pub const CONTROL_STOP: u8 = 1;

/// 청취자별로 순번을 매겨 패킷을 만든다.
pub struct Packetizer {
    seq: u16,
    max_payload: usize,
}

impl Packetizer {
    /// max_packet: 한 번에 보낼 수 있는 최대 바이트 (헤더 포함)
    pub fn new(max_packet: usize) -> Self {
        Self { seq: 0, max_payload: max_packet.saturating_sub(HEADER_LEN).max(1) }
    }

    /// 프레임 하나 = 패킷 하나. MTU보다 크면 못 보내므로 버린다 (None).
    pub fn packet_for(&mut self, codec: Codec, frame: &[u8]) -> Option<Vec<u8>> {
        if frame.len() > self.max_payload {
            println!("[packet] 프레임 {}B가 MTU 여유 {}B보다 커서 버림", frame.len(), self.max_payload);
            return None;
        }
        Some(self.packet(codec.id(), frame))
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
    fn packet_has_header() {
        let mut p = Packetizer::new(100);
        p.packet_for(Codec::OpusStereo64k, &[7u8; 10]);
        let pk = p.packet_for(Codec::OpusStereo128k, &[7u8; 96]).unwrap();
        assert_eq!(&pk[..4], &[1, 2, 1, 0]);
        assert_eq!(pk.len(), 100);
    }

    #[test]
    fn oversized_frame_is_dropped() {
        let mut p = Packetizer::new(100);
        assert_eq!(p.packet_for(Codec::OpusStereo64k, &[0u8; 97]), None);
    }

    #[test]
    fn control_packet_continues_seq() {
        let mut p = Packetizer::new(100);
        p.packet_for(Codec::OpusStereo64k, &[0u8; 10]);
        assert_eq!(p.control(CONTROL_STOP), vec![1, CONTROL, 1, 0, CONTROL_STOP]);
    }
}
