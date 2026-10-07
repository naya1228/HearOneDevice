// BLE 송신 (Windows/WinRT). PC가 광고(peripheral)하고 폰 앱이 찾아와 구독(central)한다.
// 구독한 폰마다 먼저 열쇠를 확인하고(auth), 통과한 폰에만 인코딩된 데이터(encoding.rs)를 notify로 밀어 보낸다.
// 바깥에서 보는 모양(adapter_address·start·listeners·stop)은 ble.rs(Linux)와 같다.

use crate::auth::{Handshake, Key, REPLY_TIMEOUT};
use crate::codec::packet::{
    Packetizer, CONTROL_AUTH_FAIL, CONTROL_AUTH_OK, CONTROL_CHALLENGE, CONTROL_STOP,
};
use crate::device_id::{device_id_hex, DeviceId};
use crate::encoding::EncodedTx;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch, Notify};
use tokio::task::{AbortHandle, JoinSet};
use windows::core::{Ref, GUID};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristicProperties, GattCommunicationStatus, GattLocalCharacteristic,
    GattLocalCharacteristicParameters, GattServiceProvider, GattServiceProviderAdvertisementStatus,
    GattServiceProviderAdvertisingParameters, GattSubscribedClient, GattWriteOption,
    GattWriteRequestedEventArgs,
};
use windows::Devices::Bluetooth::{BluetoothAdapter, BluetoothError};
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::{DataReader, DataWriter, IBuffer};

// UUID·광고 형식은 docs/PROTOCOL.md
const SERVICE_UUID: GUID = GUID::from_u128(0x5e7a0001_3c1b_4f6e_9d2a_7b1c0e5a9f10);
const AUDIO_CHAR_UUID: GUID = GUID::from_u128(0x5e7a0002_3c1b_4f6e_9d2a_7b1c0e5a9f10);

/// 폰마다(장치 ID 문자열) 그 폰이 쓴 바이트를 받는 청취자 쪽 통로
type Inboxes = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<Vec<u8>>>>>;

/// 이 PC의 블루투스 주소 6바이트. 폰이 QR로 받아 이 주소로 연결한다 (docs/PROTOCOL.md 1절)
pub async fn adapter_address() -> Result<[u8; 6], String> {
    let adapter = BluetoothAdapter::GetDefaultAsync()
        .map_err(|e| e.to_string())?
        .await
        .map_err(|e| format!("블루투스 어댑터 없음: {e}"))?;
    let addr = adapter.BluetoothAddress().map_err(|e| e.to_string())?;
    Ok(addr.to_be_bytes()[2..].try_into().expect("u64의 뒤 6바이트"))
}

/// drop되면 광고·GATT 등록이 해제되고 모든 전송이 멈춘다.
pub struct BleServer {
    provider: GattServiceProvider,
    characteristic: GattLocalCharacteristic,
    clients_token: i64,
    write_token: i64,
    accept_task: tokio::task::JoinHandle<()>,
    listeners: Arc<AtomicUsize>,
    stop_tx: watch::Sender<bool>,
}

impl BleServer {
    /// 확인을 통과해 소리를 받는 폰 수
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
        let _ = self.characteristic.RemoveWriteRequested(self.write_token);
        let _ = self.provider.StopAdvertising();
    }
}

pub async fn start(encoded: EncodedTx, id: DeviceId, key: Key) -> Result<BleServer, String> {
    let created = GattServiceProvider::CreateAsync(SERVICE_UUID)
        .map_err(|e| format!("GATT 서비스 만들기 실패: {e}"))?
        .await
        .map_err(|e| format!("GATT 서비스 만들기 실패: {e}"))?;
    check(created.Error(), "GATT 서비스 만들기 실패")?;
    let provider = created.ServiceProvider().map_err(|e| e.to_string())?;

    // Notify = 소리·제어 패킷, Write = 폰의 확인 답
    let params = GattLocalCharacteristicParameters::new().map_err(|e| e.to_string())?;
    params
        .SetCharacteristicProperties(GattCharacteristicProperties::Notify | GattCharacteristicProperties::Write)
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

    // 폰이 쓴 바이트는 그 폰의 청취자에게 넘긴다
    let inboxes: Inboxes = Arc::default();
    let routes = inboxes.clone();
    let write_token = characteristic
        .WriteRequested(&TypedEventHandler::new(
            move |_, args: Ref<'_, GattWriteRequestedEventArgs>| on_write(args.ok()?, &routes),
        ))
        .map_err(|e| e.to_string())?;

    if let Err(e) = advertise(&provider).await {
        let _ = characteristic.RemoveSubscribedClientsChanged(clients_token);
        let _ = characteristic.RemoveWriteRequested(write_token);
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
            active.retain(|name, task| {
                let keep = now.contains_key(name) && !task.is_finished();
                if !keep {
                    task.abort();
                    inboxes.lock().unwrap().remove(name);
                }
                keep
            });
            for (name, client) in now {
                if active.contains_key(&name) {
                    continue;
                }
                let max = client.MaxNotificationSize().unwrap_or(20);
                println!("[ble] 구독 시작: {name} (최대 패킷 {max}B)");
                let (inbox_tx, inbox_rx) = mpsc::unbounded_channel();
                inboxes.lock().unwrap().insert(name.clone(), inbox_tx);
                let task = set.spawn(listener_loop(
                    Listener { ch: ch.clone(), client, name: name.clone() },
                    key,
                    inbox_rx,
                    encoded.clone(),
                    stop_rx.clone(),
                    count.clone(),
                ));
                active.insert(name, task);
            }
            changed.notified().await;
        }
    });

    println!("[ble] 광고 시작: {}", device_id_hex(&id));
    Ok(BleServer { provider, characteristic, clients_token, write_token, accept_task, listeners, stop_tx })
}

