# HearOneDevice

Windows 또는 Linux PC의 시스템 오디오를 같은 네트워크의 다른 기기에서 브라우저로 들을 수 있게 해주는 Tauri 앱입니다.

## 지원 플랫폼

- Host app: Windows, Linux
- Receiver: WebSocket과 Web Audio를 지원하는 브라우저

Android 앱 빌드는 더 이상 지원하지 않습니다.

## 개발

```bash
npm install
npm run tauri dev
```

## 빌드

```bash
npm run tauri build
```

## Linux 방화벽

다른 기기에서 수신 페이지에 접속하려면 TCP 6767 포트를 열어야 합니다.

```bash
sudo ufw allow 6767/tcp
```

또는 firewalld를 사용하는 경우:

```bash
sudo firewall-cmd --add-port=6767/tcp --permanent
sudo firewall-cmd --reload
```

## 기술 스택

- Frontend: React, TypeScript, Vite
- Backend: Rust, Tauri 2
- Audio capture: cpal on Windows, PulseAudio on Linux
- Streaming: WebSocket
