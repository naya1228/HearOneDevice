// UI 없이 송신만 돌려보는 테스트용: cargo run --example send [코덱 번호]
// 코덱 번호를 빼면 기본 코덱. 번호는 docs/CODECS.md
use hear_one_device_lib::{audio, ble, capture, codec::Codec, encoding, link};
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), String> {
    let codec = match std::env::args().nth(1) {
        Some(arg) => {
            arg.parse().ok().and_then(Codec::from_id).ok_or(format!("없는 코덱 번호: {arg}"))?
        }
        None => Codec::DEFAULT,
    };
    println!("코덱 {} {}", codec.id(), codec.name());

    let tx = audio::channel();
    let (_codec_tx, codec_rx) = watch::channel(codec);
    let (_encoding, encoded) = encoding::start(&tx, codec_rx);
    // 테스트용 고정 번호·열쇠. 아래 링크를 QR로 만들어 폰으로 찍는다
    let (id, key) = ([0x00, 0xc0, 0xff, 0xee], [0x11; 32]);
    println!("연결 링크: {}", link::connect_link(&id, &key, "send-example"));
    let server = ble::start(encoded, id, key).await?;
    let _capture = capture::start(tx)?;
    println!("송신 중. Ctrl+C로 종료");

    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = tick.tick() => println!("청취자 {}명", server.listeners()),
        }
    }
    server.stop().await;
    Ok(())
}
