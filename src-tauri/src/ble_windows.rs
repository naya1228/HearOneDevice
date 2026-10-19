// docs/PROTOCOL.md: 찾기, 연결 (Windows)
use crate::auth::Key;
use crate::encoding::EncodedTx;
use crate::listener::{self, PhoneLink, MIN_PACKET};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, watch, Notify};
use tokio::task::{AbortHandle, JoinSet};
use windows::core::{Ref, GUID};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristicProperties, GattCommunicationStatus, GattLocalCharacteristic,
    GattLocalCharacteristicParameters, GattServiceProvider, GattServiceProviderAdvertisementStatus,
    GattServiceProviderAdvertisingParameters, GattSubscribedClient, GattWriteOption,
    GattWriteRequestedEventArgs,
};
use windows::Devices::Bluetooth::BluetoothError;
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::{DataReader, DataWriter, IBuffer};

const SERVICE_UUID: GUID = GUID::from_u128(0x5e7a0001_3c1b_4f6e_9d2a_7b1c0e5a9f10);
const AUDIO_CHAR_UUID: GUID = GUID::from_u128(0x5e7a0002_3c1b_4f6e_9d2a_7b1c0e5a9f10);
const ADVERTISE_WAIT: Duration = Duration::from_secs(2);
const ADVERTISE_POLL: Duration = Duration::from_millis(20);

type Inboxes = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<Vec<u8>>>>>;

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
    pub fn listeners(&self) -> usize {
        self.listeners.load(Ordering::Relaxed)
    }

    pub async fn stop(&self) {
        listener::stop_all(&self.stop_tx, &self.listeners).await;
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

pub async fn start(encoded: EncodedTx, key: Key) -> Result<BleServer, String> {
    let created = GattServiceProvider::CreateAsync(SERVICE_UUID)
        .map_err(|e| format!("GATT 서비스 만들기 실패: {e}"))?
        .await
        .map_err(|e| format!("GATT 서비스 만들기 실패: {e}"))?;
    check(created.Error(), "GATT 서비스 만들기 실패")?;
    let provider = created.ServiceProvider().map_err(|e| e.to_string())?;

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

    let changed = Arc::new(Notify::new());
    let wake = changed.clone();
    let clients_token = characteristic
        .SubscribedClientsChanged(&TypedEventHandler::new(move |_, _| {
            wake.notify_one();
            Ok(())
        }))
        .map_err(|e| e.to_string())?;

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
        let mut set = JoinSet::new();
        let mut active: HashMap<String, AbortHandle> = HashMap::new();
        loop {
            while set.try_join_next().is_some() {}
            let now = subscribed_clients(&ch);
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
                let max = client.MaxNotificationSize().map_or(MIN_PACKET, usize::from);
                println!("[ble] 구독 시작: {name} (최대 패킷 {max}B)");
                let (inbox_tx, inbox_rx) = mpsc::unbounded_channel();
                inboxes.lock().unwrap().insert(name.clone(), inbox_tx);
                let task = set.spawn(listener::serve(
                    Listener { ch: ch.clone(), client },
                    name.clone(),
                    max,
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

    println!("[ble] 광고 시작");
    Ok(BleServer { provider, characteristic, clients_token, write_token, accept_task, listeners, stop_tx })
}

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
    deferral.Complete()?;
    result
}

// docs/PROTOCOL.md: 찾기
async fn advertise(provider: &GattServiceProvider) -> Result<(), String> {
    let adv = GattServiceProviderAdvertisingParameters::new().map_err(|e| e.to_string())?;
    adv.SetIsConnectable(true).map_err(|e| e.to_string())?;
    adv.SetIsDiscoverable(true).map_err(|e| e.to_string())?;
    provider.StartAdvertisingWithParameters(&adv).map_err(|e| format!("BLE 광고 실패: {e}"))?;

    let deadline = tokio::time::Instant::now() + ADVERTISE_WAIT;
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
        tokio::time::sleep(ADVERTISE_POLL).await;
    }
}

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

struct Listener {
    ch: GattLocalCharacteristic,
    client: GattSubscribedClient,
}

impl PhoneLink for Listener {
    async fn send(&mut self, packet: &[u8]) -> Result<(), String> {
        let op = buffer(packet)
            .and_then(|buf| self.ch.NotifyValueForSubscribedClientAsync(&buf, &self.client))
            .map_err(|e| e.to_string())?;
        let result = op.await.map_err(|e| e.to_string())?;
        match result.Status().map_err(|e| e.to_string())? {
            GattCommunicationStatus::Success => Ok(()),
            s => Err(format!("전송 실패 (상태 {})", s.0)),
        }
    }
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
