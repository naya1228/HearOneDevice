// 기본 출력 장치의 monitor(스피커로 가는 소리의 복사본)를 PulseAudio로 캡처
use crate::audio::{AudioChunk, AudioTx};
use libpulse_binding::def::BufferAttr;
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 2;
const CHUNK_FRAMES: usize = 960; // 20ms
const CHUNK_BYTES: usize = CHUNK_FRAMES * CHANNELS as usize * 4;

/// drop되면 캡처 스레드가 멈춘다.
pub struct Capture {
    running: Arc<AtomicBool>,
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

pub fn start(tx: AudioTx) -> Result<Capture, String> {
    let output = std::process::Command::new("pactl")
        .arg("get-default-sink")
        .output()
        .map_err(|e| format!("pactl 실행 실패: {e}"))?;
    let sink = String::from_utf8_lossy(&output.stdout).trim().to_string();
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
    let stream = Simple::new(
        None,
        "HearOneDevice",
        Direction::Record,
        Some(&monitor),
        "capture",
        &spec,
        None,
        Some(&buf_attr),
    )
    .map_err(|e| e.to_string().unwrap_or("PulseAudio 오류".into()))?;
    println!("[capture] {monitor}");

    let running = Arc::new(AtomicBool::new(true));
    let flag = running.clone();
    std::thread::spawn(move || {
        let mut buf = vec![0u8; CHUNK_BYTES];
        while flag.load(Ordering::Relaxed) {
            if stream.read(&mut buf).is_err() {
                break;
            }
            let samples = buf
                .chunks_exact(4)
                .map(|b| f32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
                .collect();
            // 받는 쪽이 없으면 버려짐
            let _ = tx.send(Arc::new(AudioChunk {
                sample_rate: SAMPLE_RATE,
                channels: CHANNELS,
                samples,
            }));
        }
    });

    Ok(Capture { running })
}
