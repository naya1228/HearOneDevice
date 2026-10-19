// docs/PROTOCOL.md: 찾기
pub type DeviceId = [u8; 4];

pub fn device_id_hex(id: &DeviceId) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

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
