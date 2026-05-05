use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Mutex;
use tauri::State;

use crate::channel::AudioBroadcast;
use crate::AudioConfig;

pub struct CaptureStream(pub Mutex<Option<cpal::Stream>>);

#[tauri::command]
pub fn capture_sound(
    state: State<'_, CaptureStream>,
    broadcast: State<'_, AudioBroadcast>,
) -> Result<AudioConfig, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("출력 장치를 찾을 수 없습니다")?;

    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let sample_rate = supported.sample_rate();
    let channels = supported.channels();
    let config: cpal::StreamConfig = supported.into();

    let tx = broadcast.0.clone();
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if tx.receiver_count() > 0 {
                    let bytes = Bytes::from(
                        data.iter()
                            .flat_map(|s| s.to_le_bytes())
                            .collect::<Vec<u8>>(),
                    );
                    let _ = tx.send(bytes);
                }
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
