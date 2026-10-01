// UI 없이 송신만 돌려보는 테스트용: cargo run --example send
use hear_one_device_lib::{audio, ble, capture, start_encoding, CODEC};

#[tokio::main]
async fn main() -> Result<(), String> {
    let tx = audio::channel();
    // 테스트용 고정 번호. 폰에서 hearone://connect?id=00c0ffee 로 연결
    let (_encoding, encoded) = start_encoding(&tx);
    let server = ble::start(encoded, CODEC, [0x00, 0xc0, 0xff, 0xee]).await?;
    let _capture = capture::start(tx)?;
    println!("송신 중. Ctrl+C로 종료");

    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = tick.tick() => println!("청취자 {}명", server.listeners()),
        }
    }
    Ok(())
}
