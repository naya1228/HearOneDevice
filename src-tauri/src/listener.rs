// docs/PROTOCOL.md: 제어 메시지, 연결 확인, 잠그기
use crate::auth::{Handshake, Key, Sealer, REPLY_TIMEOUT};
use crate::codec::packet::{
    Packetizer, CONTROL_AUTH_FAIL, CONTROL_AUTH_OK, CONTROL_CHALLENGE, CONTROL_STOP,
};
use crate::encoding::EncodedTx;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch};

pub const ATT_HEADER_LEN: usize = 3;
pub const MIN_PACKET: usize = 20;
pub const MAX_PACKET: usize = 512;
const STOP_WAIT: Duration = Duration::from_millis(500);
const STOP_FLUSH: Duration = Duration::from_millis(200);
const STOP_POLL: Duration = Duration::from_millis(10);

pub trait PhoneLink: Send {
    fn send(&mut self, packet: &[u8]) -> impl Future<Output = Result<(), String>> + Send;
}

pub async fn serve(
    mut link: impl PhoneLink,
    name: String,
    max_packet: usize,
    key: Key,
    inbox: mpsc::UnboundedReceiver<Vec<u8>>,
    encoded: EncodedTx,
    stop: watch::Receiver<bool>,
    count: Arc<AtomicUsize>,
) {
    let mut packetizer = Packetizer::new(max_packet.clamp(MIN_PACKET, MAX_PACKET));
    if authenticate(&mut link, &mut packetizer, &name, key, inbox).await {
        let _counted = Counted::new(count);
        send_audio(&mut link, &mut packetizer, &name, encoded, stop).await;
    }
}

// docs/PROTOCOL.md: 제어 메시지
pub async fn stop_all(stop: &watch::Sender<bool>, count: &AtomicUsize) {
    stop.send_replace(true);
    let deadline = tokio::time::Instant::now() + STOP_WAIT;
    while count.load(Ordering::Relaxed) > 0 && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(STOP_POLL).await;
    }
    tokio::time::sleep(STOP_FLUSH).await;
}

struct Counted(Arc<AtomicUsize>);

impl Counted {
    fn new(count: Arc<AtomicUsize>) -> Self {
        count.fetch_add(1, Ordering::Relaxed);
        Self(count)
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

// docs/PROTOCOL.md: 연결 확인
async fn authenticate(
    link: &mut impl PhoneLink,
    packetizer: &mut Packetizer,
    name: &str,
    key: Key,
    mut inbox: mpsc::UnboundedReceiver<Vec<u8>>,
) -> bool {
    let hs = Handshake::new(key);
    if let Err(e) = link.send(&packetizer.control_with(CONTROL_CHALLENGE, hs.challenge())).await {
        println!("[ble] {name} 연결 끊김: {e}");
        return false;
    }
    let passed = match tokio::time::timeout(REPLY_TIMEOUT, inbox.recv()).await {
        Ok(Some(reply)) => hs.verify(&reply),
        _ => None,
    };
    let Some(passed) = passed else {
        let _ = link.send(&packetizer.control(CONTROL_AUTH_FAIL)).await;
        println!("[ble] {name} 확인 실패 (열쇠가 다르거나 답이 없음). 소리를 보내지 않음");
        return false;
    };
    if let Err(e) = link.send(&packetizer.control_with(CONTROL_AUTH_OK, &passed.pc_proof)).await {
        println!("[ble] {name} 연결 끊김: {e}");
        return false;
    }
    packetizer.seal_from_now(Sealer::new(&passed.audio_key));
    println!("[ble] {name} 확인 통과");
    true
}

async fn send_audio(
    link: &mut impl PhoneLink,
    packetizer: &mut Packetizer,
    name: &str,
    encoded: EncodedTx,
    mut stop: watch::Receiver<bool>,
) {
    let mut rx = encoded.subscribe();
    loop {
        let received = tokio::select! {
            r = rx.recv() => r,
            _ = stop.changed() => {
                let _ = link.send(&packetizer.control(CONTROL_STOP)).await;
                println!("[ble] {name} 공유 중지 알림");
                break;
            }
        };
        let enc = match received {
            Ok(u) => u,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                println!("[ble] {name} 전송 밀림, 조각 {n}개 건너뜀");
                continue;
            }
            Err(broadcast::error::RecvError::Closed) => break,
        };
        let Some(packet) = packetizer.packet_for(enc.codec, &enc.data) else { continue };
        if let Err(e) = link.send(&packet).await {
            println!("[ble] {name} 연결 끊김: {e}");
            break;
        }
    }
}

// docs/PROTOCOL.md: 제어 메시지, 연결 확인, 잠그기
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Codec;
    use crate::encoding::Encoded;
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    use aes_gcm::Aes256Gcm;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    const KEY: Key = [7; 32];

    struct FakePhone(mpsc::UnboundedSender<Vec<u8>>);

