use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple as LinuxAudioCapture;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, State};

use crate::AudioConfig;
use std::string::String;

pub struct CaptureStream(pub Mutex<Option<Arc<LinuxAudioCapture>>>);

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 2;

#[tauri::command]
pub fn capture_sound(
    app: tauri::AppHandle,
    state: State<'_, CaptureStream>,
) -> Result<AudioConfig, String> {
    //.monitor로 끝나는 입력장치 찾기
    let output = std::process::Command::new("pactl")
        .args(["get-default-sink"])
        .output()
        .map_err(|e| e.to_string())?;
    let sink = String::from_utf8(output.stdout)
        .map_err(|e| e.to_string())?
        .trim()
        .to_string();
    let monitor = format!("{sink}.monitor");
    println!("연결된 모니터 소스: {monitor}");

    let spec = Spec {
        format: Format::FLOAT32NE,
        channels: CHANNELS as u8,
        rate: SAMPLE_RATE,
    };

    //입력장치 바인딩
    let stream = Arc::new(
        LinuxAudioCapture::new(
            None,                   // server
            "Heare-one-device",     // name
            Direction::Record,      // dir
            Some(monitor.as_str()), // dev (monitor source)
            "capture",              // stream_name
            &spec,                  // sample spec
            None,                   // channel map
            None,                   // buffer attr
        )
        .map_err(|e| e.to_string().unwrap_or("PulseAudio error".into()))?,
    );

    let stream_clone = stream.clone();
    std::thread::spawn(move || {
        let frame_bytes = (CHANNELS as usize) * std::mem::size_of::<f32>();
        let mut buf = vec![0u8; frame_bytes * 1024];

        loop {
            if stream_clone.read(&mut buf).is_err() {
                break;
            }
            let samples: Vec<f32> = buf
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                .collect();
            let _ = app.emit("audio-data", samples);
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
