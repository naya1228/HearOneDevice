import type { CodecInfo } from "../components/CodecSelect";

export type Status = { running: boolean; listeners: number; link: string; name: string; codec: number };

// 세로·가로 화면이 똑같이 받는 값
export type LayoutProps = {
  status: Status;
  codecs: CodecInfo[];
  busy: boolean;
  error: string;
  onToggle: () => void;
  onCodecChange: (id: number) => void;
};
