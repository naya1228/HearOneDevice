type Props = { running: boolean; listeners: number };

// 점 색으로 상태 구분: 회색 꺼짐 / 주황 깜빡임 기다리는 중 / 초록 연결됨
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
