import { useEffect, useState } from "react";
import "./App.css";
import Button from "./components/Button";
import { invoke } from "@tauri-apps/api/core";
import QrBox from "./components/QrBox";
import StatusLine from "./components/StatusLine";
import CodecSelect, { type CodecInfo } from "./components/CodecSelect";

type Status = { running: boolean; listeners: number; link: string; name: string; codec: number };

function App() {
  const [status, setStatus] = useState<Status>({ running: false, listeners: 0, link: "", name: "", codec: 0 });
  const [codecs, setCodecs] = useState<CodecInfo[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<CodecInfo[]>("codecs").then(setCodecs);
  }, []);

  // 연결된 폰 수를 1초마다 갱신
  useEffect(() => {
    const refresh = () => invoke<Status>("sharing_status").then(setStatus);
    refresh();
    const id = setInterval(refresh, 1000);
    return () => clearInterval(id);
  }, []);

  const toggle = async () => {
    setError("");
    setBusy(true);
    try {
      await invoke(status.running ? "stop_sharing" : "start_sharing");
      setStatus(await invoke<Status>("sharing_status"));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  // 공유 중에 바꿔도 바로 적용됨. 폰은 받은 패킷의 코덱 번호를 보고 따라감
  const changeCodec = async (id: number) => {
    setError("");
    try {
      await invoke("set_codec", { id });
      setStatus(await invoke<Status>("sharing_status"));
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <main className="flex flex-col items-center justify-center gap-8 p-6 min-h-dvh">
      <header className="flex flex-col items-center text-center">
        <img className="w-44" src="sharing.svg" />
        <h1 className="text-white text-4xl font-bold mt-2">HearOneDevice</h1>
        <p className="text-gray-500 mt-1">PC 소리를 블루투스로 폰에 보냅니다</p>
        <p className="text-gray-600 text-sm mt-1">이 PC 이름: {status.name}</p>
      </header>

      <section className="flex gap-4 w-full max-w-md">
        <QrBox link={status.running ? status.link : null} />
        <div className="flex flex-col justify-between flex-1 min-w-0">
          <StatusLine running={status.running} listeners={status.listeners} />
          <CodecSelect codecs={codecs} value={status.codec} onChange={changeCodec} />
          <Button
            type="button"
            variant={status.running ? "secondary" : "primary"}
            disabled={busy}
            onClick={toggle}
          >
            {busy ? "..." : status.running ? "공유 중지" : "공유 시작"}
          </Button>
        </div>
      </section>

      {error && <p className="text-red-400 text-sm text-center break-all max-w-md">{error}</p>}
    </main>
  );
}

export default App;
