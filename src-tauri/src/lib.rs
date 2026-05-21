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

mod server;
mod tunnel;

use server::{start_server, stop_server, ServerHandle};
use tokio::sync::broadcast;
use tunnel::{close_tunnel, open_tunnel, TunnelHandle};

#[tauri::command]
fn get_ip() -> String {
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_default()
}

pub fn run() {
    // 오디오 청크 브로드캐스트 채널 (1024 청크 버퍼)
    let (audio_tx, _) = broadcast::channel::<Vec<u8>>(1024);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(CaptureStream::new())
        .manage(ServerHandle::new())
        .manage(TunnelHandle(tokio::sync::Mutex::new(None)))
        .manage(audio_tx)
        .invoke_handler(tauri::generate_handler![
            get_ip,
            capture_sound,
            stop_capture,
            start_server,
            stop_server,
            open_tunnel,
            close_tunnel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
