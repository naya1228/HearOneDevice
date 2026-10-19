// docs/PROTOCOL.md: 찾기
use crate::auth::{key_hex, Key};
use crate::device_id::{device_id_hex, DeviceId};

pub fn pc_name() -> String {
    gethostname::gethostname().to_string_lossy().into_owned()
}

pub fn connect_link(id: &DeviceId, key: &Key, name: &str) -> String {
    format!("hearone://connect?id={}&key={}&name={}", device_id_hex(id), key_hex(key), encode(name))
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // docs/PROTOCOL.md: 시험 값
    #[test]
    fn link_has_id_key_and_encoded_name() {
        let id = [0x00, 0xc0, 0xff, 0xee];
        let key = [0xab; 32];
        let key_hex = "ab".repeat(32);
        assert_eq!(
            connect_link(&id, &key, "Dell-Pro-14"),
            format!("hearone://connect?id=00c0ffee&key={key_hex}&name=Dell-Pro-14")
        );
        assert_eq!(
            connect_link(&id, &key, "내 PC"),
            format!("hearone://connect?id=00c0ffee&key={key_hex}&name=%EB%82%B4%20PC")
        );
    }
}
