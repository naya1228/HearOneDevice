// 코덱 공통: 코덱 번호 목록 + 인코더 고르기.
// 어떤 코덱으로 보낼지는 실행 중에 고른다 (encoding.rs). 코덱 표는 docs/CODECS.md.
// 패킷 헤더·자르기는 packet.rs.

mod adpcm;
mod opus;
pub mod packet;
mod resample;
mod ulaw;

use crate::audio::AudioChunk;

/// 코덱 번호. 패킷 헤더에 들어가므로 android/.../Codecs.kt 와 번호가 같아야 함.
/// 새 코덱은 번호를 하나 추가하고 ALL·name()·framed()·encoder()에 연결한다 (번호는 재사용 금지).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Codec {
    /// μ-law, 16kHz 모노 (16KB/s)
    Ulaw16kMono = 1,
    /// IMA ADPCM, 32kHz 스테레오 (약 33KB/s)
    AdpcmStereo32k = 2,
    /// IMA ADPCM, 48kHz 스테레오 (약 49KB/s)
    AdpcmStereo48k = 3,
    /// Opus 48kHz 스테레오 128kbps (16KB/s)
    OpusStereo128k = 4,
    /// Opus 48kHz 스테레오 64kbps (8KB/s)
    OpusStereo64k = 5,
}

impl Codec {
    /// 화면에서 고를 수 있는 코덱 목록
    pub const ALL: &'static [Codec] =
        &[
            Codec::Ulaw16kMono,
            Codec::AdpcmStereo32k,
            Codec::AdpcmStereo48k,
            Codec::OpusStereo128k,
            Codec::OpusStereo64k,
        ];
    /// 앱을 켰을 때 처음 쓰는 코덱
    pub const DEFAULT: Codec = Codec::OpusStereo128k;

    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<Codec> {
        Self::ALL.iter().copied().find(|c| c.id() == id)
    }

    pub fn name(self) -> &'static str {
        match self {
            Codec::Ulaw16kMono => "μ-law 16kHz 모노",
            Codec::AdpcmStereo32k => "ADPCM 32kHz 스테레오",
            Codec::AdpcmStereo48k => "ADPCM 48kHz 스테레오",
            Codec::OpusStereo128k => "Opus 48kHz 스테레오 128kbps",
            Codec::OpusStereo64k => "Opus 48kHz 스테레오 64kbps",
        }
    }

    /// true: encode()가 내놓는 조각 하나하나가 프레임이라 쪼개면 못 푼다 → 패킷 하나에 프레임 하나.
    /// false: 바이트 단위로 아무 데서나 잘라도 된다 (μ-law, PCM).
    pub fn framed(self) -> bool {
        match self {
            Codec::Ulaw16kMono => false,
            Codec::AdpcmStereo32k
            | Codec::AdpcmStereo48k
            | Codec::OpusStereo128k
            | Codec::OpusStereo64k => true,
        }
    }
}

/// 캡처 조각 → 코덱 데이터 조각들. 조각 경계를 넘는 상태를 가질 수 있다.
pub trait Encoder: Send {
    /// framed 코덱이면 조각 하나 = 프레임 하나. 아니면 몇 개로 나뉘든 상관없다.
    fn encode(&mut self, chunk: &AudioChunk) -> Vec<Vec<u8>>;
}

pub fn encoder(codec: Codec) -> Box<dyn Encoder> {
    match codec {
        Codec::Ulaw16kMono => Box::new(ulaw::UlawEncoder::new(16000)),
        Codec::AdpcmStereo32k => Box::new(adpcm::AdpcmEncoder::new(32000)),
        Codec::AdpcmStereo48k => Box::new(adpcm::AdpcmEncoder::new(48000)),
        Codec::OpusStereo128k => Box::new(opus::OpusEncoder::new(128_000)),
        Codec::OpusStereo64k => Box::new(opus::OpusEncoder::new(64_000)),
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
