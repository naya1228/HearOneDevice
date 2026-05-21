# HearOneDevice

Windows 또는 Linux PC의 시스템 오디오를 휴대폰 브라우저로 들을 수 있게 해주는 Tauri 앱입니다.
Cloudflare Quick Tunnel을 사용해 같은 네트워크가 아니어도(공공 WiFi, 다른 네트워크) 연결됩니다.

## 지원 플랫폼

- Host app: Windows, Linux
- Receiver: WebSocket과 Web Audio를 지원하는 브라우저

Android 앱 빌드는 더 이상 지원하지 않습니다.

## 동작 방식

1. 앱이 PC 내부에 작은 HTTP/WebSocket 서버를 띄움 (localhost:6767)
2. cloudflared가 그 서버를 임시 공개 URL로 노출 (`*.trycloudflare.com`)
3. 폰이 그 URL에 접속하면 동일 페이지에서 WebSocket으로 오디오를 받아 재생

방화벽 설정이 필요 없습니다. 모든 트래픽은 PC가 Cloudflare로 outbound로 연결한 통로를 통해 전달됩니다.

## 개발

```bash
npm install
npm run tauri dev
```

## 빌드

```bash
npm run tauri build
```

## 기술 스택

- Frontend: React, TypeScript, Vite
- Backend: Rust, Tauri 2, axum
- Audio capture: cpal on Windows, PulseAudio on Linux
- Streaming: WebSocket over Cloudflare Quick Tunnel
