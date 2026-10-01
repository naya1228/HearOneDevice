# 코덱 표

PC → 폰으로 보내는 소리 형식 목록입니다. 패킷 헤더 두 번째 바이트(`[1]`)에 아래 **번호**가 들어갑니다.

## 구현된 코덱

| 번호 | 이름 | 샘플레이트 | 채널 | 샘플당 | 대역폭 | PC 인코더 | 앱 디코더 | 비고 |
|---|---|---|---|---|---|---|---|---|
| **1** | μ-law 16kHz 모노 | 16 kHz | 모노 | 8 bit | 16 KB/s | `src-tauri/src/codec/ulaw.rs` | `Codecs.kt` `Ulaw16kMono` | 기본값. 2026-10-01 S25 실기기 확인 (손실 0) |

## 후보 (아직 번호 없음)

| 이름 | 예상 대역폭 | 메모 |
|---|---|---|
| PCM 16bit 16kHz 모노 | 32 KB/s | 압축 없음. μ-law와 음질 비교용 |
| μ-law 24/32kHz 모노 | 24/32 KB/s | 고음역 개선. BLE notify 대역폭 한계 확인 필요 |
| Opus | 4~16 KB/s | 음질↑ 대역폭↓. 프레임 단위라 `Packetizer` 쪼개기 방식 변경 필요. L2CAP와 같이 검토 |

번호는 한 번 쓰면 **재사용하지 않습니다**. 실험하다 버린 코덱도 번호는 비워 둡니다.

---

## 코덱 바꾸는 법

양쪽 번호를 **같게** 맞추고 둘 다 다시 빌드합니다.

| 쪽 | 파일 | 바꿀 곳 |
|---|---|---|
| PC | `src-tauri/src/lib.rs` | `pub const CODEC: codec::Codec = codec::Codec::Ulaw16kMono;` |
| 앱 | `android/.../Codecs.kt` | `const val ACTIVE = ULAW_16K_MONO` |

μ-law로 되돌릴 때도 위 두 줄을 `Ulaw16kMono` / `ULAW_16K_MONO`로 바꾸면 됩니다.

번호가 다르면 앱 화면에 `코덱 불일치: PC N번, 앱 M번`이 뜨고 소리는 나지 않습니다.
맞으면 `재생 중 · 코덱 1 μ-law 16kHz 모노 · ...`처럼 지금 쓰는 코덱이 표시됩니다.
PC 쪽은 시작할 때 터미널에 `[ble] 광고 시작: ... (코덱 1 μ-law 16kHz 모노)`가 찍힙니다.

## 코덱 추가하는 법

1. **PC** `src-tauri/src/codec/`
   - 새 파일(예: `pcm.rs`)에 `Encoder` 트레이트를 구현합니다. 리샘플이 필요하면 `resample::Resampler`를 같이 씁니다.
   - `mod.rs`의 `Codec`에 번호를 추가하고 `name()`, `encoder()`에 연결합니다.
2. **앱** `Codecs.kt`
   - 같은 번호로 `const val`을 추가하고, `Decoder`(`sampleRate`, `decode`)를 구현한 뒤 `decoder()`에 연결합니다.
3. **이 표**에 한 줄 추가합니다.
4. 양쪽의 `CODEC` / `ACTIVE`를 새 번호로 바꿔 테스트합니다.
