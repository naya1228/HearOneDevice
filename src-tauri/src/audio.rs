// docs/FEATURES.md: 소리 캡처
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct AudioChunk {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

const CHANNEL_CAPACITY: usize = 64;

pub type AudioTx = broadcast::Sender<Arc<AudioChunk>>;

pub fn channel() -> AudioTx {
    broadcast::channel(CHANNEL_CAPACITY).0
}
