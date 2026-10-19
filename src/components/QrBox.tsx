import QRCode from "react-qr-code";

type Props = { link: string | null; size?: string };

// docs/FEATURES.md: PC 화면
function QrBox({ link, size = "size-36" }: Props) {
  if (!link) {
    return (
      <div className={`${size} shrink-0 rounded-lg border-2 border-dashed border-[#3A3A39] flex items-center justify-center text-center text-gray-600 text-xs break-keep p-3`}>
        공유를 시작하면 QR이 나타납니다
      </div>
    );
  }
  return (
    <div className={`${size} shrink-0 bg-white p-2.5 rounded-lg`}>
      <QRCode value={link} size={256} viewBox="0 0 256 256" className="size-full" />
    </div>
  );
}

export default QrBox;
