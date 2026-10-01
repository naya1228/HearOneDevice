# HearOneDevice — 작업 메모

다음 세션의 Claude를 위한 인수인계서. **짧게 유지할 것.**

## 개요
PC 시스템 소리 → 블루투스(BLE) → 안드로이드 앱에서 재생. 폰 화면이 꺼져도 재생 유지가 목표.
(웹/Cloudflare 터널 버전은 `main` 브랜치. 이 계열은 `cd04425`에서 갈라져 새로 만든 것)

## 구조
- `src-tauri/src/capture/` — 소리 캡처 (linux: PulseAudio monitor, windows: cpal 루프백)
- `src-tauri/src/codec.rs` — 16kHz 모노 μ-law 압축 + 패킷 헤더 `[버전|코덱|순번 u16 LE|데이터]`
- `src-tauri/src/ble.rs` — BLE 송신 (Linux/BlueZ). Windows는 `ble_unsupported.rs` (미구현)
- `src-tauri/examples/send.rs` — UI 없이 송신 테스트: `cargo run --example send`
- `android/` — 수신 앱 (Kotlin, 외부 라이브러리 없음). `AudioService`(포그라운드 서비스)가 BLE 수신+재생
- **UUID·패킷 형식은 `codec.rs`/`ble.rs` ↔ `Protocol.kt` 양쪽이 같아야 함**

## 확정된 결정
- 브라우저(Web Bluetooth)는 iOS 미지원·화면 꺼짐 보장 불가 → 폰 쪽은 네이티브 앱.
- BLE 대역폭 때문에 μ-law 16kHz 모노(16KB/s)로 시작. 음질 개선은 L2CAP + Opus 후보.
- 안드로이드 빌드는 RealHunter와 같은 AGP 9.4.0 / Gradle 9.6.0 (Kotlin 내장, 플러그인 불필요).
- 2026-10-01 Galaxy S25(Android 16)에서 실기기 확인: MTU 517, 손실 0, 화면 꺼짐 60초 재생 유지. 버퍼가 260ms까지 쌓여 지연 개선 여지 있음.

## 기록 규칙
- "무엇이/언제" → Stop hook이 자동 기록. "왜" → 위 "확정된 결정"에 한 줄 append.
