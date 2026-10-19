#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("HearOneDevice supports Windows and Linux only.");

pub mod audio;
pub mod auth;
#[cfg_attr(target_os = "windows", path = "ble_windows.rs")]
pub mod ble;
pub mod capture;
pub mod codec;
pub mod device_id;
pub mod encoding;
pub mod link;
pub mod listener;

use codec::Codec;
use serde::Serialize;
use tauri::{Manager, State};
use tokio::sync::{watch, Mutex};

// docs/FEATURES.md: 공유
struct Sharing {
    ble: ble::BleServer,
    _encoding: encoding::Encoding,
    _capture: capture::Capture,
}

struct AppState {
    sharing: Mutex<Option<Sharing>>,
    id: device_id::DeviceId,
    key: auth::Key,
    name: String,
    codec: watch::Sender<Codec>,
}

#[derive(Serialize)]
struct Status {
    running: bool,
    listeners: usize,
    link: String,
    name: String,
    codec: u8,
}

#[derive(Serialize)]
struct CodecInfo {
    id: u8,
    name: &'static str,
    warning: Option<&'static str>,
}

#[tauri::command]
async fn start_sharing(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.sharing.lock().await;
    if guard.is_some() {
        return Ok(());
    }
    let audio = audio::channel();
    let (encoding, encoded) = encoding::start(&audio, state.codec.subscribe());
    let ble = ble::start(encoded, state.key).await?;
    let capture = capture::start(audio)?;
    *guard = Some(Sharing { ble, _encoding: encoding, _capture: capture });
    Ok(())
}

#[tauri::command]
async fn stop_sharing(state: State<'_, AppState>) -> Result<(), String> {
    end_sharing(&state).await;
    Ok(())
}

// docs/PROTOCOL.md: 제어 메시지
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
        link: link::connect_link(&state.id, &state.key, &state.name),
        name: state.name.clone(),
        codec: state.codec.borrow().id(),
    })
}

// docs/CODECS.md: 코덱 바꾸는 법
#[tauri::command]
fn codecs() -> Vec<CodecInfo> {
    Codec::ALL.iter().map(|&c| CodecInfo { id: c.id(), name: c.name(), warning: c.warning() }).collect()
}

// docs/CODECS.md: 코덱 바꾸는 법
#[tauri::command]
fn set_codec(state: State<'_, AppState>, id: u8) -> Result<(), String> {
    let codec = Codec::from_id(id).ok_or(format!("없는 코덱 번호: {id}"))?;
    state.codec.send_replace(codec);
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            app.manage(AppState {
                sharing: Mutex::new(None),
                id: device_id::load_device_id(dir.clone()),
                key: auth::load_key(dir),
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
            // docs/FEATURES.md: 공유
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
