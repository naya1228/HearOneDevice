// docs/CODECS.md
mod opus;
pub mod packet;
mod resample;

use crate::audio::AudioChunk;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Codec {
    OpusStereo64k = 1,
    OpusStereo128k = 2,
}

impl Codec {
    pub const ALL: &'static [Codec] = &[Codec::OpusStereo64k, Codec::OpusStereo128k];
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

    pub fn warning(self) -> Option<&'static str> {
        match self {
            Codec::OpusStereo64k => None,
            Codec::OpusStereo128k => Some("사람 많은 곳에서는 끊길 수 있어요"),
        }
    }
}

pub trait Encoder: Send {
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
