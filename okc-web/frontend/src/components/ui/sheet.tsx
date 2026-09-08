import * as D from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

// Side panel for finding/provenance detail on narrow viewports (design-system §4).
export function Sheet({
  open, onOpenChange, title, children, side = "right",
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  title?: ReactNode;
  children: ReactNode;
  side?: "right" | "left";
}) {
  return (
    <D.Root open={open} onOpenChange={onOpenChange}>
      <D.Portal>
        <D.Overlay className="fixed inset-0 z-40 bg-black/30" />
        <D.Content
          className={cn(
            "fixed inset-y-0 z-50 flex w-[min(24rem,90vw)] flex-col border-line bg-surface shadow-lg focus:outline-none",
            side === "right" ? "right-0 border-l" : "left-0 border-r",
          )}
        >
          <div className="flex items-center justify-between border-b border-line px-4 py-3">
            <D.Title className="text-sm font-semibold">{title}</D.Title>
            <D.Close className="rounded-md p-1 text-faint hover:bg-subtle" aria-label="닫기">
              <X className="size-4" />
            </D.Close>
          </div>
          <div className="flex-1 overflow-y-auto p-4">{children}</div>
        </D.Content>
      </D.Portal>
    </D.Root>
  );
}
