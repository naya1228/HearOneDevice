// BLE 송신 (Linux/BlueZ). PC가 광고(peripheral)하고 폰 앱이 찾아와 구독(central)한다.
// 인코딩된 데이터(encoding.rs)를 받아, 구독한 폰마다 notify로 패킷을 계속 밀어 보낸다.

use crate::codec::packet::Packetizer;
use crate::encoding::{Encoded, EncodedTx};
use bluer::{
    adv::{Advertisement, AdvertisementHandle},
    gatt::local::{
        characteristic_control, Application, ApplicationHandle, Characteristic,
        CharacteristicControlEvent, CharacteristicNotify, CharacteristicNotifyMethod, Service,
    },
    gatt::CharacteristicWriter,
    Uuid,
};
use futures_util::StreamExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use tokio::task::JoinSet;

// android/.../Protocol.kt 의 UUID와 같아야 함
pub const SERVICE_UUID: Uuid = Uuid::from_u128(0x5e7a0001_3c1b_4f6e_9d2a_7b1c0e5a9f10);
pub const AUDIO_CHAR_UUID: Uuid = Uuid::from_u128(0x5e7a0002_3c1b_4f6e_9d2a_7b1c0e5a9f10);

// 광고에 PC 고유 번호를 실어 폰이 QR로 받은 번호와 맞춰 찾는다 (Protocol.kt 와 같아야 함)
// 광고 31바이트 제한: flags(3) + 128bit UUID(18) + 제조사 데이터(4+4) = 29 → 이름은 못 넣음
pub const MANUFACTURER_ID: u16 = 0xFFFF; // 테스트/미등록용 예약 ID

/// PC 고유 번호 4바이트. QR에는 16진수 8자리로 들어간다.
pub type DeviceId = [u8; 4];

pub fn device_id_hex(id: &DeviceId) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

/// drop되면 광고·GATT 등록이 해제되고 모든 전송이 멈춘다.
pub struct BleServer {
    _adv: AdvertisementHandle,
    _app: ApplicationHandle,
    accept_task: tokio::task::JoinHandle<()>,
    listeners: Arc<AtomicUsize>,
}

impl BleServer {
    pub fn listeners(&self) -> usize {
        self.listeners.load(Ordering::Relaxed)
    }
}

impl Drop for BleServer {
    fn drop(&mut self) {
        self.accept_task.abort();
    }
}

pub async fn start(encoded: EncodedTx, id: DeviceId) -> Result<BleServer, String> {
    let session = bluer::Session::new().await.map_err(|e| format!("BlueZ 연결 실패: {e}"))?;
    let adapter = session
        .default_adapter()
        .await
        .map_err(|e| format!("블루투스 어댑터 없음: {e}"))?;
    adapter.set_powered(true).await.map_err(|e| e.to_string())?;

    let adv = adapter
        .advertise(Advertisement {
            advertisement_type: bluer::adv::Type::Peripheral,
            service_uuids: [SERVICE_UUID].into_iter().collect(),
            manufacturer_data: [(MANUFACTURER_ID, id.to_vec())].into_iter().collect(),
            discoverable: Some(true),
            ..Default::default()
        })
        .await
        .map_err(|e| format!("BLE 광고 실패: {e}"))?;

    let (char_control, char_handle) = characteristic_control();
    let app = adapter
        .serve_gatt_application(Application {
            services: vec![Service {
                uuid: SERVICE_UUID,
                primary: true,
                characteristics: vec![Characteristic {
                    uuid: AUDIO_CHAR_UUID,
                    notify: Some(CharacteristicNotify {
                        notify: true,
                        method: CharacteristicNotifyMethod::Io,
                        ..Default::default()
                    }),
                    control_handle: char_handle,
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        })
        .await
        .map_err(|e| format!("GATT 서버 등록 실패: {e}"))?;

    let listeners = Arc::new(AtomicUsize::new(0));
    let count = listeners.clone();
    let accept_task = tokio::spawn(async move {
        // 이 태스크가 abort되면 JoinSet이 drop되며 청취자 태스크도 전부 abort됨
        let mut set = JoinSet::new();
        let mut char_control = Box::pin(char_control);
        while let Some(evt) = char_control.next().await {
            if let CharacteristicControlEvent::Notify(writer) = evt {
                println!("[ble] 구독 시작: {} (MTU {})", writer.device_address(), writer.mtu());
                set.spawn(listener_loop(writer, encoded.subscribe(), count.clone()));
            }
        }
    });

    println!("[ble] 광고 시작: {}", device_id_hex(&id));
    Ok(BleServer { _adv: adv, _app: app, accept_task, listeners })
}

async fn listener_loop(
    mut writer: CharacteristicWriter,
    mut rx: broadcast::Receiver<Arc<Encoded>>,
    count: Arc<AtomicUsize>,
) {
    count.fetch_add(1, Ordering::Relaxed);
    let addr = writer.device_address();
    // ATT 헤더 3바이트 제외, 속성 최대 길이 512
    let mut packetizer = Packetizer::new(writer.mtu().saturating_sub(3).clamp(20, 512));
    'outer: loop {
        let enc = match rx.recv().await {
            Ok(u) => u,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                println!("[ble] {addr} 전송 밀림, 조각 {n}개 건너뜀");
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };
        for packet in packetizer.packets(enc.codec, &enc.data) {
            if let Err(e) = writer.write_all(&packet).await {
                println!("[ble] {addr} 연결 끊김: {e}");
                break 'outer;
            }
        }
    }
    count.fetch_sub(1, Ordering::Relaxed);
}
