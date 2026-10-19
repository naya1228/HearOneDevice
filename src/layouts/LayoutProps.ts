import type { CodecInfo } from "../components/CodecSelect";
import type { UpdateState } from "../useUpdate";

export type Status = { running: boolean; listeners: number; link: string; name: string; codec: number };

export type LayoutProps = {
  status: Status;
  codecs: CodecInfo[];
  busy: boolean;
  error: string;
  onToggle: () => void;
  onCodecChange: (id: number) => void;
  update: UpdateState;
};
