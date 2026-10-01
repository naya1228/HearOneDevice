// Windows용 BLE 송신은 아직 없음 (WinRT GattServiceProvider로 구현 예정)
use crate::audio::AudioTx;

pub type DeviceId = [u8; 4];

pub fn device_id_hex(id: &DeviceId) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

pub struct BleServer;

impl BleServer {
    pub fn listeners(&self) -> usize {
        0
    }
}

pub async fn start(_audio: AudioTx, _id: DeviceId) -> Result<BleServer, String> {
    Err("블루투스 송신은 아직 Linux만 지원합니다.".into())
}
