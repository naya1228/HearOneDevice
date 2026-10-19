import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateState = {
  version: string | null;
  installing: boolean;
  error: string;
  install: () => void;
};

// docs/FEATURES.md: 업데이트
export function useUpdate(): UpdateState {
  const [update, setUpdate] = useState<Update | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    check().then(setUpdate).catch(() => {});
  }, []);

  const install = async () => {
    if (!update) return;
    setError("");
    setInstalling(true);
    try {
      await invoke("stop_sharing");
      await update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      setError(String(e));
      setInstalling(false);
    }
  };

  return { version: update?.version ?? null, installing, error, install };
}
