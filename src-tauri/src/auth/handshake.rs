// 연결 한 번의 확인 절차 (PC 쪽 계산). 문제를 내고, 폰의 답을 검사하고, PC도 열쇠가 있다는 증명을 만든다.
// 메시지 순서·형식은 docs/PROTOCOL.md 5절

use super::key::Key;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// 문제를 낸 뒤 폰의 답을 기다리는 시간
pub const REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

const NONCE_LEN: usize = 16;
const PROOF_LEN: usize = 32;
const REPLY_VERSION: u8 = 1;
const REPLY_TYPE: u8 = 1;
/// 폰의 답 길이: 버전 + 종류 + 폰 무작위 값 + 폰 증명
pub const REPLY_LEN: usize = 2 + NONCE_LEN + PROOF_LEN;

const PHONE_LABEL: &[u8] = b"hearone-phone";
const PC_LABEL: &[u8] = b"hearone-pc";

pub struct Handshake {
    key: Key,
    pc_nonce: [u8; NONCE_LEN],
}

impl Handshake {
    /// 이번 연결에 쓸 문제(무작위 값)를 새로 만든다
    pub fn new(key: Key) -> Self {
        let mut pc_nonce = [0; NONCE_LEN];
        getrandom::fill(&mut pc_nonce).expect("무작위 값 만들기 실패");
        Self { key, pc_nonce }
    }

    /// 폰에 보낼 문제
    pub fn challenge(&self) -> &[u8] {
        &self.pc_nonce
    }

    /// 폰의 답을 검사한다. 맞으면 폰에 돌려줄 PC 쪽 증명, 틀리면 None
    pub fn verify(&self, reply: &[u8]) -> Option<[u8; PROOF_LEN]> {
        if reply.len() != REPLY_LEN || reply[0] != REPLY_VERSION || reply[1] != REPLY_TYPE {
            return None;
        }
        let phone_nonce = &reply[2..2 + NONCE_LEN];
        let phone_proof = &reply[2 + NONCE_LEN..];
        // 비교에 걸리는 시간으로 답이 새지 않게 verify_slice로 비교
        proof(&self.key, PHONE_LABEL, &self.pc_nonce, phone_nonce).verify_slice(phone_proof).ok()?;
        Some(proof(&self.key, PC_LABEL, &self.pc_nonce, phone_nonce).finalize().into_bytes().into())
    }
}

/// 증명 = HMAC-SHA256(열쇠, 이름표 + PC 무작위 값 + 폰 무작위 값)
fn proof(key: &Key, label: &[u8], pc_nonce: &[u8], phone_nonce: &[u8]) -> Hmac<Sha256> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC은 어떤 길이의 열쇠도 받음");
    mac.update(label);
    mac.update(pc_nonce);
    mac.update(phone_nonce);
    mac
}

#[cfg(test)]
mod tests {
    use super::*;

    // 폰이 하는 계산을 흉내 낸다 (앱 쪽 구현과 같아야 함)
    fn phone_reply(key: &Key, challenge: &[u8], phone_nonce: &[u8; NONCE_LEN]) -> Vec<u8> {
        let mut reply = vec![REPLY_VERSION, REPLY_TYPE];
        reply.extend_from_slice(phone_nonce);
        reply.extend_from_slice(&proof(key, PHONE_LABEL, challenge, phone_nonce).finalize().into_bytes());
        reply
    }

    #[test]
    fn right_key_passes_and_pc_proof_checks_out() {
        let key = [7u8; 32];
        let hs = Handshake::new(key);
        let phone_nonce = [9u8; NONCE_LEN];
        let pc_proof = hs.verify(&phone_reply(&key, hs.challenge(), &phone_nonce)).unwrap();
        // 폰은 같은 계산으로 PC 증명을 검사한다
        let expected = proof(&key, PC_LABEL, hs.challenge(), &phone_nonce);
        assert!(expected.verify_slice(&pc_proof).is_ok());
    }

    // 폰 앱 AuthTest.kt와 같은 값 (양쪽 계산이 같은지 확인)
    #[test]
    fn proof_matches_fixed_values() {
        let key = [7u8; 32];
        let pc_nonce: Vec<u8> = (0..16).collect();
        let phone_nonce = [9u8; NONCE_LEN];
        let hex = |m: Hmac<Sha256>| m.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(
            hex(proof(&key, PHONE_LABEL, &pc_nonce, &phone_nonce)),
            "9beffad1415a5c8a6753ab385c6705c48e7e55360551650d2aee0ced3c03ae31"
        );
        assert_eq!(
            hex(proof(&key, PC_LABEL, &pc_nonce, &phone_nonce)),
            "2bf2e7dbe786aad5e2980fe4816e0ba6b991034e44dbbf698966c29b5c7ac527"
        );
    }

    #[test]
    fn wrong_key_fails() {
        let hs = Handshake::new([7u8; 32]);
        assert_eq!(hs.verify(&phone_reply(&[8u8; 32], hs.challenge(), &[9u8; NONCE_LEN])), None);
    }

    #[test]
    fn reply_to_old_challenge_fails() {
        let key = [7u8; 32];
        let old = Handshake::new(key);
        let reply = phone_reply(&key, old.challenge(), &[9u8; NONCE_LEN]);
        assert_eq!(Handshake::new(key).verify(&reply), None);
    }

    #[test]
    fn malformed_reply_fails() {
        let key = [7u8; 32];
        let hs = Handshake::new(key);
        let mut reply = phone_reply(&key, hs.challenge(), &[9u8; NONCE_LEN]);
        assert_eq!(hs.verify(&reply[..REPLY_LEN - 1]), None);
        reply[0] = 2;
        assert_eq!(hs.verify(&reply), None);
    }
}
