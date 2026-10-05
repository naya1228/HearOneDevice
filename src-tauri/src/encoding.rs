// 캡처한 소리를 고른 코덱으로 인코딩해 전송 쪽에 흘린다.
// 코덱은 실행 중에 바꿀 수 있다 (watch 통로로 받은 값이 바뀌면 다음 조각부터 새 코덱).

use crate::audio::{AudioChunk, AudioTx};
use crate::codec::{self, Codec};
use std::sync::Arc;
use tokio::sync::{broadcast, watch};

/// 인코딩된 조각 하나. 어떤 코덱인지 같이 실어서 전송 쪽이 패킷 헤더에 쓴다.
pub struct Encoded {
    pub codec: Codec,
    pub data: Vec<u8>,
}

/// 인코딩된 데이터가 흐르는 통로 (인코더 → 전송)
pub type EncodedTx = broadcast::Sender<Arc<Encoded>>;

/// drop되면 인코딩을 멈춘다.
pub struct Encoding(tokio::task::JoinHandle<()>);

impl Drop for Encoding {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// 캡처한 소리를 codec 통로가 가리키는 코덱으로 인코딩해 새 통로로 흘린다.
/// 인코딩은 한 번만 하고, 전송 쪽 청취자들이 이 통로를 나눠 구독한다.
pub fn start(audio: &AudioTx, codec: watch::Receiver<Codec>) -> (Encoding, EncodedTx) {
    let (tx, _) = broadcast::channel(64);
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
                // 고르는 쪽이 사라졌으면(Err) 지금 코덱 그대로
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
