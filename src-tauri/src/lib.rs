// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use local_ip_address::local_ip;

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
        .invoke_handler(tauri::generate_handler![greet])
        .invoke_handler(tauri::generate_handler![get_ip])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
