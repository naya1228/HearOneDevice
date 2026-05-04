import { useState } from "react";
import Button from "./components/Button";

type Status = "idle" | "listening";

function ReceiverApp() {
  const [status, setStatus] = useState<Status>("idle");

  return (
    <main className="flex flex-col bg-[#1F1F1E] items-center p-3 h-dvh">
      <img className="rounded-md" src="/sharing.svg" />
      <span className="text-white text-4xl font-bold m-2">ShareYourSounds</span>
      <p className="text-gray-500 mb-8">Receiver Mode</p>

      {status === "idle" && (
        <div className="flex flex-col items-center gap-4">
          <p className="text-gray-400 text-sm text-center mb-2">
            호스트가 방을 열었다면 연결 버튼을 누르세요.
          </p>
          <Button type="button" onClick={() => setStatus("listening")}>
            Connect &amp; Listen
          </Button>
        </div>
      )}

      {status === "listening" && (
        <div className="flex flex-col items-center gap-4">
          <div className="relative flex items-center justify-center w-24 h-24">
            <span className="absolute inline-flex h-full w-full rounded-full bg-[#FD6000] opacity-20 animate-ping" />
            <span className="relative inline-flex rounded-full h-16 w-16 bg-[#FD6000] opacity-80 items-center justify-center">
              <svg xmlns="http://www.w3.org/2000/svg" className="w-8 h-8 text-white" viewBox="0 0 24 24" fill="currentColor">
                <path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3A4.5 4.5 0 0 0 14 7.97v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77 0-4.28-2.99-7.86-7-8.77z"/>
              </svg>
            </span>
          </div>

          <p className="text-[#FD6000] text-lg font-semibold">Listening...</p>

          <div className="mt-6 w-full max-w-sm bg-black/30 rounded-lg p-6 flex items-end justify-center gap-1 h-24">
            {Array.from({ length: 16 }).map((_, i) => (
              <div
                key={i}
                className="w-2 rounded-full bg-[#FD6000] opacity-40"
                style={{ height: `${Math.random() * 60 + 10}%` }}
              />
            ))}
          </div>

          <button
            className="text-gray-500 text-sm mt-4 underline"
            onClick={() => setStatus("idle")}
          >
            Disconnect
          </button>
        </div>
      )}
    </main>
  );
}

export default ReceiverApp;
