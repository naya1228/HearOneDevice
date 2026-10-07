// BLE 송신 (Linux/BlueZ). PC가 광고(peripheral)하고 폰 앱이 찾아와 구독(central)한다.
// 인코딩된 데이터(encoding.rs)를 받아, 구독한 폰마다 열쇠 확인을 거친 뒤 notify로 패킷을 계속 밀어 보낸다.

use crate::auth::{Handshake, Key, REPLY_TIMEOUT};
use crate::codec::packet::{
    Packetizer, CONTROL_AUTH_FAIL, CONTROL_AUTH_OK, CONTROL_CHALLENGE, CONTROL_STOP,
};
use crate::device_id::{device_id_hex, DeviceId};
use crate::encoding::EncodedTx;
use bluer::{
    adv::{Advertisement, AdvertisementHandle},
    gatt::local::{
        characteristic_control, Application, ApplicationHandle, Characteristic,
        CharacteristicControlEvent, CharacteristicNotify, CharacteristicNotifyMethod,
        CharacteristicWrite, CharacteristicWriteMethod, Service,
    },
    gatt::CharacteristicWriter,
    Address, Uuid,
};
use futures_util::{FutureExt, StreamExt};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinSet;

// UUID·광고 형식은 docs/PROTOCOL.md
pub const SERVICE_UUID: Uuid = Uuid::from_u128(0x5e7a0001_3c1b_4f6e_9d2a_7b1c0e5a9f10);
pub const AUDIO_CHAR_UUID: Uuid = Uuid::from_u128(0x5e7a0002_3c1b_4f6e_9d2a_7b1c0e5a9f10);

/// 폰마다(블루투스 주소) 그 폰이 쓴 바이트를 받는 청취자 쪽 통로
type Inboxes = Arc<Mutex<HashMap<Address, mpsc::UnboundedSender<Vec<u8>>>>>;

/// drop되면 광고·GATT 등록이 해제되고 모든 전송이 멈춘다.
pub struct BleServer {
    _adv: AdvertisementHandle,
    _app: ApplicationHandle,
    accept_task: tokio::task::JoinHandle<()>,
    listeners: Arc<AtomicUsize>,
    stop_tx: watch::Sender<bool>,
}

impl BleServer {
    pub fn listeners(&self) -> usize {
        self.listeners.load(Ordering::Relaxed)
    }

    /// 구독 중인 폰마다 "공유 중지" 제어 패킷을 보내고 기다린다. 이후 drop하면 연결이 정리된다.
    /// (그냥 drop하면 폰은 연결이 끊긴 줄 알고 다시 찾는다)
    pub async fn stop(&self) {
        self.stop_tx.send_replace(true);
        // 모든 청취자가 중지 패킷을 쓰고 끝날 때까지 (최대 0.5초)
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(500);
        while self.listeners() > 0 && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        // 소켓에 쓴 패킷이 실제 전파로 나갈 시간
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

impl Drop for BleServer {
    fn drop(&mut self) {
        self.accept_task.abort();
    }
}

/// 이 PC의 블루투스 주소 6바이트. 폰이 QR로 받아 이 주소로 연결한다 (docs/PROTOCOL.md 1절)
pub async fn adapter_address() -> Result<[u8; 6], String> {
    let session = bluer::Session::new().await.map_err(|e| format!("BlueZ 연결 실패: {e}"))?;
    let adapter = session
        .default_adapter()
        .await
        .map_err(|e| format!("블루투스 어댑터 없음: {e}"))?;
    Ok(adapter.address().await.map_err(|e| e.to_string())?.0)
}

pub async fn start(encoded: EncodedTx, id: DeviceId, key: Key) -> Result<BleServer, String> {
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
            discoverable: Some(true),
            ..Default::default()
        })
        .await
        .map_err(|e| format!("BLE 광고 실패: {e}"))?;

    // 폰이 쓴 바이트(확인 답)는 그 폰의 청취자에게 넘긴다
    let inboxes: Inboxes = Arc::default();
    let routes = inboxes.clone();
    let on_write = move |value: Vec<u8>, req: bluer::gatt::local::CharacteristicWriteRequest| {
        if let Some(inbox) = routes.lock().unwrap().get(&req.device_address) {
            let _ = inbox.send(value);
        }
        async { Ok(()) }.boxed()
    };

