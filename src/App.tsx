import { useEffect, useState } from "react";
import "./App.css";
import Button from "./components/Button";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
          <Button type="button" onClick={handleDisconnect}>
            Cancel
          </Button>
        </div>
      )}
    </main>
  );
}

export default App;
