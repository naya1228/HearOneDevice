import Button from "./Button";

type Props = { running: boolean; busy: boolean; onClick: () => void };

// docs/FEATURES.md: PC 화면
function ShareButton({ running, busy, onClick }: Props) {
  return (
    <Button type="button" variant={running ? "secondary" : "primary"} disabled={busy} onClick={onClick}>
      {busy ? "..." : running ? "공유 중지" : "공유 시작"}
    </Button>
  );
}

export default ShareButton;
