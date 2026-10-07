// 연결 확인: QR로 건넨 열쇠로 PC와 폰이 연결할 때마다 서로 확인한다.
// 전송(ble.rs·ble_windows.rs)과 무관한 계산만 여기 둔다. 절차·형식은 docs/PROTOCOL.md 5절

mod handshake;
mod key;

pub use handshake::{Handshake, REPLY_LEN, REPLY_TIMEOUT};
pub use key::{key_hex, load_key, Key};
