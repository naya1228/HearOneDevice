import { useEffect, useState } from "react";
import "./App.css";
import { invoke } from "@tauri-apps/api/core";
import type { CodecInfo } from "./components/CodecSelect";
import type { LayoutProps, Status } from "./layouts/LayoutProps";
import PortraitLayout from "./layouts/PortraitLayout";
import LandscapeLayout from "./layouts/LandscapeLayout";
import TinyLayout from "./layouts/TinyLayout";
import { useUpdate } from "./useUpdate";

const STATUS_REFRESH_MS = 1000;

// docs/FEATURES.md: PC 화면
function App() {
  const [status, setStatus] = useState<Status>({ running: false, listeners: 0, link: "", name: "", codec: 0 });
  const [codecs, setCodecs] = useState<CodecInfo[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const update = useUpdate();

  useEffect(() => {
    invoke<CodecInfo[]>("codecs").then(setCodecs);
  }, []);

  useEffect(() => {
    const refresh = () => invoke<Status>("sharing_status").then(setStatus);
    refresh();
    const id = setInterval(refresh, STATUS_REFRESH_MS);
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

  const changeCodec = async (id: number) => {
    setError("");
    try {
      await invoke("set_codec", { id });
      setStatus(await invoke<Status>("sharing_status"));
    } catch (e) {
      setError(String(e));
    }
  };

  const props: LayoutProps = { status, codecs, busy, error, onToggle: toggle, onCodecChange: changeCodec, update };
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
