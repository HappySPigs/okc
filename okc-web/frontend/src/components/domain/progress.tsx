import { cn } from "@/lib/cn";

export function ProgressBar({
  value, total, className, tone = "accent",
}: { value: number; total?: number | null; className?: string; tone?: "accent" | "ok" }) {
  const pct = total && total > 0 ? Math.min(100, Math.round((value / total) * 100)) : value > 0 ? 100 : 0;
  return (
    <div className={cn("h-2 w-full overflow-hidden rounded-full bg-subtle", className)}>
      <div
        className={cn("h-full rounded-full transition-[width] duration-300 ease-out", tone === "ok" ? "bg-ok" : "bg-accent")}
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}
