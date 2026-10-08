# HearOneDevice

PC 시스템 소리를 블루투스(BLE)로 안드로이드 폰에 보내 재생합니다.
이어폰은 폰에만 페어링해 두고 PC 소리를 들을 수 있습니다. 폰 화면이 꺼져도 재생이 이어집니다.

- PC 앱: Windows, Linux (PulseAudio 또는 PipeWire의 PulseAudio 호환, BlueZ)
- 폰 앱: Android 8.0 이상, BLE 지원 기기
- 인터넷·Wi-Fi가 필요 없습니다. PC와 폰이 블루투스로 직접 연결됩니다.

## 사용 방법

1. PC 앱을 실행하고 "공유 시작"을 누르면 QR 코드가 뜹니다.
2. 폰 앱에서 QR을 찍으면 그 PC가 기록에 저장되고 바로 재생됩니다.
3. 다음부터는 폰 앱에서 "마지막 PC로 듣기"나 "PC 연결 기록"에서 고르면 됩니다.

PC 블루투스 기기 목록에 폰이 등록돼 있으면 연결할 때 폰에 페어링 창이 뜰 수 있습니다(일부 Windows 블루투스 칩). 등록을 지우면 됩니다. 이 앱은 블루투스 페어링을 쓰지 않습니다.

## 보안

QR에 실린 열쇠로 연결할 때마다 PC와 폰이 서로를 확인하고, 확인을 통과한 폰에만 소리를 잠가서(AES-256-GCM) 보냅니다.
QR을 찍지 않은 폰은 소리를 들을 수 없고, 전파를 엿들어도 내용이 보이지 않습니다. 자세한 방식은 [docs/PROTOCOL.md](docs/PROTOCOL.md).

## 개발

PC 앱 (Tauri 2 + React):

```bash
npm install
npm run tauri dev      # 실행
npm run tauri build    # 설치 파일 만들기
cd src-tauri && cargo test
cd src-tauri && cargo run --example send   # 화면 없이 송신만
```

Linux 빌드에는 `libpulse`, `dbus`, Tauri 기본 의존성(webkit2gtk 4.1)의 개발 패키지가 필요합니다.

폰 앱 (Kotlin):

```bash
cd android
./gradlew assembleDebug
```

## 문서

- [docs/PROTOCOL.md](docs/PROTOCOL.md) — PC ↔ 폰 약속 (UUID, 광고, 패킷, 연결 확인, 잠그기)
- [docs/CODECS.md](docs/CODECS.md) — 코덱 번호 표 (Opus 64k 기본, 128k 고음질)
- [docs/UI.md](docs/UI.md) — PC 앱 화면 배치

이전 웹 버전(Cloudflare 터널, 폰 브라우저 수신)은 [v0.3.0](https://github.com/naya1228/HearOneDevice/releases/tag/v0.3.0)에 남아 있습니다.
