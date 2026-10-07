import QRCode from "react-qr-code";

type Props = {
  running: boolean;
  listeners: number;
  link: string;
};

// 화면 가운데 상태 영역: 꺼져 있으면 안내, 공유 중이면 QR과 연결 상태
function SharePanel({ running, listeners, link }: Props) {
  if (!running) {
    return (
      <div className="flex flex-col items-center gap-3 text-center break-keep">
        <img className="w-28 opacity-40" src="sharing.svg" />
        <p className="text-white text-lg">공유가 꺼져 있습니다</p>
        <p className="text-gray-500 text-sm">공유를 시작하면 폰으로 찍을 QR이 나타납니다</p>
      </div>
    );
  }

  const connected = listeners > 0;
  return (
    <div className="flex flex-col items-center gap-3 text-center break-keep">
      <div className="bg-white p-3 rounded-lg">
        <QRCode
          value={link}
          size={256}
          viewBox="0 0 256 256"
          className="h-auto w-[clamp(120px,25vh,280px)] sm:w-[clamp(120px,45vh,320px)]"
        />
      </div>
      <p className="text-gray-400 text-sm">폰 앱이나 카메라로 QR을 찍으면 연결됩니다</p>
      <p className={`flex items-center gap-2 ${connected ? "text-green-400" : "text-[#FD6000]"}`}>
        <span
          className={`size-2 rounded-full ${connected ? "bg-green-400" : "bg-[#FD6000] animate-pulse"}`}
        />
        {connected ? `폰 ${listeners}대 연결됨` : "연결 기다리는 중..."}
      </p>
    </div>
  );
}

export default SharePanel;
