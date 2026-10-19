// docs/PROTOCOL.md: 연결 확인, 잠그기
use super::key::Key;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

const NONCE_LEN: usize = 16;
const PROOF_LEN: usize = 32;
const REPLY_VERSION: u8 = 1;
const REPLY_TYPE: u8 = 1;
pub const REPLY_LEN: usize = 2 + NONCE_LEN + PROOF_LEN;

const PHONE_LABEL: &[u8] = b"hearone-phone";
const PC_LABEL: &[u8] = b"hearone-pc";
const AUDIO_LABEL: &[u8] = b"hearone-audio";

pub struct Passed {
    pub pc_proof: [u8; PROOF_LEN],
    pub audio_key: Key,
}

pub struct Handshake {
    key: Key,
    pc_nonce: [u8; NONCE_LEN],
}

impl Handshake {
    pub fn new(key: Key) -> Self {
        let mut pc_nonce = [0; NONCE_LEN];
        getrandom::fill(&mut pc_nonce).expect("무작위 값 만들기 실패");
        Self { key, pc_nonce }
    }

    pub fn challenge(&self) -> &[u8] {
        &self.pc_nonce
    }

    pub fn verify(&self, reply: &[u8]) -> Option<Passed> {
        if reply.len() != REPLY_LEN || reply[0] != REPLY_VERSION || reply[1] != REPLY_TYPE {
            return None;
        }
        let phone_nonce = &reply[2..2 + NONCE_LEN];
        let phone_proof = &reply[2 + NONCE_LEN..];
        proof(&self.key, PHONE_LABEL, &self.pc_nonce, phone_nonce).verify_slice(phone_proof).ok()?;
        let make = |label| proof(&self.key, label, &self.pc_nonce, phone_nonce).finalize().into_bytes().into();
        Some(Passed { pc_proof: make(PC_LABEL), audio_key: make(AUDIO_LABEL) })
    }
}

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
        let passed = hs.verify(&phone_reply(&key, hs.challenge(), &phone_nonce)).unwrap();
        let expected = proof(&key, PC_LABEL, hs.challenge(), &phone_nonce);
        assert!(expected.verify_slice(&passed.pc_proof).is_ok());
        let audio = proof(&key, AUDIO_LABEL, hs.challenge(), &phone_nonce);
        assert!(audio.verify_slice(&passed.audio_key).is_ok());
    }

    // docs/PROTOCOL.md: 시험 값
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
        assert_eq!(
            hex(proof(&key, AUDIO_LABEL, &pc_nonce, &phone_nonce)),
            "90de82fa19fcc35801587b8582a6fb07730eefba845edac03b027ff24a417245"
        );
    }

    #[test]
    fn wrong_key_fails() {
        let hs = Handshake::new([7u8; 32]);
        assert!(hs.verify(&phone_reply(&[8u8; 32], hs.challenge(), &[9u8; NONCE_LEN])).is_none());
    }

    #[test]
    fn reply_to_old_challenge_fails() {
        let key = [7u8; 32];
        let old = Handshake::new(key);
        let reply = phone_reply(&key, old.challenge(), &[9u8; NONCE_LEN]);
        assert!(Handshake::new(key).verify(&reply).is_none());
    }

    #[test]
    fn malformed_reply_fails() {
        let key = [7u8; 32];
        let hs = Handshake::new(key);
        let mut reply = phone_reply(&key, hs.challenge(), &[9u8; NONCE_LEN]);
        assert!(hs.verify(&reply[..REPLY_LEN - 1]).is_none());
        reply[0] = 2;
        assert!(hs.verify(&reply).is_none());
    }
}
