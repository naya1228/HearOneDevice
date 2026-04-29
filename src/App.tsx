import { useEffect, useState, useRef } from "react";
import "./App.css";
import Button from "./components/Button";
import { invoke } from "@tauri-apps/api/core";

export function useIP() {
  const [ip, setIP] = useState<string>("");
  useEffect(() => {
    invoke<string>("get_ip").then(setIP);
  }, []);
  return ip;
}

function App() {
  const handleConnect = async (e: React.ChangeEvent<HTMLFormElement>) => {};

  const handleOpenRoom = async () => {
    invoke("capture_sound");
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
