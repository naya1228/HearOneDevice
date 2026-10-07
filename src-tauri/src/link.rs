// 폰이 QR로 읽는 연결 링크 만들기. 형식은 docs/PROTOCOL.md 1절

use crate::device_id::{device_id_hex, DeviceId};

/// 이 PC의 이름 (QR에 실어 폰 목록에 표시). 컴퓨터 이름(hostname)
pub fn pc_name() -> String {
    gethostname::gethostname().to_string_lossy().into_owned()
}

/// hearone://connect?id=<번호>&name=<이름>
pub fn connect_link(id: &DeviceId, name: &str) -> String {
    format!("hearone://connect?id={}&name={}", device_id_hex(id), encode(name))
}

// URL 퍼센트 인코딩 (영문·숫자·-._~ 말고는 UTF-8 바이트마다 %XX)
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

    #[test]
    fn link_has_id_and_encoded_name() {
        let id = [0x00, 0xc0, 0xff, 0xee];
        assert_eq!(connect_link(&id, "Dell-Pro-14"), "hearone://connect?id=00c0ffee&name=Dell-Pro-14");
        assert_eq!(connect_link(&id, "내 PC"), "hearone://connect?id=00c0ffee&name=%EB%82%B4%20PC");
    }
}
