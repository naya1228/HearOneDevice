# HearOneDevice

**Hear One Device** — keep your earbuds paired to one device, and hear the rest through it.

[English](#english) · [한국어](#한국어)

---

## English

HearOneDevice streams your PC's system audio to your Android phone over Bluetooth LE. Your earbuds stay paired to the phone only, and PC audio plays through the phone, mixed with the phone's own sounds. Playback keeps going with the phone screen off.

### Why

- **Pairing per PC.** A laptop, a desktop, and each OS on a dual-boot machine all need their own pairing with the same earbuds.
- **Multipoint auto-switching.** With earbuds connected to both PC and phone, one phone notification can pull the audio away from the PC.
- **Only one source at a time.** I wanted to hear the PC and the phone together (a video on the PC plus notifications and calls on the phone).

The earlier web version ([v0.3.0](https://github.com/naya1228/HearOneDevice/releases/tag/v0.3.0): Cloudflare tunnel + phone browser) needed an internet connection, and a browser can't guarantee playback with the screen off (and Web Bluetooth doesn't work on iOS either). So this version uses a native Android app and a direct Bluetooth link.

I use it every day: a laptop at school and in the library (crowded 2.4 GHz), and a desktop at home (2–3 hours of YouTube in one sitting with no audible dropouts).

### Requirements

- PC app: Windows, Linux (PulseAudio, or PipeWire's PulseAudio compatibility; BlueZ)
- Phone app: Android 8.0+ with BLE (iOS is not supported)
- No internet or Wi-Fi needed — the PC and phone talk directly over Bluetooth.

### Usage

Download the PC installer and the `.apk` from [Releases](https://github.com/naya1228/HearOneDevice/releases).

1. Open the PC app and press **공유 시작** (Start sharing). The app UI is in Korean. A QR code appears.
2. Scan it with the phone app. The PC is saved to the phone's history and audio starts playing.
3. Next time, pick **마지막 PC로 듣기** (listen to last PC) or choose from **PC 연결 기록** (PC history) in the phone app.

If the phone is registered as a Bluetooth device on the PC, a pairing prompt may pop up on the phone when connecting (seen with some Windows Bluetooth chips). Removing the phone from the PC's device list fixes it. HearOneDevice does not use Bluetooth pairing.

### How it works

```
PC                                                         Android phone
system audio ─► Opus encode ─► AES-256-GCM ─► BLE notify ─► decrypt ─► MediaCodec ─► AudioTrack ─► earbuds
(PulseAudio monitor /           (per-connection            (jitter buffer, foreground service)
 WASAPI loopback)                audio key)
```

- **Discovery:** the PC advertises a service UUID. The phone treats every PC advertising it as a candidate and connects, and the key check (below) tells it which one is yours. It tries the last PC that passed first, then by signal strength.
- **Codec:** Opus 48 kHz stereo — 64 kbps by default (less airtime, better in crowded places), 128 kbps as a high-quality option. Switchable live from the PC app; the phone picks the decoder from the packet header.
- **Measured (Galaxy S25, Android 16):** MTU 517, 0 packet loss, playback continues with the screen off. The jitter buffer targets 80 ms, favoring low latency over riding out bursts.
- **Stop:** when the PC stops sharing it sends a control packet, so the phone stops instead of searching again. The phone plays a short chime on connect and disconnect, so you know without looking at the screen.

### Security

The QR code carries a key that is handed over once. On every connection the PC and phone verify each other with it, and audio is sent only to a verified phone, encrypted with AES-256-GCM. A phone that never scanned the QR can't listen, and sniffing the radio reveals nothing. Details in [docs/PROTOCOL.md](docs/PROTOCOL.md).

Accepted risk: while sharing, the fixed service UUID makes the PC's presence visible nearby.

### Development

PC app (Tauri 2 + React):

```bash
npm install
npm run tauri dev      # run
npm run tauri build    # build installer
cd src-tauri && cargo test
cd src-tauri && cargo run --example send   # send only, no UI
```

Building on Linux needs the dev packages for `libpulse`, `dbus`, and Tauri's base dependencies (webkit2gtk 4.1).

Phone app (Kotlin):

```bash
cd android
./gradlew assembleDebug
```

Code layout and rules are in [CLAUDE.md](CLAUDE.md). One file does one thing; capture, codec, and transport only pass bytes/samples to each other.

### Docs (Korean)

- [docs/PROTOCOL.md](docs/PROTOCOL.md) — PC ↔ phone protocol (UUIDs, advertising, packets, key check, encryption)
- [docs/CODECS.md](docs/CODECS.md) — codec table (Opus 64k default, 128k high quality)
- [docs/UI.md](docs/UI.md) — PC app layout

### License

[MIT](LICENSE)

---

## 한국어

HearOneDevice는 PC 시스템 소리를 블루투스(BLE)로 안드로이드 폰에 보내 재생합니다. 이어폰은 폰에만 페어링해 두면 되고, PC 소리는 폰 소리와 섞여서 들립니다. 폰 화면이 꺼져도 재생이 이어집니다.

### 왜 만들었나

- **PC마다 페어링.** 노트북, 데스크탑, 듀얼부팅의 OS마다 같은 이어폰을 따로 페어링해야 했습니다.
- **멀티포인트 자동 전환.** 이어폰을 PC와 폰에 같이 연결해 두면 폰 알림 하나에 소리가 폰으로 넘어갔습니다.
- **소리는 한 번에 하나.** PC와 폰 소리를 같이 듣고 싶었습니다 (PC로 영상 보면서 폰 알림·통화).

이전 웹 버전([v0.3.0](https://github.com/naya1228/HearOneDevice/releases/tag/v0.3.0): Cloudflare 터널 + 폰 브라우저)은 인터넷이 필요했고, 브라우저로는 화면이 꺼졌을 때 재생을 보장할 수 없었습니다(Web Bluetooth는 iOS도 미지원). 그래서 이 버전은 안드로이드 네이티브 앱과 블루투스 직접 연결로 다시 만들었습니다.

매일 쓰고 있습니다: 학교·도서관(혼잡한 2.4GHz)에서는 노트북으로, 집에서는 데스크탑으로 (유튜브 2~3시간 연속, 귀로 들리는 끊김 없음).

### 필요한 것

- PC 앱: Windows, Linux (PulseAudio 또는 PipeWire의 PulseAudio 호환, BlueZ)
- 폰 앱: Android 8.0 이상, BLE 지원 기기 (iOS 미지원)
- 인터넷·Wi-Fi가 필요 없습니다. PC와 폰이 블루투스로 직접 연결됩니다.

### 사용 방법

[Releases](https://github.com/naya1228/HearOneDevice/releases)에서 PC 설치 파일과 `.apk`를 받습니다.

1. PC 앱을 실행하고 "공유 시작"을 누르면 QR 코드가 뜹니다.
2. 폰 앱에서 QR을 찍으면 그 PC가 기록에 저장되고 바로 재생됩니다.
3. 다음부터는 폰 앱에서 "마지막 PC로 듣기"나 "PC 연결 기록"에서 고르면 됩니다.

PC 블루투스 기기 목록에 폰이 등록돼 있으면 연결할 때 폰에 페어링 창이 뜰 수 있습니다(일부 Windows 블루투스 칩). 등록을 지우면 됩니다. 이 앱은 블루투스 페어링을 쓰지 않습니다.

### 어떻게 동작하나

```
PC                                                         안드로이드 폰
시스템 소리 ─► Opus 인코딩 ─► AES-256-GCM ─► BLE 알림 ─► 풀기 ─► MediaCodec ─► AudioTrack ─► 이어폰
(PulseAudio monitor /           (연결마다 새            (지터 버퍼, 포그라운드 서비스)
 WASAPI 루프백)                  소리 열쇠)
```

- **찾기:** PC는 서비스 UUID를 광고합니다. 폰은 그 UUID를 광고하는 PC를 모두 후보로 보고 붙어서, 아래 열쇠 확인으로 내 PC를 가립니다. 마지막으로 통과한 PC부터, 그다음은 신호 센 순서로 시도합니다.
- **코덱:** Opus 48kHz 스테레오. 기본 64kbps(전파를 덜 써서 혼잡한 곳에 유리), 고음질 옵션 128kbps. PC 앱에서 재생 중에 바꿀 수 있고, 폰은 패킷 헤더를 보고 디코더를 고릅니다.
- **실측 (Galaxy S25, Android 16):** MTU 517, 손실 0, 화면 꺼짐에도 재생 유지. 지터 버퍼 목표는 80ms로, 몰려 오는 지연을 버티기보다 지연이 짧은 쪽을 택했습니다.
- **멈춤:** PC가 공유를 끝내면 제어 패킷을 보내, 폰이 다시 찾지 않고 멈춥니다. 폰은 연결·해제 때 짧은 알림음을 내서 화면을 안 봐도 알 수 있습니다.

### 보안

QR에 실린 열쇠를 한 번 건네고, 연결할 때마다 PC와 폰이 그 열쇠로 서로를 확인합니다. 확인을 통과한 폰에만 소리를 잠가서(AES-256-GCM) 보냅니다. QR을 찍지 않은 폰은 소리를 들을 수 없고, 전파를 엿들어도 내용이 보이지 않습니다. 자세한 방식은 [docs/PROTOCOL.md](docs/PROTOCOL.md).

받아들인 위험: 공유하는 동안에는 고정된 서비스 UUID 때문에 근처에 그 PC가 있다는 것은 드러납니다.

### 개발

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

코드 구조와 규칙은 [CLAUDE.md](CLAUDE.md)에 있습니다. 한 파일은 한 가지 기능만 하고, 캡처·코덱·전송은 바이트/샘플만 주고받습니다.

### 문서

- [docs/PROTOCOL.md](docs/PROTOCOL.md) — PC ↔ 폰 약속 (UUID, 광고, 패킷, 연결 확인, 잠그기)
- [docs/CODECS.md](docs/CODECS.md) — 코덱 번호 표 (Opus 64k 기본, 128k 고음질)
- [docs/UI.md](docs/UI.md) — PC 앱 화면 배치

### 라이선스

[MIT](LICENSE)
