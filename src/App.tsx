import { useEffect, useState, useRef } from "react";
import "./App.css";
import Button from "./components/Button";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const RTC_CONFIG: RTCConfiguration = {
  iceServers: [],
};

export function useIP() {
  const [ip, setIP] = useState<string>("");
  useEffect(() => {
    invoke<string>("get_ip").then(setIP);
  }, []);
  return ip;
}

function App() {
  const pcRef = useRef<RTCPeerConnection | null>(null);
  const [signaling_server, set_server] = useState("http://localhost:6767");

  useEffect(() => {
    invoke("capture_sound");

    const unlisten = listen<number[]>("audio-data", (e) => {
      console.log(e.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
      pcRef.current?.close();
      pcRef.current = null;
    };
  }, []);

  const handleConnect = async (e: React.ChangeEvent<HTMLFormElement>) => {
    e.preventDefault();
    const pc = new RTCPeerConnection(RTC_CONFIG);
    pcRef.current = pc;
    pc.onconnectionstatechange = () => {
      if (pc.connectionState === "disconnected") {
        pc.close();
        pcRef.current = null;
      }
    };

    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);
  };

  const handleOpenRoom = async () => {
    if (pcRef.current === null) {
    }
  };

  return (
    <main className="flex flex-col bg-[#1F1F1E] items-center p-3 h-dvh">
      <img className="rounded-md" src="sharing.svg" />
      <span className="text-white text-4xl font-bold m-2">
        SharingYourSounds
      </span>
      <div className="flex m-2 gap-2 items-center">
        <p className="text-gray-500">your local ip is {useIP()}</p>
        <Button type="button" onClick={handleOpenRoom}>
          Open Host & Wait
        </Button>
      </div>
      <span>or</span>
      <form className="flex gap-1.5 mb-10" onSubmit={handleConnect}>
        <input
          onChange={(e) => set_server(`http://${e.target.value}:6767`)}
          className="outline rounded-md placeholder:text-gray-500 placeholder:italic"
          type="text"
          placeholder={"ex) " + useIP()}
        ></input>
        <Button type="submit">Connect</Button>
      </form>
    </main>
  );
}

export default App;
