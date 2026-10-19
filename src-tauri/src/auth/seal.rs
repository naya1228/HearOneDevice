// docs/PROTOCOL.md: 잠그기
use super::key::Key;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};

pub const TAG_LEN: usize = 16;

pub struct Sealer(Aes256Gcm);

impl Sealer {
    pub fn new(key: &Key) -> Self {
        Self(Aes256Gcm::new(key.into()))
    }

    pub fn seal(&self, seq: u64, header: &[u8], body: &[u8]) -> Vec<u8> {
        self.0
            .encrypt(&nonce(seq), Payload { msg: body, aad: header })
            .expect("AES-GCM 잠그기는 길이 제한 안에서 실패하지 않음")
    }
}

fn nonce(seq: u64) -> Nonce<aes_gcm::aead::consts::U12> {
    let mut n = [0u8; 12];
    n[4..].copy_from_slice(&seq.to_le_bytes());
    n.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // docs/PROTOCOL.md: 시험 값
    #[test]
    fn matches_fixed_value() {
        let key: Key = hex("90de82fa19fcc35801587b8582a6fb07730eefba845edac03b027ff24a417245").try_into().unwrap();
        let sealed = Sealer::new(&key).seal(70000, &hex("02017011"), &[5; 10]);
        assert_eq!(sealed, hex("38a121539dec3e5259d7bb98e1229d66d650a06dc5afd1cbd8d4"));
        assert_eq!(sealed.len(), 10 + TAG_LEN);
    }

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
}
