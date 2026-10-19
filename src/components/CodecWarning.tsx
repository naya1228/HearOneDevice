import type { CodecInfo } from "./CodecSelect";

type Props = { codecs: CodecInfo[]; value: number };

// docs/FEATURES.md: PC 화면
function CodecWarning({ codecs, value }: Props) {
  const warning = codecs.find((c) => c.id === value)?.warning;
  if (!warning) return null;
  return <p className="text-amber-400 text-xs break-keep">⚠ {warning}</p>;
}

export default CodecWarning;
