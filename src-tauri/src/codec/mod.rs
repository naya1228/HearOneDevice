// 코덱 공통: 코덱 번호 목록 + 인코더 고르기.
// 어떤 코덱으로 보낼지는 실행 중에 고른다 (encoding.rs). 코덱 표는 docs/CODECS.md.
// 패킷 헤더는 packet.rs.

mod opus;
pub mod packet;
mod resample;

use crate::audio::AudioChunk;

/// 코덱 번호. 패킷 헤더에 들어가므로 android/.../Codecs.kt 와 번호가 같아야 함.
/// 새 코덱은 번호를 하나 추가하고 ALL·name()·warning()·encoder()에 연결한다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Codec {
    /// Opus 48kHz 스테레오 64kbps (8KB/s)
    OpusStereo64k = 1,
    /// Opus 48kHz 스테레오 128kbps (16KB/s)
    OpusStereo128k = 2,
}

impl Codec {
    /// 화면에서 고를 수 있는 코덱 목록
    pub const ALL: &'static [Codec] = &[Codec::OpusStereo64k, Codec::OpusStereo128k];
    /// 앱을 켰을 때 처음 쓰는 코덱
    pub const DEFAULT: Codec = Codec::OpusStereo64k;

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<Codec> {
        Self::ALL.iter().copied().find(|c| c.id() == id)
    }

    pub fn name(self) -> &'static str {
        match self {
            Codec::OpusStereo64k => "Opus 64kbps (기본)",
            Codec::OpusStereo128k => "Opus 128kbps (고음질)",
        }
    }

    /// 이 코덱을 골랐을 때 화면에 띄울 주의 문구
    pub fn warning(self) -> Option<&'static str> {
        match self {
            Codec::OpusStereo64k => None,
            // 전파를 두 배로 써서 혼잡한 2.4GHz에서 재전송이 몰림 (docs/CODECS.md)
            Codec::OpusStereo128k => Some("사람 많은 곳에서는 끊길 수 있어요"),
        }
    }
}

/// 캡처 조각 → 코덱 프레임들. 조각 경계를 넘는 상태를 가질 수 있다.
pub trait Encoder: Send {
    /// 내놓는 조각 하나 = 프레임 하나 = 패킷 하나 (쪼개면 못 푼다)
    fn encode(&mut self, chunk: &AudioChunk) -> Vec<Vec<u8>>;
}

pub fn encoder(codec: Codec) -> Box<dyn Encoder> {
    match codec {
        Codec::OpusStereo64k => Box::new(opus::OpusEncoder::new(64_000)),
        Codec::OpusStereo128k => Box::new(opus::OpusEncoder::new(128_000)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for &c in Codec::ALL {
            assert_eq!(Codec::from_id(c.id()), Some(c));
        }
        assert_eq!(Codec::from_id(0), None);
    }
}
