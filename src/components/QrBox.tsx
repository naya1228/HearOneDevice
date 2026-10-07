import QRCode from "react-qr-code";

// size: 크기 클래스 (화면마다 다름)
type Props = { link: string | null; size?: string };

// QR 자리. 공유가 꺼져 있어도 자리를 남겨 켜고 끌 때 화면이 움직이지 않게 함
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
