// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Data;
use local_ip_address::local_ip;
use tauri::Emitter;

#[tauri::command]
fn capture_sound(app: tauri::AppHandle) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");

    let config = device.default_output_config().unwrap().into();

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                app.emit("audio-data", data.to_vec()).unwrap();
            },
            move |e| {
                println!("{}", e);
            },
            None,
        )
        .unwrap();

    std::thread::spawn(move || {
        stream.play().unwrap();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
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
        .invoke_handler(tauri::generate_handler![greet, get_ip, capture_sound])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
