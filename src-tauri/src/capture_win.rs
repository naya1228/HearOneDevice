use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Mutex;
use tauri::{Emitter, State};

use crate::AudioConfig;

pub struct CaptureStream(pub Mutex<Option<cpal::Stream>>);

#[tauri::command]
pub fn capture_sound(
    app: tauri::AppHandle,
    state: State<'_, CaptureStream>,
) -> Result<AudioConfig, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("출력 장치를 찾을 수 없습니다")?;

    let supported = device.default_output_config().map_err(|e| e.to_string())?;

    let sample_rate = supported.sample_rate();
    let channels = supported.channels();
    let config: cpal::StreamConfig = supported.into();

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let _ = app.emit("audio-data", data.to_vec());
            },
            |err| eprintln!("stream error: {err}"),
            None,
        )
        .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;

    *state.0.lock().unwrap() = Some(stream);

    Ok(AudioConfig {
        sample_rate,
        channels,
    })
}

#[tauri::command]
pub fn stop_capture(state: State<'_, CaptureStream>) {
    *state.0.lock().unwrap() = None;
}
