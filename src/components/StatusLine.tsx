type Props = { running: boolean; listeners: number };

// docs/FEATURES.md: PC 화면
function StatusLine({ running, listeners }: Props) {
  const [text, color, dot] = !running
    ? ["공유 꺼짐", "text-gray-400", "bg-gray-500"]
    : listeners > 0
      ? [`폰 ${listeners}대 연결됨`, "text-green-400", "bg-green-400"]
      : ["연결 기다리는 중...", "text-[#FD6000]", "bg-[#FD6000] animate-pulse"];
  return (
    <p className={`flex items-center gap-2 ${color}`}>
      <span className={`size-2 rounded-full ${dot}`} />
      {text}
    </p>
  );
}

export default StatusLine;
