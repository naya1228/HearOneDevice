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
  const [hostIp, setHostIp] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    const unlisten = listen<string>("rtc-status", (event) => {
      const s = event.payload;
      if (s === "connected") setStatus("connected");
      else if (s === "disconnected" || s === "failed" || s.startsWith("error")) {
        setStatus("failed");
        if (s.startsWith("error")) setError(s);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleOpenRoom = async () => {
    setError("");
    setStatus("waiting");
    try {
      await invoke("open_room");
    } catch (e) {
      setError(String(e));
      setStatus("failed");
    }
  };

  const handleConnect = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!hostIp.trim()) return;
    setError("");
    setStatus("connecting");
    try {
      await invoke("connect_to_host", { host_ip: hostIp.trim() });
    } catch (e) {
      setError(String(e));
      setStatus("failed");
    }
  };

  const handleDisconnect = async () => {
    try {
      await invoke("close_rtc");
    } catch (_) {}
    setStatus("idle");
    setError("");
  };

  return (
    <main className="flex flex-col bg-[#1F1F1E] items-center p-3 h-dvh">
      <img className="rounded-md" src="sharing.svg" />
      <span className="text-white text-4xl font-bold m-2">
        ShareYourSounds
      </span>
      <p className="text-gray-500 mb-4">your local ip is {ip}</p>

      {status === "idle" && (
        <>
          <Button type="button" onClick={handleOpenRoom}>
            Open Host & Wait
          </Button>
          <span className="text-gray-500 my-2">or</span>
          <form className="flex gap-1.5" onSubmit={handleConnect}>
            <input
              value={hostIp}
              onChange={(e) => setHostIp(e.target.value)}
              className="outline rounded-md px-2 placeholder:text-gray-500 placeholder:italic"
              type="text"
              placeholder={"ex) " + ip}
            />
            <Button type="submit">Connect</Button>
          </form>
        </>
      )}

      {status === "waiting" && (
        <div className="flex flex-col items-center">
          <p className="text-yellow-400 text-lg mb-2">
            Waiting for connection...
          </p>
          <p className="text-gray-400 text-sm mb-4">
            Share your IP ({ip}) with the other device
          </p>
          <Button type="button" onClick={handleDisconnect}>
            Cancel
          </Button>
        </div>
      )}

      {status === "connecting" && (
        <p className="text-yellow-400 text-lg">Connecting to {hostIp}...</p>
      )}

      {status === "connected" && (
        <div className="flex flex-col items-center">
          <p className="text-green-400 text-lg mb-4">Connected!</p>
          <Button type="button" onClick={handleDisconnect}>
            Disconnect
          </Button>
        </div>
      )}

      {status === "failed" && (
        <div className="flex flex-col items-center">
          <p className="text-red-400 text-lg mb-2">Connection failed</p>
          {error && <p className="text-red-300 text-sm mb-4">{error}</p>}
          <Button type="button" onClick={handleDisconnect}>
            Try Again
          </Button>
        </div>
      )}
    </main>
  );
}

export default App;