    let (char_control, char_handle) = characteristic_control();
    let app = adapter
        .serve_gatt_application(Application {
            services: vec![Service {
                uuid: SERVICE_UUID,
                primary: true,
                characteristics: vec![Characteristic {
                    uuid: AUDIO_CHAR_UUID,
                    // Notify = 소리·제어 패킷, Write = 폰의 확인 답
                    write: Some(CharacteristicWrite {
                        write: true,
                        method: CharacteristicWriteMethod::Fun(Box::new(on_write)),
                        ..Default::default()
                    }),
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
    let (stop_tx, stop_rx) = watch::channel(false);
    let accept_task = tokio::spawn(async move {
        // 이 태스크가 abort되면 JoinSet이 drop되며 청취자 태스크도 전부 abort됨
        let mut set = JoinSet::new();
        let mut char_control = Box::pin(char_control);
        while let Some(evt) = char_control.next().await {
            if let CharacteristicControlEvent::Notify(writer) = evt {
                let addr = writer.device_address();
                println!("[ble] 구독 시작: {addr} (MTU {})", writer.mtu());
                let (inbox_tx, inbox_rx) = mpsc::unbounded_channel();
                inboxes.lock().unwrap().insert(addr, inbox_tx);
                set.spawn(listener_loop(
                    writer,
                    key,
                    inbox_rx,
                    inboxes.clone(),
                    encoded.clone(),
                    stop_rx.clone(),
                    count.clone(),
                ));
            }
        }
    });

    println!("[ble] 광고 시작: {}", device_id_hex(&id));
    Ok(BleServer { _adv: adv, _app: app, accept_task, listeners, stop_tx })
}

async fn listener_loop(
    mut writer: CharacteristicWriter,
    key: Key,
    inbox: mpsc::UnboundedReceiver<Vec<u8>>,
    inboxes: Inboxes,
    encoded: EncodedTx,
    stop: watch::Receiver<bool>,
    count: Arc<AtomicUsize>,
) {
    let addr = writer.device_address();
    // ATT 헤더 3바이트 제외, 속성 최대 길이 512
    let mut packetizer = Packetizer::new(writer.mtu().saturating_sub(3).clamp(20, 512));
    if authenticate(&mut writer, &mut packetizer, key, inbox).await {
        count.fetch_add(1, Ordering::Relaxed);
        send_audio(&mut writer, &mut packetizer, encoded, stop).await;
        count.fetch_sub(1, Ordering::Relaxed);
    }
    inboxes.lock().unwrap().remove(&addr);
}

/// 열쇠 확인: 문제 → 폰의 답 → PC 증명 (docs/PROTOCOL.md 5절). 통과하면 true
async fn authenticate(
    writer: &mut CharacteristicWriter,
    packetizer: &mut Packetizer,
    key: Key,
    mut inbox: mpsc::UnboundedReceiver<Vec<u8>>,
) -> bool {
    let addr = writer.device_address();
    let hs = Handshake::new(key);
    if let Err(e) = writer.write_all(&packetizer.control_with(CONTROL_CHALLENGE, hs.challenge())).await {
        println!("[ble] {addr} 연결 끊김: {e}");
        return false;
    }
    let pc_proof = match tokio::time::timeout(REPLY_TIMEOUT, inbox.recv()).await {
        Ok(Some(reply)) => hs.verify(&reply),
        _ => None,
    };
    let Some(pc_proof) = pc_proof else {
        let _ = writer.write_all(&packetizer.control(CONTROL_AUTH_FAIL)).await;
        println!("[ble] {addr} 확인 실패 (열쇠가 다르거나 답이 없음). 소리를 보내지 않음");
        return false;
    };
    if let Err(e) = writer.write_all(&packetizer.control_with(CONTROL_AUTH_OK, &pc_proof)).await {
        println!("[ble] {addr} 연결 끊김: {e}");
        return false;
    }
    println!("[ble] {addr} 확인 통과");
    true
}

async fn send_audio(
    writer: &mut CharacteristicWriter,
    packetizer: &mut Packetizer,
    encoded: EncodedTx,
    mut stop: watch::Receiver<bool>,
) {
    let addr = writer.device_address();
    let mut rx = encoded.subscribe();
    loop {
        let received = tokio::select! {
            r = rx.recv() => r,
            // stop 값은 false → true 로 한 번만 바뀐다
            _ = stop.changed() => {
                let _ = writer.write_all(&packetizer.control(CONTROL_STOP)).await;
                println!("[ble] {addr} 공유 중지 알림");
                break;
            }
        };
        let enc = match received {
            Ok(u) => u,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                println!("[ble] {addr} 전송 밀림, 조각 {n}개 건너뜀");
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };
        let Some(packet) = packetizer.packet_for(enc.codec, &enc.data) else { continue };
        if let Err(e) = writer.write_all(&packet).await {
            println!("[ble] {addr} 연결 끊김: {e}");
            break;
        }
    }
}
