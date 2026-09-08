import { FileArchive } from "lucide-react";
import type { ReactNode } from "react";

// Upload Shell (design-system §11): NO app shell — thin org bar + centered card.
export function UploadShell({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-screen bg-app">
      <div className="flex h-12 items-center gap-2 border-b border-line bg-surface px-6">
        <FileArchive className="size-4 text-accent" />
        <span className="text-sm font-semibold tracking-tight">okc-web · 개인 Vault 업로드</span>
      </div>
      <div className="mx-auto flex max-w-2xl flex-col gap-4 px-4 py-10">{children}</div>
    </div>
  );
}
