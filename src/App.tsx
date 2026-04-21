import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import Button from "./components/Button";

function App() {
  const [ip, setIP] = useState("127.0.0.1");
  const [connect_ip, set_arrive] = useState("");

  const handleConnect = useCallback<React.SubmitEventHandler<HTMLFormElement>>(
    (e) => {
      e.preventDefault();
      console.log(connect_ip);
    },
    [connect_ip],
  );

  useEffect(() => {
    invoke<string>("get_ip", {}).then(setIP);
  }, []);

  return (
    <main className="flex flex-col bg-[#1F1F1E] items-center p-3 h-dvh">
      <img className="rounded-md" src="sharing.svg" />
      <span className="text-white text-4xl font-bold m-2">
        SharingYourSounds
      </span>
      <p className="m-3 text-gray-300">Current address is {ip}</p>
      <form className="flex gap-1.5 mb-10" onSubmit={handleConnect}>
        <input
          onChange={(e) => set_arrive(e.target.value)}
          className="outline rounded-md placeholder:text-gray-500 placeholder:italic"
          type="text"
          placeholder="ex)192.168.43.1"
        ></input>
        <Button type="submit">Connect</Button>
      </form>
    </main>
  );
}

export default App;
