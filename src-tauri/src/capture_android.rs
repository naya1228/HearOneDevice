use std::sync::Mutex;
use tauri::State;

use crate::AudioConfig;

pub struct CaptureStream(pub Mutex<Option<()>>);

#[tauri::command]
pub fn capture_sound(
    _app: tauri::AppHandle,
    _state: State<'_, CaptureStream>,
) -> Result<AudioConfig, String> {
    Err("Android에서는 오디오 캡처가 아직 지원되지 않습니다".to_string())
}

#[tauri::command]
pub fn stop_capture(_state: State<'_, CaptureStream>) {}
