// 기본 출력 장치를 WASAPI 루프백으로 캡처 (cpal: 출력 장치에 input stream을 열면 루프백)
use crate::audio::{AudioChunk, AudioTx};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;

/// drop되면 캡처가 멈춘다.
pub struct Capture {
    _stream: cpal::Stream,
}

pub fn start(tx: AudioTx) -> Result<Capture, String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("출력 장치를 찾을 수 없습니다")?;
    let config: cpal::StreamConfig = device
        .default_output_config()
        .map_err(|e| e.to_string())?
        .into();
    let (sample_rate, channels) = (config.sample_rate, config.channels);

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let _ = tx.send(Arc::new(AudioChunk {
                    sample_rate,
                    channels,
                    samples: data.to_vec(),
                }));
            },
            |err| eprintln!("[capture] {err}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;

    Ok(Capture { _stream: stream })
}
