import { useEffect, useState } from "react";
import "./App.css";
import { invoke } from "@tauri-apps/api/core";
import type { CodecInfo } from "./components/CodecSelect";
import type { LayoutProps, Status } from "./layouts/LayoutProps";
import PortraitLayout from "./layouts/PortraitLayout";
import LandscapeLayout from "./layouts/LandscapeLayout";
import TinyLayout from "./layouts/TinyLayout";

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

  const props: LayoutProps = { status, codecs, busy, error, onToggle: toggle, onCodecChange: changeCodec };
  // 창 모양에 따라 하나만 보임: 아주 작으면 상태만, 아니면 높이가 폭보다 크면 세로
  return (
    <>
      <div className="landscape:hidden tiny:hidden">
        <PortraitLayout {...props} />
      </div>
      <div className="portrait:hidden tiny:hidden">
        <LandscapeLayout {...props} />
      </div>
      <div className="hidden tiny:block">
        <TinyLayout {...props} />
      </div>
    </>
  );
}

export default App;
