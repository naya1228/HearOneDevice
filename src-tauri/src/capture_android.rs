use std::sync::Mutex;
use tauri::State;

use crate::channel::AudioBroadcast;
use crate::AudioConfig;

pub struct CaptureStream(pub Mutex<Option<()>>);

#[tauri::command]
pub fn capture_sound(
    _state: State<'_, CaptureStream>,
    _broadcast: State<'_, AudioBroadcast>,
) -> Result<AudioConfig, String> {
    Err("Android에서는 오디오 캡처가 아직 지원되지 않습니다".to_string())
}

#[tauri::command]
pub fn stop_capture(_state: State<'_, CaptureStream>) {}
