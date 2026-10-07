// 구독한 폰 한 대 처리: 열쇠 확인 → 소리 보내기. OS와 무관한 공통 순서.
// OS 파일(ble.rs·ble_windows.rs)은 그 폰에 바이트를 보내는 방법(PhoneLink)과 폰이 쓴 바이트(inbox)만 넘긴다.
// 절차·형식은 docs/PROTOCOL.md 4·5절

use crate::auth::{Handshake, Key, REPLY_TIMEOUT};
use crate::codec::packet::{
    Packetizer, CONTROL_AUTH_FAIL, CONTROL_AUTH_OK, CONTROL_CHALLENGE, CONTROL_STOP,
};
use crate::encoding::EncodedTx;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch};

/// 구독한 폰 한 대에 패킷 하나(notify 하나)를 보내는 방법. OS마다 구현
pub trait PhoneLink: Send {
    fn send(&mut self, packet: &[u8]) -> impl Future<Output = Result<(), String>> + Send;
}

/// 한 폰이 끝날 때까지(연결 끊김·확인 실패·공유 중지). 확인을 통과한 동안만 count에 들어간다.
/// name = 로그에 찍을 폰 이름, max_packet = 한 번에 보낼 수 있는 최대 바이트 (MTU − 3)
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
    // 속성 최대 길이 512
    let mut packetizer = Packetizer::new(max_packet.clamp(20, 512));
    if authenticate(&mut link, &mut packetizer, &name, key, inbox).await {
        count.fetch_add(1, Ordering::Relaxed);
        send_audio(&mut link, &mut packetizer, &name, encoded, stop).await;
        count.fetch_sub(1, Ordering::Relaxed);
    }
}

/// 열쇠 확인: 문제 → 폰의 답 → PC 증명 (docs/PROTOCOL.md 5절). 통과하면 true
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
    let pc_proof = match tokio::time::timeout(REPLY_TIMEOUT, inbox.recv()).await {
        Ok(Some(reply)) => hs.verify(&reply),
        _ => None,
    };
    let Some(pc_proof) = pc_proof else {
        let _ = link.send(&packetizer.control(CONTROL_AUTH_FAIL)).await;
        println!("[ble] {name} 확인 실패 (열쇠가 다르거나 답이 없음). 소리를 보내지 않음");
        return false;
    };
    if let Err(e) = link.send(&packetizer.control_with(CONTROL_AUTH_OK, &pc_proof)).await {
        println!("[ble] {name} 연결 끊김: {e}");
        return false;
    }
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
            // stop 값은 false → true 로 한 번만 바뀐다
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

// 가짜 폰 통로로 PC가 보내는 패킷 순서가 docs/PROTOCOL.md 4·5절과 맞는지 확인.
// 폰의 계산은 handshake.rs를 쓰지 않고 문서대로 따로 한다.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::Codec;
    use crate::encoding::Encoded;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    const KEY: Key = [7; 32];

    /// 보낸 패킷을 테스트에 넘기기만 하는 폰 통로
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
        let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
        parts.iter().for_each(|p| mac.update(p));
        mac.finalize().into_bytes().to_vec()
    }

    /// 헤더 [버전 1, 코덱 0(제어), 순번 LE] + 명령
    fn control(seq: u16, cmd: u8) -> Vec<u8> {
        let s = seq.to_le_bytes();
        vec![1, 0, s[0], s[1], cmd]
    }

    /// 문제를 받아 열쇠 key로 답한다. 돌려주는 값 = 폰 무작위 값 F와 PC 문제 P
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

        // 통과한 뒤에야 청취자로 셈 (그 순간 소리 통로도 이미 구독됨)
        while pc.count.load(Ordering::Relaxed) == 0 {
            tokio::task::yield_now().await;
        }
        pc.encoded.send(Arc::new(Encoded { codec: Codec::OpusStereo64k, data: vec![5; 10] })).ok().unwrap();
        let audio = pc.sent.recv().await.unwrap();
        assert_eq!(&audio[..4], &[1, Codec::OpusStereo64k.id(), 2, 0]);
        assert_eq!(audio[4..], [5; 10]);

        pc.stop.send_replace(true);
        assert_eq!(pc.sent.recv().await.unwrap(), control(3, 1));
        pc.task.await.unwrap();
        assert_eq!(pc.count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn wrong_key_gets_fail_and_no_audio() {
        let mut pc = start_pc();
        answer(&mut pc, &[8; 32]).await;
        assert_eq!(pc.sent.recv().await.unwrap(), control(1, 4));
        pc.task.await.unwrap();
        // 끝났으므로 더 보낸 것 없음
        assert!(pc.sent.recv().await.is_none());
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
