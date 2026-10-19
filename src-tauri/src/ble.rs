// docs/PROTOCOL.md: 찾기, 연결 (Linux)
use crate::auth::Key;
use crate::encoding::EncodedTx;
use crate::listener::{self, PhoneLink, ATT_HEADER_LEN};
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
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;

pub const SERVICE_UUID: Uuid = Uuid::from_u128(0x5e7a0001_3c1b_4f6e_9d2a_7b1c0e5a9f10);
pub const AUDIO_CHAR_UUID: Uuid = Uuid::from_u128(0x5e7a0002_3c1b_4f6e_9d2a_7b1c0e5a9f10);

type Inboxes = Arc<Mutex<HashMap<Address, mpsc::UnboundedSender<Vec<u8>>>>>;

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

    pub async fn stop(&self) {
        listener::stop_all(&self.stop_tx, &self.listeners).await;
    }
}

impl Drop for BleServer {
    fn drop(&mut self) {
        self.accept_task.abort();
    }
}

pub async fn start(encoded: EncodedTx, key: Key) -> Result<BleServer, String> {
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

    println!("[ble] 광고 시작");
    Ok(BleServer { _adv: adv, _app: app, accept_task, listeners, stop_tx })
}

async fn listener_loop(
    writer: CharacteristicWriter,
    key: Key,
    inbox: mpsc::UnboundedReceiver<Vec<u8>>,
    inboxes: Inboxes,
    encoded: EncodedTx,
    stop: watch::Receiver<bool>,
    count: Arc<AtomicUsize>,
) {
    let addr = writer.device_address();
    let max = writer.mtu().saturating_sub(ATT_HEADER_LEN);
    listener::serve(Notifier(writer), addr.to_string(), max, key, inbox, encoded, stop, count).await;
    inboxes.lock().unwrap().remove(&addr);
}

struct Notifier(CharacteristicWriter);

impl PhoneLink for Notifier {
    async fn send(&mut self, packet: &[u8]) -> Result<(), String> {
        self.0.write_all(packet).await.map_err(|e| e.to_string())
    }
}
