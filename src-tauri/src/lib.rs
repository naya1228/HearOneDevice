#[cfg(target_os = "linux")]
mod capture_linux;
#[cfg(target_os = "windows")]
mod capture_win;
#[cfg(target_os = "android")]
mod capture_android;

#[cfg(target_os = "linux")]
use capture_linux::{capture_sound, stop_capture, CaptureStream};
#[cfg(target_os = "windows")]
use capture_win::{capture_sound, stop_capture, CaptureStream};
#[cfg(target_os = "android")]
use capture_android::{capture_sound, stop_capture, CaptureStream};

mod rtc;
use rtc::{close_rtc, connect_to_host, open_room, RtcState};

use local_ip_address::local_ip;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Serialize)]
pub struct AudioConfig {
    sample_rate: u32,
    channels: u16,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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
        .manage(RtcState {
            session: tokio::sync::Mutex::new(None),
            server_task: tokio::sync::Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_ip,
            capture_sound,
            stop_capture,
            open_room,
            connect_to_host,
            close_rtc,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
