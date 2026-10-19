// docs/PROTOCOL.md: 연결 확인, 잠그기
mod handshake;
mod key;
mod seal;

pub use handshake::{Handshake, Passed, REPLY_LEN, REPLY_TIMEOUT};
pub use key::{key_hex, load_key, Key};
pub use seal::{Sealer, TAG_LEN};
