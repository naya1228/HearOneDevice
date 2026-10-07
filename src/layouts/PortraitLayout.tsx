import QrBox from "../components/QrBox";
import StatusLine from "../components/StatusLine";
import CodecSelect from "../components/CodecSelect";
import ShareButton from "../components/ShareButton";
import type { LayoutProps } from "./LayoutProps";

// 세로 창: 작은 헤더, 가운데 QR·상태 카드, 아래 설정 카드와 넓은 버튼
function PortraitLayout({ status, codecs, busy, error, onToggle, onCodecChange }: LayoutProps) {
  return (
    <main className="flex flex-col p-4 gap-4 min-h-dvh w-full max-w-md mx-auto">
      <header className="flex items-center gap-3 short:hidden">
        <img className="w-14" src="sharing.svg" />
        <div>
          <h1 className="text-white text-xl font-bold">HearOneDevice</h1>
          <p className="text-gray-500 text-sm">PC 소리를 블루투스로 폰에 보냅니다</p>
        </div>
      </header>

      <section className="flex-1 flex flex-col items-center justify-center gap-3 bg-[#262625] rounded-xl p-4 text-center break-keep">
        <QrBox link={status.running ? status.link : null} size="size-[clamp(120px,30vh,280px)]" />
        {status.running && <p className="text-gray-400 text-sm">폰 앱이나 카메라로 QR을 찍으면 연결됩니다</p>}
        <StatusLine running={status.running} listeners={status.listeners} />
      </section>

      <section className="bg-[#262625] rounded-xl divide-y divide-[#333332] text-sm">
        <div className="flex items-center justify-between px-4 py-2.5 short:hidden">
          <span className="text-gray-400">이 PC 이름</span>
          <span className="text-white">{status.name}</span>
        </div>
        <label className="flex items-center justify-between gap-3 px-4 py-2">
          <span className="text-gray-400 shrink-0">코덱</span>
          <CodecSelect codecs={codecs} value={status.codec} onChange={onCodecChange} />
        </label>
      </section>

      {error && <p className="text-red-400 text-sm text-center break-all">{error}</p>}

      <ShareButton running={status.running} busy={busy} onClick={onToggle} />
    </main>
  );
}

export default PortraitLayout;
