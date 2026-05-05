import { useEffect, useState } from "react";
import "./App.css";
import Button from "./components/Button";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import QRCode from "react-qr-code";

function useIP() {
  const [ip, setIP] = useState("");
  useEffect(() => {
    invoke<string>("get_ip").then(setIP);
  }, []);
  return ip;
}

type Status = "idle" | "waiting" | "connecting" | "connected" | "failed";

function App() {
  const ip = useIP();
  const [status, setStatus] = useState<Status>("idle");
  //const [hostIp, setHostIp] = useState("");
  const [_error, setError] = useState("");

  useEffect(() => {
    const unlisten = listen<string>("rtc-status", (event) => {
      const s = event.payload;
      if (s === "connected") setStatus("connected");
      else if (
        s === "disconnected" ||
        s === "failed" ||
        s.startsWith("error")
      ) {
        setStatus("failed");
        if (s.startsWith("error")) setError(s);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const [receiverUrl, setReceiverUrl] = useState("");

  const handleOpenRoom = async () => {
    setError("");
    setStatus("waiting");
    try {
      const url = await invoke<string>("open_room");
      setReceiverUrl(url);
      await invoke("capture_sound");
    } catch (e) {
      setError(String(e));
      setStatus("failed");
    }
  };

  const handleDisconnect = async () => {};

  return (
    <main className="flex flex-col bg-[#1F1F1E] items-center p-3 h-dvh">
      <img className="rounded-md" src="sharing.svg" />
      <span className="text-white text-4xl font-bold m-2">ShareYourSounds</span>
      <p className="text-gray-500 mb-4">your local ip is {ip}</p>

      <div className="text-xs text-[#C8C7C0] bg-[#2A2A29] rounded-md p-3 mb-4 max-w-sm text-center leading-relaxed">
        방화벽에서 TCP <span className="text-[#FD6000] font-mono">6767</span> 포트를 열어야 모바일에서 접속할 수 있습니다.<br />
        <span className="font-mono mt-1 block">ufw allow 6767/tcp</span>
        <span className="font-mono">firewall-cmd --add-port=6767/tcp --permanent</span>
      </div>

      {status === "idle" && (
        <>
          <Button type="button" onClick={handleOpenRoom}>
            Open Host & Wait
          </Button>
        </>
      )}

      {status === "waiting" && (
        <div className="flex flex-col items-center">
          <p className="text-yellow-400 text-lg mb-2">
            Waiting for connection...
          </p>
          <p className="text-gray-400 text-sm mb-2 text-center">
            Open this URL on your mobile device:
          </p>
          <p className="text-white font-mono bg-black p-2 rounded mb-4 break-all">
            {receiverUrl}
          </p>
          <QRCode value={receiverUrl} />
          <Button type="button" onClick={handleDisconnect}>
            Cancel
          </Button>
        </div>
      )}
    </main>
  );
}

export default App;
