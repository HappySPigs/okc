import type { HTMLAttributes } from "react";
import { cn } from "@/lib/cn";

// Product's core language (design-system §6): color + icon + text triple-encoded.
export type Tone = "neutral" | "accent" | "danger" | "warn" | "ok" | "contra";
export type Fill = "soft" | "solid" | "outline";

const SOFT: Record<Tone, string> = {
  neutral: "bg-subtle text-muted border-line-subtle",
  accent: "bg-accent-soft text-accent-text border-accent-line",
  danger: "bg-danger-soft text-danger-text border-danger-line",
  warn: "bg-warn-soft text-warn-text border-warn-line",
  ok: "bg-ok-soft text-ok-text border-ok-line",
  contra: "bg-contra-soft text-contra-text border-contra-line",
};

const SOLID: Record<Tone, string> = {
  neutral: "bg-[var(--slate-9)] text-white border-transparent",
  accent: "bg-accent text-accent-fg border-transparent",
  danger: "bg-danger text-white border-transparent",
  warn: "bg-warn text-white border-transparent",
  ok: "bg-ok text-white border-transparent",
  contra: "bg-[var(--violet-9)] text-white border-transparent",
};

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  tone?: Tone;
  fill?: Fill;
}

export function Badge({ tone = "neutral", fill = "soft", className, children, ...props }: BadgeProps) {
  const style =
    fill === "solid" ? SOLID[tone] : fill === "outline" ? "bg-transparent text-faint border-line" : SOFT[tone];
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[13px] font-medium leading-5",
        style,
        className,
      )}
      {...props}
    >
      {children}
    </span>
  );
}
