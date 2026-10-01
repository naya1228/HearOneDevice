// 캡처 → 인코더 사이에 흐르는 데이터 형식
use std::sync::Arc;
use tokio::sync::broadcast;

/// 캡처한 소리 한 조각. 장치마다 샘플레이트/채널이 다를 수 있어 함께 싣는다.
pub struct AudioChunk {
    pub sample_rate: u32,
    pub channels: u16,
    /// 인터리브된 f32 샘플 (L R L R ...)
    pub samples: Vec<f32>,
}

pub type AudioTx = broadcast::Sender<Arc<AudioChunk>>;

pub fn channel() -> AudioTx {
    // 20ms 조각 기준 약 1초 분량. 느린 소비자는 Lagged로 건너뜀
    broadcast::channel(64).0
}
