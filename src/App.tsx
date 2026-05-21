import { useLayoutEffect, useRef, useState } from "react";
import "./App.css";
import Button from "./components/Button";
import { invoke } from "@tauri-apps/api/core";
import QRCode from "react-qr-code";

type Status = "idle" | "starting" | "waiting" | "failed";

function App() {
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState("");
  const [tunnelUrl, setTunnelUrl] = useState("");

  const rightColRef = useRef<HTMLDivElement>(null);
  const [qrSize, setQrSize] = useState(0);

  useLayoutEffect(() => {
    if (status !== "waiting") return;
    const el = rightColRef.current;
    if (!el) return;
    const observer = new ResizeObserver(() => setQrSize(el.offsetHeight));
    observer.observe(el);
    setQrSize(el.offsetHeight);
    return () => observer.disconnect();
  }, [status]);

  const handleOpenRoom = async () => {
    setError("");
    setStatus("starting");
    try {
      await invoke<number>("start_server");
      await invoke("capture_sound");
      const url = await invoke<string>("open_tunnel");
      setTunnelUrl(url);
      setStatus("waiting");
    } catch (e) {
      console.error("호스트 시작 실패:", e);
      setError(String(e));
      setStatus("failed");
      // 부분적으로 켜진 자원 정리
      try { await invoke("stop_capture"); } catch {}
      try { await invoke("close_tunnel"); } catch {}
      try { await invoke("stop_server"); } catch {}
    }
  };

  const handleDisconnect = async () => {
    try { await invoke("stop_capture"); } catch {}
    try { await invoke("close_tunnel"); } catch {}
    try { await invoke("stop_server"); } catch {}
    setTunnelUrl("");
    setStatus("idle");
  };

  return (
    <main className="flex flex-col bg-[#1F1F1E] items-center p-3 h-dvh">
      <img className="rounded-md" src="sharing.svg" />
      <span className="text-white text-4xl font-bold m-2">ShareYourSounds</span>
      <p className="text-gray-500 mb-4">Tunnel mode — works on any network</p>

      {status === "idle" && (
        <div className="text-xs text-[#C8C7C0] bg-[#2A2A29] rounded-md p-3 mb-4 max-w-sm text-center leading-relaxed">
          버튼을 누르면 Cloudflare 터널이 열리고,<br />
          외부에서 접속 가능한 임시 주소가 표시됩니다.
        </div>
      )}

      {status === "idle" && (
        <Button type="button" onClick={handleOpenRoom}>
          Open Host & Wait
        </Button>
      )}

      {status === "starting" && (
        <p className="text-[#FD6000] mt-4">터널 여는 중...</p>
      )}

      {status === "waiting" && (
        <div className="flex gap-3 w-full max-w-sm items-start">
          {qrSize > 0 && <QRCode value={tunnelUrl} size={qrSize} />}
          <div ref={rightColRef} className="flex flex-col gap-2 flex-1 min-w-0">
            <div className="bg-[#111110] rounded p-3">
              <p className="text-white font-mono text-xs break-all leading-relaxed">
                {tunnelUrl}
              </p>
            </div>
            <Button type="button" onClick={handleDisconnect}>
              Disconnect
            </Button>
          </div>
        </div>
      )}

      {status === "failed" && (
        <div className="flex flex-col items-center gap-3 mt-4">
          <p className="text-red-400 text-sm max-w-sm text-center break-all">
            {error || "연결 실패"}
          </p>
          <Button type="button" onClick={() => setStatus("idle")}>
            Try Again
          </Button>
        </div>
      )}
    </main>
  );
}

export default App;
