#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("HearOneDevice supports Windows and Linux only.");

pub mod audio;
#[cfg_attr(not(target_os = "linux"), path = "ble_unsupported.rs")]
pub mod ble;
pub mod capture;
pub mod codec;

use serde::Serialize;
use std::hash::{BuildHasher, Hasher};
use tauri::{Manager, State};
use tokio::sync::Mutex;

// 공유 중일 때만 Some. drop되면 캡처·BLE가 함께 멈춘다 (ble 먼저 drop되도록 순서 유지)
struct Sharing {
    ble: ble::BleServer,
    _capture: capture::Capture,
}

struct AppState {
    sharing: Mutex<Option<Sharing>>,
    id: ble::DeviceId,
}

#[derive(Serialize)]
struct Status {
    running: bool,
    listeners: usize,
    /// 폰이 QR로 읽는 연결 주소 (android 의 MainActivity 딥링크와 같아야 함)
    link: String,
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
    let ble = ble::start(audio.clone(), state.id).await?;
    let capture = capture::start(audio)?;
    *guard = Some(Sharing { ble, _capture: capture });
    Ok(())
}

#[tauri::command]
async fn stop_sharing(state: State<'_, AppState>) -> Result<(), String> {
    state.sharing.lock().await.take();
    Ok(())
}

#[tauri::command]
async fn sharing_status(state: State<'_, AppState>) -> Result<Status, String> {
    let guard = state.sharing.lock().await;
    Ok(Status {
        running: guard.is_some(),
        listeners: guard.as_ref().map_or(0, |s| s.ble.listeners()),
        link: format!("hearone://connect?id={}", ble::device_id_hex(&state.id)),
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            app.manage(AppState { sharing: Mutex::new(None), id: load_device_id(dir) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![start_sharing, stop_sharing, sharing_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
