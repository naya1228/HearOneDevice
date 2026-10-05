# 코덱 표

PC → 폰으로 보내는 소리 형식 목록입니다. 패킷 헤더 두 번째 바이트(`[1]`)에 아래 **번호**가 들어갑니다. 패킷 형식은 [PROTOCOL.md](PROTOCOL.md).

## 구현된 코덱

| 번호 | 이름 | 샘플레이트 | 채널 | 샘플당 | 대역폭 | PC 인코더 | 앱 디코더 | 비고 |
|---|---|---|---|---|---|---|---|---|
| **1** | Opus 48kHz 스테레오 128kbps | 48 kHz | 스테레오 | 가변 | 16 KB/s | `src-tauri/src/codec/opus.rs` | `Codecs.kt` `OpusStereo` (MediaCodec) | **기본값.** 20ms 프레임, 최대 480B. 2026-10-05 S25 확인 |
| **2** | Opus 48kHz 스테레오 64kbps | 48 kHz | 스테레오 | 가변 | 8 KB/s | `src-tauri/src/codec/opus.rs` | `Codecs.kt` `OpusStereo` (MediaCodec) | 20ms 프레임, 최대 480B. 2026-10-05 S25 확인 |
| **3** | ADPCM 32kHz 스테레오 | 32 kHz | 스테레오 | 4 bit | 약 33 KB/s | `src-tauri/src/codec/adpcm.rs` | `Codecs.kt` `AdpcmStereo` | 10ms 프레임 332B → MTU 335 이상 필요. 2026-10-05 S25 확인 (손실 0) |
| **4** | ADPCM 48kHz 스테레오 | 48 kHz | 스테레오 | 4 bit | 약 49 KB/s | `src-tauri/src/codec/adpcm.rs` | `Codecs.kt` `AdpcmStereo` | 10ms 프레임 492B → MTU 495 이상 필요. 2026-10-05 S25 확인 (손실 0) |
| **5** | μ-law 16kHz 모노 | 16 kHz | 모노 | 8 bit | 16 KB/s | `src-tauri/src/codec/ulaw.rs` | `Codecs.kt` `Ulaw16kMono` | 2026-10-01 S25 실기기 확인 (손실 0) |

**0번은 코덱이 아니라 제어 메시지**입니다 ([PROTOCOL.md](PROTOCOL.md) 4절).

번호는 한 번 쓰면 **재사용하지 않습니다**. 실험하다 버린 코덱도 번호는 비워 둡니다.

---

## 코덱 바꾸는 법

**PC 앱 화면의 "코덱" 목록에서 고릅니다.** 공유 중에 바꿔도 다음 조각부터 바로 적용됩니다.
폰 앱은 패킷 헤더의 코덱 번호를 보고 맞는 디코더로 알아서 바꿉니다 (다시 빌드할 필요 없음).

- UI 없이 테스트: `cargo run --example send -- 1` (마지막 숫자가 코덱 번호, 빼면 기본 코덱)
- 앱 화면에 `재생 중 · 코덱 1 Opus 48kHz 스테레오 128kbps · ...`처럼 지금 받는 코덱이 표시됩니다.
- 앱이 모르는 번호가 오면 `PC가 보낸 코덱 N번을 이 앱이 모름. 앱 업데이트 필요`가 뜨고 소리는 나지 않습니다.
- 처음 켰을 때 코덱은 `codec/mod.rs`의 `Codec::DEFAULT`.

## 코덱 추가하는 법

1. **PC** `src-tauri/src/codec/`
   - 새 파일(예: `pcm.rs`)에 `Encoder` 트레이트를 구현합니다. 리샘플이 필요하면 `resample::Resampler`를 같이 씁니다.
   - `mod.rs`의 `Codec`에 번호를 추가하고 `ALL`, `name()`, `framed()`, `encoder()`에 연결합니다.
   - `framed()`: 프레임 단위 코덱(Opus 등)이면 `true` → 패킷 하나에 프레임 하나로 보냅니다 (`packet.rs`).
2. **앱** `Codecs.kt`
   - 같은 번호로 `const val`을 추가하고, `Decoder`(`sampleRate`, `decode`)를 구현한 뒤 `decoder()`에 연결합니다.
3. **이 표**에 한 줄 추가합니다.
4. PC 앱에서 새 코덱을 골라 테스트합니다.