    impl PhoneLink for FakePhone {
        async fn send(&mut self, packet: &[u8]) -> Result<(), String> {
            self.0.send(packet.to_vec()).map_err(|e| e.to_string())
        }
    }

    struct Pc {
        sent: mpsc::UnboundedReceiver<Vec<u8>>,
        reply: mpsc::UnboundedSender<Vec<u8>>,
        encoded: EncodedTx,
        stop: watch::Sender<bool>,
        count: Arc<AtomicUsize>,
        task: tokio::task::JoinHandle<()>,
    }

    fn start_pc() -> Pc {
        let (sent_tx, sent) = mpsc::unbounded_channel();
        let (reply, inbox) = mpsc::unbounded_channel();
        let (encoded, _) = broadcast::channel(8);
        let (stop, stop_rx) = watch::channel(false);
        let count = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn(serve(
            FakePhone(sent_tx),
            "폰".into(),
            244,
            KEY,
            inbox,
            encoded.clone(),
            stop_rx,
            count.clone(),
        ));
        Pc { sent, reply, encoded, stop, count, task }
    }

    fn hmac(key: &Key, parts: &[&[u8]]) -> Vec<u8> {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).unwrap();
        parts.iter().for_each(|p| mac.update(p));
        mac.finalize().into_bytes().to_vec()
    }

    fn control(seq: u16, cmd: u8) -> Vec<u8> {
        let s = seq.to_le_bytes();
        vec![2, 0, s[0], s[1], cmd]
    }

    fn open(audio_key: &[u8], seq: u64, packet: &[u8]) -> Vec<u8> {
        let mut nonce = [0u8; 12];
        nonce[4..].copy_from_slice(&seq.to_le_bytes());
        Aes256Gcm::new_from_slice(audio_key)
            .unwrap()
            .decrypt(&nonce.into(), Payload { msg: &packet[4..], aad: &packet[..4] })
            .expect("풀리지 않음")
    }

    async fn answer(pc: &mut Pc, key: &Key) -> ([u8; 16], Vec<u8>) {
        let challenge = pc.sent.recv().await.unwrap();
        assert_eq!(challenge.len(), 21);
        assert_eq!(&challenge[..5], control(0, 2).as_slice());
        let p = challenge[5..].to_vec();
        let f = [9u8; 16];
        let mut reply = vec![1, 1];
        reply.extend_from_slice(&f);
        reply.extend(hmac(key, &[b"hearone-phone", &p, &f]));
        pc.reply.send(reply).unwrap();
        (f, p)
    }

    #[tokio::test]
    async fn right_key_then_audio_then_stop() {
        let mut pc = start_pc();
        let (f, p) = answer(&mut pc, &KEY).await;

        let ok = pc.sent.recv().await.unwrap();
        assert_eq!(&ok[..5], control(1, 3).as_slice());
        assert_eq!(ok[5..], hmac(&KEY, &[b"hearone-pc", &p, &f]));

        while pc.count.load(Ordering::Relaxed) == 0 {
            tokio::task::yield_now().await;
        }
        let audio_key = hmac(&KEY, &[b"hearone-audio", &p, &f]);
        pc.encoded.send(Arc::new(Encoded { codec: Codec::OpusStereo64k, data: vec![5; 10] })).ok().unwrap();
        let audio = pc.sent.recv().await.unwrap();
        assert_eq!(&audio[..4], &[2, Codec::OpusStereo64k.id(), 2, 0]);
        assert_eq!(audio.len(), 4 + 10 + 16);
        assert_eq!(open(&audio_key, 2, &audio), [5; 10]);

        pc.stop.send_replace(true);
        let stop = pc.sent.recv().await.unwrap();
        assert_eq!(&stop[..4], &control(3, 1)[..4]);
        assert_eq!(open(&audio_key, 3, &stop), [1]);
        pc.task.await.unwrap();
        assert_eq!(pc.count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn wrong_key_gets_fail_and_no_audio() {
        let mut pc = start_pc();
        answer(&mut pc, &[8; 32]).await;
        assert_eq!(pc.sent.recv().await.unwrap(), control(1, 4));
        pc.task.await.unwrap();
        assert!(pc.sent.recv().await.is_none());
        assert_eq!(pc.count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn aborted_listener_is_not_counted() {
        let mut pc = start_pc();
        answer(&mut pc, &KEY).await;
        while pc.count.load(Ordering::Relaxed) == 0 {
            tokio::task::yield_now().await;
        }
        pc.task.abort();
        let _ = (&mut pc.task).await;
        assert_eq!(pc.count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn no_reply_in_5s_gets_fail() {
        let mut pc = start_pc();
        let challenge = pc.sent.recv().await.unwrap();
        assert_eq!(&challenge[..5], control(0, 2).as_slice());
        let before = tokio::time::Instant::now();
        assert_eq!(pc.sent.recv().await.unwrap(), control(1, 4));
        assert_eq!(before.elapsed(), REPLY_TIMEOUT);
        pc.task.await.unwrap();
    }
}
