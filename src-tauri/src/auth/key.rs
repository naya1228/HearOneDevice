// 확인용 열쇠: 처음 한 번 무작위로 만들어 설정 폴더에 저장 → 앱을 다시 켜도 QR이 그대로

/// 열쇠 32바이트. QR에는 16진수 64자리로 들어간다.
pub type Key = [u8; 32];

pub fn key_hex(key: &Key) -> String {
    key.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn load_key(dir: std::path::PathBuf) -> Key {
    let path = dir.join("secret");
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(key) = Key::try_from(bytes.as_slice()) {
            return key;
        }
    }
    let mut key: Key = [0; 32];
    getrandom::fill(&mut key).expect("무작위 값 만들기 실패");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(&path, key);
    key
}
