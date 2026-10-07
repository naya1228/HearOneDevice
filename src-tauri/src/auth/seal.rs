// 패킷 잠그기 (PC 쪽 계산). AES-256-GCM, 한 번만 쓰는 번호는 긴 순번. 형식은 docs/PROTOCOL.md 6절

use super::key::Key;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};

/// 잠근 패킷 뒤에 붙는 검사값 길이
pub const TAG_LEN: usize = 16;

pub struct Sealer(Aes256Gcm);

impl Sealer {
    /// key = 이번 연결의 소리 열쇠 (handshake.rs의 Passed::audio_key)
    pub fn new(key: &Key) -> Self {
        Self(Aes256Gcm::new(key.into()))
    }

    /// header는 그대로 두되 검사에 넣고, body를 잠가 검사값을 붙인다.
    /// seq = 긴 순번. 같은 열쇠로 같은 seq를 두 번 쓰면 안 됨 (부르는 쪽이 지킴)
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

    // 폰 앱 UnsealerTest.kt와 같은 값 (Python cryptography로 따로 계산)
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
