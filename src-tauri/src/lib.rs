#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("HearOneDevice supports Windows and Linux only.");

pub mod audio;
#[cfg_attr(not(target_os = "linux"), path = "ble_unsupported.rs")]
pub mod ble;
pub mod capture;
pub mod codec;

use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

// 공유 중일 때만 Some. drop되면 캡처·BLE가 함께 멈춘다 (ble 먼저 drop되도록 순서 유지)
struct Sharing {
    ble: ble::BleServer,
    _capture: capture::Capture,
}

#[derive(Default)]
struct AppState(Mutex<Option<Sharing>>);

#[derive(Serialize)]
struct Status {
    running: bool,
    listeners: usize,
}

#[tauri::command]
async fn start_sharing(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.0.lock().await;
    if guard.is_some() {
        return Ok(());
    }
    let audio = audio::channel();
    let ble = ble::start(audio.clone()).await?;
    let capture = capture::start(audio)?;
    *guard = Some(Sharing { ble, _capture: capture });
    Ok(())
}

#[tauri::command]
async fn stop_sharing(state: State<'_, AppState>) -> Result<(), String> {
    state.0.lock().await.take();
    Ok(())
}

#[tauri::command]
async fn sharing_status(state: State<'_, AppState>) -> Result<Status, String> {
    let guard = state.0.lock().await;
    Ok(Status {
        running: guard.is_some(),
        listeners: guard.as_ref().map_or(0, |s| s.ble.listeners()),
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![start_sharing, stop_sharing, sharing_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
