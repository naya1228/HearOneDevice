use libpulse_binding::def::BufferAttr;
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple as LinuxAudioCapture;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;
use tokio::sync::broadcast;

pub struct CaptureStream {
    pub running: Mutex<Option<Arc<AtomicBool>>>,
}

impl CaptureStream {
    pub fn new() -> Self {
        Self {
            running: Mutex::new(None),
        }
    }
}

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 2;
const CHUNK_FRAMES: usize = 480;
const CHUNK_BYTES: usize = CHUNK_FRAMES * CHANNELS as usize * std::mem::size_of::<f32>();

#[tauri::command]
pub fn capture_sound(
    state: State<'_, CaptureStream>,
    tx: State<'_, broadcast::Sender<Vec<u8>>>,
) -> Result<(), String> {
    // 이미 캡처 중이면 중복 시작 방지
    if state.running.lock().unwrap().is_some() {
        return Ok(());
    }

    let output = std::process::Command::new("pactl")
        .args(["get-default-sink"])
        .output()
        .map_err(|e| e.to_string())?;
    let sink = String::from_utf8(output.stdout)
        .map_err(|e| e.to_string())?
        .trim()
        .to_string();
    let monitor = format!("{sink}.monitor");

    let spec = Spec {
        format: Format::FLOAT32NE,
        channels: CHANNELS as u8,
        rate: SAMPLE_RATE,
    };

    let buf_attr = BufferAttr {
        maxlength: u32::MAX,
        fragsize: CHUNK_BYTES as u32,
        ..Default::default()
    };

    let stream = LinuxAudioCapture::new(
        None,
        "ShareYourSounds",
        Direction::Record,
        Some(monitor.as_str()),
        "capture",
        &spec,
        None,
        Some(&buf_attr),
    )
    .map_err(|e| e.to_string().unwrap_or("PulseAudio error".into()))?;

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    let sender = tx.inner().clone();

    std::thread::spawn(move || {
        let mut buf = vec![0u8; CHUNK_BYTES];
        while running_clone.load(Ordering::Relaxed) {
            if stream.read(&mut buf).is_err() {
                break;
            }
            // 구독자가 없어도 무시 (lossy broadcast)
            let _ = sender.send(buf.clone());
        }
    });

    *state.running.lock().unwrap() = Some(running);
    Ok(())
}

#[tauri::command]
pub fn stop_capture(state: State<'_, CaptureStream>) {
    if let Some(running) = state.running.lock().unwrap().take() {
        running.store(false, Ordering::Relaxed);
    }
}
