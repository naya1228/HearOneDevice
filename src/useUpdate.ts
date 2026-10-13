import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateState = {
  /** 새 버전 번호. 없거나 아직 확인 전이면 null */
  version: string | null;
  installing: boolean;
  error: string;
  install: () => void;
};

// 켤 때 한 번 GitHub 릴리스의 새 버전을 확인. 설치는 사용자가 버튼을 눌러야 함 (공유 중에 갑자기 끊기지 않게)
export function useUpdate(): UpdateState {
  const [update, setUpdate] = useState<Update | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    // 인터넷이 없으면 조용히 넘어감 (이 앱은 인터넷 없이도 쓰는 앱)
    check().then(setUpdate).catch(() => {});
  }, []);

  const install = async () => {
    if (!update) return;
    setError("");
    setInstalling(true);
    try {
      // 설치 프로그램이 앱을 바로 끝낼 수 있어서, 폰에 "공유 중지"를 먼저 알림
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
