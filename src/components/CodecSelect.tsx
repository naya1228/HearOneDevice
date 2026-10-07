export type CodecInfo = { id: number; name: string; warning: string | null };
type Props = { codecs: CodecInfo[]; value: number; onChange: (id: number) => void };

// WebKitGTK는 기본 select를 GTK 모양(밝은 바탕)으로 그려서 appearance-none + 화살표 직접 그림
function CodecSelect({ codecs, value, onChange }: Props) {
  return (
    <span className="relative block">
      <select
        className="appearance-none w-full bg-[#2A2A29] text-white text-sm rounded-md pl-3 pr-8 py-2"
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      >
        {codecs.map((c) => (
          <option key={c.id} value={c.id} className="bg-[#2A2A29] text-white">
            {c.id}. {c.name}
          </option>
        ))}
      </select>
      <span className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 text-xs">
        ▼
      </span>
    </span>
  );
}

export default CodecSelect;
