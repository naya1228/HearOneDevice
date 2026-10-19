import QrBox from "../components/QrBox";
import StatusLine from "../components/StatusLine";
import CodecSelect from "../components/CodecSelect";
import CodecWarning from "../components/CodecWarning";
import ShareButton from "../components/ShareButton";
import UpdateNotice from "../components/UpdateNotice";
import type { LayoutProps } from "./LayoutProps";

// docs/FEATURES.md: PC 화면
function LandscapeLayout({ status, codecs, busy, error, onToggle, onCodecChange, update }: LayoutProps) {
  return (
    <main className="flex flex-col items-center justify-center gap-8 p-6 pb-[calc(1.5rem+5vh)] min-h-dvh">
      <header className="flex flex-col items-center text-center short:hidden">
        <img className="w-40" src="sharing.svg" />
        <h1 className="text-white text-4xl font-bold mt-2">HearOneDevice</h1>
        <p className="text-gray-500 mt-1">PC 소리를 블루투스로 폰에 보냅니다</p>
        <p className="text-gray-600 text-sm mt-1">이 PC 이름: {status.name}</p>
      </header>

      <section className="flex gap-4 w-full max-w-md">
        <QrBox link={status.running ? status.link : null} />
        <div className="flex flex-col justify-between flex-1 min-w-0">
          <StatusLine running={status.running} listeners={status.listeners} />
          <div className="flex flex-col gap-1">
            <CodecSelect codecs={codecs} value={status.codec} onChange={onCodecChange} />
            <CodecWarning codecs={codecs} value={status.codec} />
          </div>
          <ShareButton running={status.running} busy={busy} onClick={onToggle} />
        </div>
      </section>

      {error && <p className="text-red-400 text-sm text-center break-all max-w-md">{error}</p>}
      <UpdateNotice {...update} />
    </main>
  );
}

export default LandscapeLayout;
