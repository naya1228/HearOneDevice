import StatusLine from "../components/StatusLine";
import type { LayoutProps } from "./LayoutProps";

// docs/FEATURES.md: PC 화면
function TinyLayout({ status }: LayoutProps) {
  return (
    <main className="flex items-center justify-center p-2 min-h-dvh text-sm">
      <StatusLine running={status.running} listeners={status.listeners} />
    </main>
  );
}

export default TinyLayout;
