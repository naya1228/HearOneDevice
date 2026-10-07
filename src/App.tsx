import { useEffect, useState } from "react";
import "./App.css";
import Button from "./components/Button";
import { invoke } from "@tauri-apps/api/core";
import SharePanel from "./components/SharePanel";

type Status = { running: boolean; listeners: number; link: string; name: string; codec: number };
type CodecInfo = { id: number; name: string };

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
    <main className="flex flex-col bg-[#1F1F1E] p-4 gap-4 h-dvh">
      <header className="flex items-center gap-3">
        <img className="w-16" src="sharing.svg" />
        <div>
          <h1 className="text-white text-xl font-bold">HearOneDevice</h1>
          <p className="text-gray-500 text-sm">PC 소리를 블루투스로 폰에 보냅니다</p>
        </div>
      </header>

      <section className="flex-1 flex items-center justify-center bg-[#262625] rounded-xl p-4">
        <SharePanel running={status.running} listeners={status.listeners} link={status.link} />
      </section>

      <section className="bg-[#262625] rounded-xl divide-y divide-[#333332] text-sm">
        <div className="flex items-center justify-between px-4 py-2.5">
          <span className="text-gray-400">이 PC 이름</span>
          <span className="text-white">{status.name}</span>
        </div>
        <label className="flex items-center justify-between px-4 py-2">
          <span className="text-gray-400">코덱</span>
          {/* WebKitGTK는 기본 select를 GTK 모양(밝은 바탕)으로 그려서 appearance-none + 화살표 직접 그림 */}
          <span className="relative">
            <select
              className="appearance-none bg-[#333332] text-white rounded-md pl-3 pr-8 py-1.5"
              value={status.codec}
              onChange={(e) => changeCodec(Number(e.target.value))}
            >
              {codecs.map((c) => (
                <option key={c.id} value={c.id} className="bg-[#333332] text-white">
                  {c.id}. {c.name}
                </option>
              ))}
            </select>
            <span className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 text-xs">
              ▼
            </span>
          </span>
        </label>
      </section>

      {error && <p className="text-red-400 text-sm text-center break-all">{error}</p>}

      <Button
        type="button"
        variant={status.running ? "secondary" : "primary"}
        disabled={busy}
        onClick={toggle}
      >
        {busy ? "..." : status.running ? "공유 중지" : "공유 시작"}
      </Button>
    </main>
  );
}

export default App;
