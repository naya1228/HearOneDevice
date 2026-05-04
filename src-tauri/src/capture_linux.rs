use bytes::Bytes;
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple as LinuxAudioCapture;
use std::sync::{Arc, Mutex};
use tauri::State;

use crate::channel::AudioBroadcast;
use crate::AudioConfig;

pub struct CaptureStream(pub Mutex<Option<Arc<LinuxAudioCapture>>>);

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 2;

#[tauri::command]
pub fn capture_sound(
    state: State<'_, CaptureStream>,
    broadcast: State<'_, AudioBroadcast>,
) -> Result<AudioConfig, String> {
    let output = std::process::Command::new("pactl")
        .args(["get-default-sink"])
        .output()
        .map_err(|e| e.to_string())?;
    let sink = String::from_utf8(output.stdout)
        .map_err(|e| e.to_string())?
        .trim()
        .to_string();
    let monitor = format!("{sink}.monitor");
    println!("모니터 소스: {monitor}");

    let spec = Spec {
        format: Format::FLOAT32NE,
        channels: CHANNELS as u8,
        rate: SAMPLE_RATE,
    };

    let stream = Arc::new(
        LinuxAudioCapture::new(
            None,
            "ShareYourSounds",
            Direction::Record,
            Some(monitor.as_str()),
            "capture",
            &spec,
            None,
            None,
        )
        .map_err(|e| e.to_string().unwrap_or("PulseAudio error".into()))?,
    );

    let stream_clone = stream.clone();
    let tx = broadcast.0.clone();
    std::thread::spawn(move || {
        let frame_bytes = (CHANNELS as usize) * std::mem::size_of::<f32>();
        let mut buf = vec![0u8; frame_bytes * 1024];

        loop {
            if stream_clone.read(&mut buf).is_err() {
                break;
            }
            // 수신자가 없으면 전송 스킵
            if tx.receiver_count() > 0 {
                let _ = tx.send(Bytes::copy_from_slice(&buf));
            }
        }
    });

    *state.0.lock().unwrap() = Some(stream);

    Ok(AudioConfig {
        sample_rate: SAMPLE_RATE,
        channels: CHANNELS,
    })
}

#[tauri::command]
pub fn stop_capture(state: State<'_, CaptureStream>) {
    *state.0.lock().unwrap() = None;
}
