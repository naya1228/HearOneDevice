use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Mutex;
use tauri::State;
use tokio::sync::broadcast;

pub struct CaptureStream(pub Mutex<Option<cpal::Stream>>);

impl CaptureStream {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
}

#[tauri::command]
pub fn capture_sound(
    state: State<'_, CaptureStream>,
    tx: State<'_, broadcast::Sender<Vec<u8>>>,
) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("출력 장치를 찾을 수 없습니다")?;

    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let config: cpal::StreamConfig = supported.into();

    let sender = tx.inner().clone();

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // f32 슬라이스를 little-endian 바이트로 변환
                let mut bytes = Vec::with_capacity(data.len() * 4);
                for &sample in data {
                    bytes.extend_from_slice(&sample.to_le_bytes());
                }
                let _ = sender.send(bytes);
            },
            |err| eprintln!("stream error: {err}"),
            None,
        )
        .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;
    *state.0.lock().unwrap() = Some(stream);

    Ok(())
}

#[tauri::command]
pub fn stop_capture(state: State<'_, CaptureStream>) {
    *state.0.lock().unwrap() = None;
}
