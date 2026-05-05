# ShareYourSounds

버즈 하나로 2~3개 디바이스의 소리를 동시에 듣는 리눅스 앱

## 왜 만들었나

블루투스 이어폰을 휴대폰에 연결하면, 노트북으로 유튜브 소리를 들을 수 없음

## 구조

```
노트북이나 데스크톱에서 앱 실행 → 앱에서 웹서버 실행→  모바일에서 로컬 웹 접속 → 웹소켓으로 오디오 재생
```

- **호스트**: 시스템 소리를 캡처해서 송출하는 기기

## 방화벽 설정 (Linux)

모바일 기기에서 접속하려면 TCP **6767** 포트를 열어야 합니다.

**ufw**
```bash
sudo ufw allow 6767/tcp
```

**firewalld**
```bash
sudo firewall-cmd --add-port=6767/tcp --permanent
sudo firewall-cmd --reload
```

## 기술 스택

- **Frontend**: React + TypeScript (Vite)
- **Backend**: Rust (Tauri 2)
- **오디오 캡처**: PulseAudio (Linux) / cpal (Windows)
- **스트리밍**: WebSocket
