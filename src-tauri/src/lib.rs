#[cfg(target_os = "android")]
mod capture_android;
#[cfg(target_os = "linux")]
mod capture_linux;
#[cfg(target_os = "windows")]
mod capture_win;

#[cfg(target_os = "android")]
use capture_android::{capture_sound, stop_capture, CaptureStream};
#[cfg(target_os = "linux")]
use capture_linux::{capture_sound, stop_capture, CaptureStream};
#[cfg(target_os = "windows")]
use capture_win::{capture_sound, stop_capture, CaptureStream};

mod channel;
use channel::open_room;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(CaptureStream(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            get_ip,
            capture_sound,
            stop_capture,
            open_room,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
