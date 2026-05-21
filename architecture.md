```
PC (Tauri/Rust)  ──── WebRTC P2P ────  폰 브라우저 (GitHub Pages)
      │                                        │
  오디오 캡처                              오디오 재생
  webrtc-rs                            브라우저 내장 WebRTC
  PeerConnection                        RTCPeerConnection
      │
  STUN 서버 (1회, Google 무료)
  공인 IP:포트 확인

시그널링 (미해결)
  PC → offer → QR 표시 → 폰 스캔   ✅
  폰 → answer → PC 전달             ❓
```
