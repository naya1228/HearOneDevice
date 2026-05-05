#[cfg(not(any(target_os = "linux", target_os = "windows")))]
compile_error!("HearOneDevice supports Windows and Linux only.");

#[cfg(target_os = "linux")]
mod capture_linux;
#[cfg(target_os = "windows")]
mod capture_win;

#[cfg(target_os = "linux")]
use capture_linux::{capture_sound, stop_capture, CaptureStream};
#[cfg(target_os = "windows")]
use capture_win::{capture_sound, stop_capture, CaptureStream};

mod channel;
use channel::{close_room, open_room, AudioBroadcast, ServerHandle};

use local_ip_address::local_ip;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Serialize)]
pub struct AudioConfig {
    sample_rate: u32,
    channels: u16,
}

#[tauri::command]
fn get_ip() -> String {
    let local_address = local_ip().unwrap();
    format!("{}", local_address)
}

pub fn run() {
    let (audio_tx, _) = tokio::sync::broadcast::channel::<bytes::Bytes>(8);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(CaptureStream(Mutex::new(None)))
        .manage(AudioBroadcast(audio_tx))
        .manage(ServerHandle(tokio::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            get_ip,
            capture_sound,
            stop_capture,
            open_room,
            close_room,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
