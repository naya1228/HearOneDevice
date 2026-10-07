import type { CodecInfo } from "./CodecSelect";

type Props = { codecs: CodecInfo[]; value: number };

// 고른 코덱에 주의 문구가 있으면 한 줄 (문구는 PC 코덱 정의 codec/mod.rs 에 있음)
function CodecWarning({ codecs, value }: Props) {
  const warning = codecs.find((c) => c.id === value)?.warning;
  if (!warning) return null;
  return <p className="text-amber-400 text-xs break-keep">⚠ {warning}</p>;
}

export default CodecWarning;
