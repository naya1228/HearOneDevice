#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("HearOneDevice supports Windows and Linux only.");

pub mod audio;
#[cfg_attr(not(target_os = "linux"), path = "ble_unsupported.rs")]
pub mod ble;
pub mod capture;
pub mod codec;
pub mod encoding;
pub mod link;

use codec::Codec;
use serde::Serialize;
use std::hash::{BuildHasher, Hasher};
use tauri::{Manager, State};
use tokio::sync::{watch, Mutex};

// 공유 중일 때만 Some. drop되면 전송·인코딩·캡처가 함께 멈춘다 (필드 순서 = drop 순서, ble 먼저)
struct Sharing {
    ble: ble::BleServer,
    _encoding: encoding::Encoding,
    _capture: capture::Capture,
}

struct AppState {
    sharing: Mutex<Option<Sharing>>,
    id: ble::DeviceId,
    /// QR에 싣는 PC 이름
    name: String,
    /// 지금 고른 코덱. 공유 중에 바꾸면 인코더가 다음 조각부터 따라간다
    codec: watch::Sender<Codec>,
}

#[derive(Serialize)]
struct Status {
    running: bool,
    listeners: usize,
    /// 폰이 QR로 읽는 연결 주소 (docs/PROTOCOL.md 1절)
    link: String,
    /// QR에 싣는 PC 이름 (폰 기록에 처음 저장되는 이름)
    name: String,
    /// 지금 고른 코덱 번호
    codec: u8,
}

#[derive(Serialize)]
struct CodecInfo {
    id: u8,
    name: &'static str,
}

// PC 고유 번호: 처음 한 번 만들어 설정 폴더에 저장 → 앱을 다시 켜도 QR이 그대로
fn load_device_id(dir: std::path::PathBuf) -> ble::DeviceId {
    let path = dir.join("device_id");
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(id) = <ble::DeviceId>::try_from(bytes.as_slice()) {
            return id;
        }
    }
    let random = std::collections::hash_map::RandomState::new().build_hasher().finish();
    let id: ble::DeviceId = (random as u32).to_be_bytes();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(&path, id);
    id
}

#[tauri::command]
async fn start_sharing(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.sharing.lock().await;
    if guard.is_some() {
        return Ok(());
    }
    let audio = audio::channel();
    let (encoding, encoded) = encoding::start(&audio, state.codec.subscribe());
    let ble = ble::start(encoded, state.id).await?;
    let capture = capture::start(audio)?;
    *guard = Some(Sharing { ble, _encoding: encoding, _capture: capture });
    Ok(())
}

#[tauri::command]
async fn stop_sharing(state: State<'_, AppState>) -> Result<(), String> {
    end_sharing(&state).await;
    Ok(())
}

/// 공유를 끝낸다. 폰에 "공유 중지"를 알린 뒤 drop (그래야 폰이 다시 찾지 않고 멈춘다)
async fn end_sharing(state: &AppState) {
    let sharing = state.sharing.lock().await.take();
    if let Some(s) = sharing {
        s.ble.stop().await;
    }
}

#[tauri::command]
async fn sharing_status(state: State<'_, AppState>) -> Result<Status, String> {
    let guard = state.sharing.lock().await;
    Ok(Status {
        running: guard.is_some(),
        listeners: guard.as_ref().map_or(0, |s| s.ble.listeners()),
        link: link::connect_link(&state.id, &state.name),
        name: state.name.clone(),
        codec: state.codec.borrow().id(),
    })
}

/// 화면에서 고를 수 있는 코덱 목록 (번호·이름은 docs/CODECS.md)
#[tauri::command]
fn codecs() -> Vec<CodecInfo> {
    Codec::ALL.iter().map(|&c| CodecInfo { id: c.id(), name: c.name() }).collect()
}

/// 코덱 바꾸기. 공유 중이면 바로 적용되고, 폰은 패킷 헤더의 번호를 보고 따라간다
#[tauri::command]
fn set_codec(state: State<'_, AppState>, id: u8) -> Result<(), String> {
    let codec = Codec::from_id(id).ok_or(format!("없는 코덱 번호: {id}"))?;
    state.codec.send_replace(codec);
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            app.manage(AppState {
                sharing: Mutex::new(None),
                id: load_device_id(dir),
                name: link::pc_name(),
                codec: watch::channel(Codec::DEFAULT).0,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_sharing,
            stop_sharing,
            sharing_status,
            codecs,
            set_codec
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // 창을 닫아 앱이 끝날 때도 폰에 "공유 중지"를 알리고 종료.
            // code: None = 사용자가 닫음, Some = 아래 app.exit() 가 부른 것 (다시 막지 않음)
            if let tauri::RunEvent::ExitRequested { code: None, api, .. } = event {
                api.prevent_exit();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    end_sharing(&app.state::<AppState>()).await;
                    app.exit(0);
                });
            }
        });
}
