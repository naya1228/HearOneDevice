// 연결 확인·잠그기: QR로 건넨 열쇠로 PC와 폰이 연결할 때마다 서로 확인하고, 이번 연결의 소리 열쇠로 패킷을 잠근다.
// 전송(ble.rs·ble_windows.rs)과 무관한 계산만 여기 둔다. 절차·형식은 docs/PROTOCOL.md 5·6절

mod handshake;
mod key;
mod seal;

pub use handshake::{Handshake, Passed, REPLY_LEN, REPLY_TIMEOUT};
pub use key::{key_hex, load_key, Key};
pub use seal::{Sealer, TAG_LEN};
