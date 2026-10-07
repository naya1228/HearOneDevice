// PC 고유 번호: 광고·QR에 실어 폰이 이 PC를 알아보게 한다. 형식은 docs/PROTOCOL.md 1절

/// PC 고유 번호 4바이트. QR에는 16진수 8자리로 들어간다.
pub type DeviceId = [u8; 4];

pub fn device_id_hex(id: &DeviceId) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

/// 처음 한 번 만들어 설정 폴더에 저장 → 앱을 다시 켜도 QR이 그대로
pub fn load_device_id(dir: std::path::PathBuf) -> DeviceId {
    use std::hash::{BuildHasher, Hasher};
    let path = dir.join("device_id");
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(id) = DeviceId::try_from(bytes.as_slice()) {
            return id;
        }
    }
    let random = std::collections::hash_map::RandomState::new().build_hasher().finish();
    let id: DeviceId = (random as u32).to_be_bytes();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(&path, id);
    id
}
