// UI 없이 송신만 돌려보는 테스트용: cargo run --example send
use hear_one_device_lib::{audio, ble, capture};

#[tokio::main]
async fn main() -> Result<(), String> {
    let tx = audio::channel();
    let server = ble::start(tx.clone()).await?;
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
