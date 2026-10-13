import type { UpdateState } from "../useUpdate";

// 새 버전이 있을 때만 한 줄 + 업데이트 버튼
function UpdateNotice({ version, installing, error, install }: UpdateState) {
  if (!version) return null;
  return (
    <div className="flex flex-col items-center gap-1 text-sm break-keep">
      <p className="flex items-center gap-3 text-gray-400">
        새 버전 v{version}
        <button
          className="text-[#FD6000] hover:underline disabled:opacity-60"
          type="button"
          disabled={installing}
          onClick={install}
        >
          {installing ? "설치 중..." : "업데이트"}
        </button>
      </p>
      {error && <p className="text-red-400 text-xs break-all">{error}</p>}
    </div>
  );
}

export default UpdateNotice;
