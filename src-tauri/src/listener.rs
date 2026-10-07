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
