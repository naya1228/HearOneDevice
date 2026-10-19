// docs/PROTOCOL.md: 찾기
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
