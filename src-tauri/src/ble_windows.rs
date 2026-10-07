// BLE 송신 (Windows/WinRT). PC가 광고(peripheral)하고 폰 앱이 찾아와 구독(central)한다.
// 인코딩된 데이터(encoding.rs)를 받아, 구독한 폰마다 notify로 패킷을 계속 밀어 보낸다.
// 바깥에서 보는 모양(start·listeners·stop)은 ble.rs(Linux)와 같다.

use crate::codec::packet::{Packetizer, CONTROL_STOP};
use crate::device_id::{device_id_hex, DeviceId};
use crate::encoding::{Encoded, EncodedTx};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch, Notify};
use tokio::task::{AbortHandle, JoinSet};
use windows::core::GUID;
use windows::Devices::Bluetooth::{BluetoothAdapter, BluetoothError};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristicProperties, GattCommunicationStatus, GattLocalCharacteristic,
    GattLocalCharacteristicParameters, GattServiceProvider, GattServiceProviderAdvertisementStatus,
    GattServiceProviderAdvertisingParameters, GattSubscribedClient,
};
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::{DataWriter, IBuffer};

// UUID·광고 형식은 docs/PROTOCOL.md
const SERVICE_UUID: GUID = GUID::from_u128(0x5e7a0001_3c1b_4f6e_9d2a_7b1c0e5a9f10);
const AUDIO_CHAR_UUID: GUID = GUID::from_u128(0x5e7a0002_3c1b_4f6e_9d2a_7b1c0e5a9f10);

/// 이 PC의 블루투스 주소 ("98:FE:3E:E1:05:27" 꼴). 폰이 연결할 주소 (docs/PROTOCOL.md)
pub async fn adapter_address() -> Result<String, String> {
    let adapter = BluetoothAdapter::GetDefaultAsync()
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| format!("블루투스 어댑터 없음: {e}"))?;
    let addr = adapter.BluetoothAddress().map_err(|e| e.to_string())?;
    let bytes = &addr.to_be_bytes()[2..];
    Ok(bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":"))
}

