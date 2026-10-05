import { useEffect, useState } from "react";
import "./App.css";
import Button from "./components/Button";
import { invoke } from "@tauri-apps/api/core";
import QRCode from "react-qr-code";

type Status = { running: boolean; listeners: number; link: string; codec: number };
type CodecInfo = { id: number; name: string };

function App() {
  const [status, setStatus] = useState<Status>({ running: false, listeners: 0, link: "", codec: 0 });
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
    <main className="flex flex-col bg-[#1F1F1E] items-center p-3 h-dvh">
      <img className="rounded-md w-32 mt-4" src="sharing.svg" />
      <span className="text-white text-4xl font-bold m-2">HearOneDevice</span>
      <p className="text-gray-500 mb-6">PC 소리를 블루투스로 폰에 보냅니다</p>

      <Button type="button" onClick={busy ? undefined : toggle}>
        {busy ? "..." : status.running ? "공유 중지" : "공유 시작"}
      </Button>

      <label className="flex items-center gap-2 mt-4 text-gray-400 text-sm">
        코덱
        <select
          className="bg-[#2A2A29] text-white rounded-md px-2 py-1"
          value={status.codec}
          onChange={(e) => changeCodec(Number(e.target.value))}
        >
          {codecs.map((c) => (
            <option key={c.id} value={c.id}>
              {c.id}. {c.name}
            </option>
          ))}
        </select>
      </label>

      {status.running && (
        <div className="flex flex-col items-center gap-2 mt-6">
          <div className="bg-white p-3 rounded-md">
            <QRCode value={status.link} size={160} />
          </div>
          <p className="text-gray-400 text-sm">폰 앱이나 카메라로 QR을 찍으면 연결됩니다</p>
          <p className="text-[#FD6000]">
            {status.listeners > 0 ? `폰 ${status.listeners}대 연결됨` : "연결 기다리는 중..."}
          </p>
        </div>
      )}
      {error && <p className="text-red-400 text-sm mt-4 max-w-sm text-center break-all">{error}</p>}
    </main>
  );
}

export default App;
