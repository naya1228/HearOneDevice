// BLE 패킷 헤더 붙이기 (+ 확인 통과 뒤엔 잠그기). 형식·제어 메시지는 docs/PROTOCOL.md

use super::Codec;
use crate::auth::{Sealer, TAG_LEN};

pub const PROTOCOL_VERSION: u8 = 2;
pub const HEADER_LEN: usize = 4;

/// 제어 메시지 (docs/PROTOCOL.md 4절)
pub const CONTROL: u8 = 0;
pub const CONTROL_STOP: u8 = 1;
pub const CONTROL_CHALLENGE: u8 = 2;
pub const CONTROL_AUTH_OK: u8 = 3;
pub const CONTROL_AUTH_FAIL: u8 = 4;

/// 청취자별로 순번을 매겨 패킷을 만든다.
pub struct Packetizer {
    /// 긴 순번 (docs/PROTOCOL.md 6절). 헤더엔 아래 16비트만 실림
    seq: u64,
    max_payload: usize,
    sealer: Option<Sealer>,
}

impl Packetizer {
    /// max_packet: 한 번에 보낼 수 있는 최대 바이트 (헤더 포함)
    pub fn new(max_packet: usize) -> Self {
        Self { seq: 0, max_payload: max_packet.saturating_sub(HEADER_LEN).max(1), sealer: None }
    }

    /// 이 뒤로 만드는 패킷은 전부 잠근다 (확인 통과 뒤). 되돌리지 않음
    pub fn seal_from_now(&mut self, sealer: Sealer) {
        self.sealer = Some(sealer);
    }

    /// 프레임 하나 = 패킷 하나. MTU보다 크면 못 보내므로 버린다 (None).
    pub fn packet_for(&mut self, codec: Codec, frame: &[u8]) -> Option<Vec<u8>> {
        let room = self.max_payload.saturating_sub(if self.sealer.is_some() { TAG_LEN } else { 0 });
        if frame.len() > room {
            println!("[packet] 프레임 {}B가 MTU 여유 {room}B보다 커서 버림", frame.len());
            return None;
        }
        Some(self.packet(codec.id(), frame))
    }

    /// 제어 메시지 패킷 (cmd = CONTROL_STOP 등)
    pub fn control(&mut self, cmd: u8) -> Vec<u8> {
        self.control_with(cmd, &[])
    }

    /// 뒤에 데이터가 붙는 제어 메시지 (CONTROL_CHALLENGE 등)
    pub fn control_with(&mut self, cmd: u8, data: &[u8]) -> Vec<u8> {
        let mut part = Vec::with_capacity(1 + data.len());
        part.push(cmd);
        part.extend_from_slice(data);
        self.packet(CONTROL, &part)
    }

    fn packet(&mut self, codec_id: u8, part: &[u8]) -> Vec<u8> {
        let mut p = Vec::with_capacity(HEADER_LEN + part.len() + TAG_LEN);
        p.push(PROTOCOL_VERSION);
        p.push(codec_id);
        p.extend_from_slice(&(self.seq as u16).to_le_bytes());
        match &self.sealer {
            Some(s) => {
                let sealed = s.seal(self.seq, &p, part);
                p.extend_from_slice(&sealed);
            }
            None => p.extend_from_slice(part),
        }
        // 같은 번호를 다시 쓰지 않도록 늘리기만 함 (u64라 사실상 끝나지 않음)
        self.seq += 1;
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
        assert_eq!(&pk[..4], &[2, 2, 1, 0]);
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
        assert_eq!(p.control(CONTROL_STOP), vec![2, CONTROL, 1, 0, CONTROL_STOP]);
    }

    #[test]
    fn control_packet_carries_data() {
        let mut p = Packetizer::new(100);
        assert_eq!(p.control_with(CONTROL_CHALLENGE, &[9, 8]), vec![2, CONTROL, 0, 0, CONTROL_CHALLENGE, 9, 8]);
    }

    const AUDIO_KEY: &str = "90de82fa19fcc35801587b8582a6fb07730eefba845edac03b027ff24a417245";

    fn sealed_packetizer(max_packet: usize) -> Packetizer {
        let key = (0..32).map(|i| u8::from_str_radix(&AUDIO_KEY[i * 2..i * 2 + 2], 16).unwrap()).collect::<Vec<_>>();
        let mut p = Packetizer::new(max_packet);
        p.seal_from_now(Sealer::new(&key.try_into().unwrap()));
        p
    }

    // 긴 순번 70000 = 헤더 순번 0x1170 (65535 넘어 한 바퀴 돈 뒤). 값은 auth/seal.rs 테스트와 같음
    #[test]
    fn sealed_packet_keeps_header_and_uses_long_seq() {
        let mut p = sealed_packetizer(100);
        p.seq = 70000;
        let pk = p.packet_for(Codec::OpusStereo64k, &[5; 10]).unwrap();
        assert_eq!(&pk[..4], &[2, 1, 0x70, 0x11]);
        assert_eq!(pk[4..].len(), 10 + TAG_LEN);
        assert_eq!(pk[4..6], [0x38, 0xa1]);
        assert_eq!(p.seq, 70001);
    }

    #[test]
    fn sealed_frame_needs_room_for_tag() {
        let mut p = sealed_packetizer(100);
        assert_eq!(p.packet_for(Codec::OpusStereo64k, &[0u8; 81]), None);
        assert_eq!(p.packet_for(Codec::OpusStereo64k, &[0u8; 80]).unwrap().len(), 100);
    }
}
