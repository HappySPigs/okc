import * as T from "@radix-ui/react-tooltip";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** Tooltip that makes constraints visible (e.g. disabled Waive on Major/Critical). */
export function Tooltip({
  content, children, className,
}: { content: ReactNode; children: ReactNode; className?: string }) {
  if (!content) return <>{children}</>;
  return (
    <T.Provider delayDuration={200}>
      <T.Root>
        <T.Trigger asChild>{children}</T.Trigger>
        <T.Portal>
          <T.Content
            sideOffset={6}
            className={cn(
              "z-50 max-w-xs rounded-md border border-line bg-surface px-2.5 py-1.5 text-[13px] text-fg shadow-lg",
              className,
            )}
          >
            {content}
            <T.Arrow className="fill-[var(--surface)]" />
          </T.Content>
        </T.Portal>
      </T.Root>
    </T.Provider>
  );
}
