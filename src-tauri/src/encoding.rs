// docs/FEATURES.md: 인코딩
use crate::audio::{AudioChunk, AudioTx};
use crate::codec::{self, Codec};
use std::sync::Arc;
use tokio::sync::{broadcast, watch};

pub struct Encoded {
    pub codec: Codec,
    pub data: Vec<u8>,
}

const CHANNEL_CAPACITY: usize = 64;

pub type EncodedTx = broadcast::Sender<Arc<Encoded>>;

pub struct Encoding(tokio::task::JoinHandle<()>);

impl Drop for Encoding {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub fn start(audio: &AudioTx, codec: watch::Receiver<Codec>) -> (Encoding, EncodedTx) {
    let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
    let task = tokio::spawn(encode_loop(audio.subscribe(), codec, tx.clone()));
    (Encoding(task), tx)
}

async fn encode_loop(
    mut rx: broadcast::Receiver<Arc<AudioChunk>>,
    mut codec_rx: watch::Receiver<Codec>,
    tx: EncodedTx,
) {
    let mut codec = *codec_rx.borrow_and_update();
    let mut enc = codec::encoder(codec);
    loop {
        match rx.recv().await {
            Ok(chunk) => {
                if codec_rx.has_changed().unwrap_or(false) {
                    codec = *codec_rx.borrow_and_update();
                    enc = codec::encoder(codec);
                    println!("[encode] 코덱 변경: {} {}", codec.id(), codec.name());
                }
                for data in enc.encode(&chunk) {
                    if !data.is_empty() {
                        let _ = tx.send(Arc::new(Encoded { codec, data }));
                    }
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}