/// drop되면 광고·GATT 등록이 해제되고 모든 전송이 멈춘다.
pub struct BleServer {
    provider: GattServiceProvider,
    characteristic: GattLocalCharacteristic,
    clients_token: i64,
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
        // 모든 청취자가 중지 패킷을 보내고 끝날 때까지 (최대 0.5초)
        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        while self.listeners() > 0 && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        // 보낸 패킷이 실제 전파로 나갈 시간
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

impl Drop for BleServer {
    fn drop(&mut self) {
        self.accept_task.abort();
        let _ = self.characteristic.RemoveSubscribedClientsChanged(self.clients_token);
        let _ = self.provider.StopAdvertising();
    }
}

pub async fn start(encoded: EncodedTx, id: DeviceId) -> Result<BleServer, String> {
    let created = GattServiceProvider::CreateAsync(SERVICE_UUID)
        .map_err(|e| format!("GATT 서비스 만들기 실패: {e}"))?
        .await
        .map_err(|e| format!("GATT 서비스 만들기 실패: {e}"))?;
    check(created.Error(), "GATT 서비스 만들기 실패")?;
    let provider = created.ServiceProvider().map_err(|e| e.to_string())?;

    let params = GattLocalCharacteristicParameters::new().map_err(|e| e.to_string())?;
    params
        .SetCharacteristicProperties(GattCharacteristicProperties::Notify)
        .map_err(|e| e.to_string())?;
    let made = provider
        .Service()
        .and_then(|s| s.CreateCharacteristicAsync(AUDIO_CHAR_UUID, &params))
        .map_err(|e| format!("오디오 특성 만들기 실패: {e}"))?
        .await
        .map_err(|e| format!("오디오 특성 만들기 실패: {e}"))?;
    check(made.Error(), "오디오 특성 만들기 실패")?;
    let characteristic = made.Characteristic().map_err(|e| e.to_string())?;

    // 구독자 목록이 바뀌면 (WinRT 스레드에서) 깨워서 아래 accept 태스크가 목록을 다시 본다
    let changed = Arc::new(Notify::new());
    let wake = changed.clone();
    let clients_token = characteristic
        .SubscribedClientsChanged(&TypedEventHandler::new(move |_, _| {
            wake.notify_one();
            Ok(())
        }))
        .map_err(|e| e.to_string())?;

    if let Err(e) = advertise(&provider).await {
        let _ = characteristic.RemoveSubscribedClientsChanged(clients_token);
        return Err(e);
    }

    let listeners = Arc::new(AtomicUsize::new(0));
    let count = listeners.clone();
    let (stop_tx, stop_rx) = watch::channel(false);
    let ch = characteristic.clone();
    let accept_task = tokio::spawn(async move {
        // 이 태스크가 abort되면 JoinSet이 drop되며 청취자 태스크도 전부 abort됨
        let mut set = JoinSet::new();
        let mut active: HashMap<String, AbortHandle> = HashMap::new();
        loop {
            while set.try_join_next().is_some() {}
            let now = subscribed_clients(&ch);
            // 구독을 끊었거나 전송이 끝난 폰은 정리
            active.retain(|key, task| {
                let keep = now.contains_key(key) && !task.is_finished();
                if !keep {
                    task.abort();
                }
                keep
            });
            for (key, client) in now {
                if active.contains_key(&key) {
                    continue;
                }
                let max = client.MaxNotificationSize().unwrap_or(20);
                println!("[ble] 구독 시작: {key} (최대 패킷 {max}B)");
                let task = set.spawn(listener_loop(
                    ch.clone(),
                    client,
                    key.clone(),
                    encoded.subscribe(),
                    stop_rx.clone(),
                    count.clone(),
                ));
                active.insert(key, task);
            }
            changed.notified().await;
        }
    });

    println!("[ble] 광고 시작: {}", device_id_hex(&id));
    Ok(BleServer { provider, characteristic, clients_token, accept_task, listeners, stop_tx })
}

/// 연결 가능한 광고. 내용(서비스 UUID)은 Windows가 채우고, 주소는 이 PC의 진짜 주소로 나간다
async fn advertise(provider: &GattServiceProvider) -> Result<(), String> {
    let adv = GattServiceProviderAdvertisingParameters::new().map_err(|e| e.to_string())?;
    adv.SetIsConnectable(true).map_err(|e| e.to_string())?;
    adv.SetIsDiscoverable(true).map_err(|e| e.to_string())?;
    provider.StartAdvertisingWithParameters(&adv).map_err(|e| format!("BLE 광고 실패: {e}"))?;

    // 광고 시작 결과는 나중에 상태로만 알 수 있어서 잠깐 지켜본다.
    // 시작 직후 잠깐 Aborted로 보였다가 Started로 바뀌기도 해서 끝까지 기다린다
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let status = provider.AdvertisementStatus().map_err(|e| e.to_string())?;
        if status == GattServiceProviderAdvertisementStatus::Started {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            let _ = provider.StopAdvertising();
            return Err(format!(
                "BLE 광고가 시작되지 않았습니다. 블루투스가 켜져 있는지, HearOne이 이미 켜져 있지 않은지 확인하세요 (상태 {})",
                status.0
            ));
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// 지금 구독 중인 폰들. 열쇠는 폰마다 다른 장치 ID 문자열
fn subscribed_clients(ch: &GattLocalCharacteristic) -> HashMap<String, GattSubscribedClient> {
    let Ok(clients) = ch.SubscribedClients() else { return HashMap::new() };
    clients
        .into_iter()
        .filter_map(|c| {
            let key = c.Session().and_then(|s| s.DeviceId()).and_then(|d| d.Id()).ok()?;
            Some((key.to_string(), c))
        })
        .collect()
}

async fn listener_loop(
    ch: GattLocalCharacteristic,
    client: GattSubscribedClient,
    name: String,
    mut rx: broadcast::Receiver<Arc<Encoded>>,
    mut stop: watch::Receiver<bool>,
    count: Arc<AtomicUsize>,
) {
    count.fetch_add(1, Ordering::Relaxed);
    // MaxNotificationSize = MTU − 3 (ATT 헤더). 속성 최대 길이 512
    let max = client.MaxNotificationSize().unwrap_or(20) as usize;
    let mut packetizer = Packetizer::new(max.clamp(20, 512));
    loop {
        let received = tokio::select! {
            r = rx.recv() => r,
            // stop 값은 false → true 로 한 번만 바뀐다
            _ = stop.changed() => {
                let _ = notify(&ch, &client, &packetizer.control(CONTROL_STOP)).await;
                println!("[ble] {name} 공유 중지 알림");
                break;
            }
        };
        let enc = match received {
            Ok(u) => u,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                println!("[ble] {name} 전송 밀림, 조각 {n}개 건너뜀");
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };
        let Some(packet) = packetizer.packet_for(enc.codec, &enc.data) else { continue };
        if let Err(e) = notify(&ch, &client, &packet).await {
            println!("[ble] {name} 연결 끊김: {e}");
            break;
        }
    }
    count.fetch_sub(1, Ordering::Relaxed);
}

async fn notify(ch: &GattLocalCharacteristic, client: &GattSubscribedClient, data: &[u8]) -> Result<(), String> {
    // IBuffer는 Send가 아니라서 await 전에 버린다
    let op = buffer(data)
        .and_then(|buf| ch.NotifyValueForSubscribedClientAsync(&buf, client))
        .map_err(|e| e.to_string())?;
    let result = op.await.map_err(|e| e.to_string())?;
    match result.Status().map_err(|e| e.to_string())? {
        GattCommunicationStatus::Success => Ok(()),
        s => Err(format!("전송 실패 (상태 {})", s.0)),
    }
}

fn buffer(data: &[u8]) -> windows::core::Result<IBuffer> {
    let w = DataWriter::new()?;
    w.WriteBytes(data)?;
    w.DetachBuffer()
}

fn check(error: windows::core::Result<BluetoothError>, what: &str) -> Result<(), String> {
    match error.map_err(|e| format!("{what}: {e}"))? {
        BluetoothError::Success => Ok(()),
        BluetoothError::RadioNotAvailable => Err("블루투스가 꺼져 있거나 어댑터가 없습니다".into()),
        BluetoothError::NotSupported => Err("이 블루투스 어댑터는 광고(peripheral)를 지원하지 않습니다".into()),
        e => Err(format!("{what} (블루투스 오류 {})", e.0)),
    }
}