/// 쓰기 요청 하나 처리 (WinRT 스레드). 응답하고, 바이트를 그 폰의 청취자에게 넘긴다
fn on_write(args: &GattWriteRequestedEventArgs, routes: &Inboxes) -> windows::core::Result<()> {
    let deferral = args.GetDeferral()?;
    let result = (|| {
        let name = args.Session()?.DeviceId()?.Id()?.to_string();
        let request = args.GetRequestAsync()?.get()?;
        let value = read(&request.Value()?)?;
        if request.Option()? == GattWriteOption::WriteWithResponse {
            request.Respond()?;
        }
        if let Some(inbox) = routes.lock().unwrap().get(&name) {
            let _ = inbox.send(value);
        }
        Ok(())
    })();
    // 중간에 실패해도 끝났다고 알려야 Windows가 이 요청을 붙잡고 있지 않는다
    deferral.Complete()?;
    result
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
            let name = c.Session().and_then(|s| s.DeviceId()).and_then(|d| d.Id()).ok()?;
            Some((name.to_string(), c))
        })
        .collect()
}

/// 구독한 폰 하나에게 보내는 통로
struct Listener {
    ch: GattLocalCharacteristic,
    client: GattSubscribedClient,
    name: String,
}

impl Listener {
    async fn notify(&self, data: &[u8]) -> Result<(), String> {
        // IBuffer는 Send가 아니라서 await 전에 버린다
        let op = buffer(data)
            .and_then(|buf| self.ch.NotifyValueForSubscribedClientAsync(&buf, &self.client))
            .map_err(|e| e.to_string())?;
        let result = op.await.map_err(|e| e.to_string())?;
        match result.Status().map_err(|e| e.to_string())? {
            GattCommunicationStatus::Success => Ok(()),
            s => Err(format!("전송 실패 (상태 {})", s.0)),
        }
    }
}

async fn listener_loop(
    to: Listener,
    key: Key,
    mut inbox: mpsc::UnboundedReceiver<Vec<u8>>,
    encoded: EncodedTx,
    mut stop: watch::Receiver<bool>,
    count: Arc<AtomicUsize>,
) {
    let name = to.name.clone();
    // MaxNotificationSize = MTU − 3 (ATT 헤더). 속성 최대 길이 512
    let max = to.client.MaxNotificationSize().unwrap_or(20) as usize;
    let mut packetizer = Packetizer::new(max.clamp(20, 512));

    // 1. 열쇠 확인: 문제 → 폰의 답 → PC 증명 (docs/PROTOCOL.md 5절)
    let hs = Handshake::new(key);
    if let Err(e) = to.notify(&packetizer.control_with(CONTROL_CHALLENGE, hs.challenge())).await {
        println!("[ble] {name} 연결 끊김: {e}");
        return;
    }
    let pc_proof = match tokio::time::timeout(REPLY_TIMEOUT, inbox.recv()).await {
        Ok(Some(reply)) => hs.verify(&reply),
        _ => None,
    };
    let Some(pc_proof) = pc_proof else {
        let _ = to.notify(&packetizer.control(CONTROL_AUTH_FAIL)).await;
        println!("[ble] {name} 확인 실패 (열쇠가 다르거나 답이 없음). 소리를 보내지 않음");
        return;
    };
    if let Err(e) = to.notify(&packetizer.control_with(CONTROL_AUTH_OK, &pc_proof)).await {
        println!("[ble] {name} 연결 끊김: {e}");
        return;
    }
    println!("[ble] {name} 확인 통과");

    // 2. 소리 보내기
    count.fetch_add(1, Ordering::Relaxed);
    let mut rx = encoded.subscribe();
    loop {
        let received = tokio::select! {
            r = rx.recv() => r,
            // stop 값은 false → true 로 한 번만 바뀐다
            _ = stop.changed() => {
                let _ = to.notify(&packetizer.control(CONTROL_STOP)).await;
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
        if let Err(e) = to.notify(&packet).await {
            println!("[ble] {name} 연결 끊김: {e}");
            break;
        }
    }
    count.fetch_sub(1, Ordering::Relaxed);
}

fn buffer(data: &[u8]) -> windows::core::Result<IBuffer> {
    let w = DataWriter::new()?;
    w.WriteBytes(data)?;
    w.DetachBuffer()
}

fn read(buf: &IBuffer) -> windows::core::Result<Vec<u8>> {
    let mut data = vec![0; buf.Length()? as usize];
    DataReader::FromBuffer(buf)?.ReadBytes(&mut data)?;
    Ok(data)
}

fn check(error: windows::core::Result<BluetoothError>, what: &str) -> Result<(), String> {
    match error.map_err(|e| format!("{what}: {e}"))? {
        BluetoothError::Success => Ok(()),
        BluetoothError::RadioNotAvailable => Err("블루투스가 꺼져 있거나 어댑터가 없습니다".into()),
        BluetoothError::NotSupported => Err("이 블루투스 어댑터는 광고(peripheral)를 지원하지 않습니다".into()),
        e => Err(format!("{what} (블루투스 오류 {})", e.0)),
    }
}
