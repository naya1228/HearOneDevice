const SAMPLE_RATE = 48000;
const CHANNELS = 2;
const MAX_AHEAD_SEC = 0.06;

let audioCtx = null;
let mediaEl = null;       // AudioContext 출력을 받는 <audio> — Android Audio Focus 획득용
let streamDest = null;
let ws = null;
let nextPlayTime = 0;

function connect() {
  audioCtx = new AudioContext({ sampleRate: SAMPLE_RATE });

  // audioCtx.destination → 실제 스피커 출력 (Chrome이 AudioContext를 "audible"로 인식해 백그라운드 suspend 방지)
  // streamDest → <audio> 엘리먼트 (Android Audio Focus 획득용, 음소거)
  streamDest = audioCtx.createMediaStreamDestination();
  mediaEl = document.getElementById('media-sink');
  mediaEl.srcObject = streamDest.stream;
  mediaEl.volume = 0;  // 실제 소리는 audioCtx.destination에서 나오므로 중복 방지
  mediaEl.play();

  if ('mediaSession' in navigator) {
    navigator.mediaSession.metadata = new MediaMetadata({
      title: 'ShareYourSounds',
      artist: 'Live Audio',
    });
    navigator.mediaSession.playbackState = 'playing';
    navigator.mediaSession.setActionHandler('play', () => {
      audioCtx?.resume();
      mediaEl?.play();
      navigator.mediaSession.playbackState = 'playing';
    });
    navigator.mediaSession.setActionHandler('pause', () => disconnect());
    navigator.mediaSession.setActionHandler('stop', () => disconnect());
  }

  nextPlayTime = 0;

  ws = new WebSocket(`ws://${window.location.host}/audio`);
  ws.binaryType = 'arraybuffer';
  ws.onopen = () => showListening();

  ws.onmessage = (e) => {
    const now = audioCtx.currentTime;
    if (nextPlayTime > now + MAX_AHEAD_SEC) return;

    const f32 = new Float32Array(e.data);
    const frameCount = Math.floor(f32.length / CHANNELS);
    if (frameCount === 0) return;

    const buffer = audioCtx.createBuffer(CHANNELS, frameCount, SAMPLE_RATE);
    for (let ch = 0; ch < CHANNELS; ch++) {
      const channelData = buffer.getChannelData(ch);
      for (let i = 0; i < frameCount; i++) {
        channelData[i] = f32[i * CHANNELS + ch];
      }
    }

    const source = audioCtx.createBufferSource();
    source.buffer = buffer;
    source.connect(audioCtx.destination); // 실제 스피커 — Chrome이 audible로 인식해 백그라운드 suspend 안 함
    source.connect(streamDest);           // Android Audio Focus용 <audio> 엘리먼트 피드

    if (nextPlayTime < now) nextPlayTime = now + 0.01;
    source.start(nextPlayTime);
    nextPlayTime += buffer.duration;
  };

  ws.onclose = disconnect;
  ws.onerror = disconnect;
}

document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible' && audioCtx?.state === 'suspended') {
    audioCtx.resume();
  }
});

function disconnect() {
  if (ws) { ws.close(); ws = null; }
  if (mediaEl) { mediaEl.srcObject = null; }
  if (audioCtx) { audioCtx.close(); audioCtx = null; }
  streamDest = null;
  if ('mediaSession' in navigator) navigator.mediaSession.playbackState = 'none';
  showIdle();
}

function showIdle() {
  document.getElementById('idle').classList.remove('hidden');
  const el = document.getElementById('listening');
  el.classList.add('hidden');
  el.classList.remove('flex');
}

function showListening() {
  document.getElementById('idle').classList.add('hidden');
  const el = document.getElementById('listening');
  el.classList.remove('hidden');
  el.classList.add('flex');
}
