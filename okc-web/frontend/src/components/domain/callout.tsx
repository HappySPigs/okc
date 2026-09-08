import { Info } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

type Tone = "neutral" | "warn" | "danger" | "contra" | "ok";

const TONES: Record<Tone, string> = {
  neutral: "bg-subtle border-line text-muted",
  warn: "bg-warn-soft border-warn-line text-warn-text",
  danger: "bg-danger-soft border-danger-line text-danger-text",
  contra: "bg-contra-soft border-contra-line text-contra-text",
  ok: "bg-ok-soft border-ok-line text-ok-text",
};

/** Inline constraint notice — provenance disclaimer, okc-mcp deferred, no-clobber. */
export function Callout({
  tone = "neutral", icon, title, children, className,
}: { tone?: Tone; icon?: ReactNode; title?: ReactNode; children?: ReactNode; className?: string }) {
  return (
    <div className={cn("flex items-start gap-2.5 rounded-lg border px-3 py-2.5 text-[13px]", TONES[tone], className)}>
      <span className="mt-0.5 shrink-0">{icon ?? <Info className="size-4" />}</span>
      <div>
        {title && <div className="font-medium">{title}</div>}
        {children && <div className={cn(title && "mt-0.5")}>{children}</div>}
      </div>
    </div>
  );
}
