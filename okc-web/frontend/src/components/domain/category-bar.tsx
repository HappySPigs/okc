import { cn } from "@/lib/cn";

export interface Segment {
  value: number;
  className: string; // bg-* utility
  label?: string;
}

/** Source-usage / severity-distribution one-line bar (design-system §4.2). */
export function CategoryBar({ segments, className }: { segments: Segment[]; className?: string }) {
  const total = segments.reduce((s, x) => s + x.value, 0) || 1;
  return (
    <div className={cn("flex h-2 w-full overflow-hidden rounded-full bg-subtle", className)}>
      {segments.map((s, i) =>
        s.value > 0 ? (
          <div key={i} className={cn(s.className)} style={{ width: `${(s.value / total) * 100}%` }} title={s.label} />
        ) : null,
      )}
    </div>
  );
}

/** n/10 source-slot bar (amber at cap). */
export function SlotBar({ used, limit }: { used: number; limit: number }) {
  const atCap = used >= limit;
  return (
    <div className="flex items-center gap-2">
      <CategoryBar
        className="w-24"
        segments={[
          { value: used, className: atCap ? "bg-warn" : "bg-accent" },
          { value: Math.max(0, limit - used), className: "bg-transparent" },
        ]}
      />
      <span className={cn("tabular text-[13px]", atCap ? "text-warn-text" : "text-muted")}>
        {used}/{limit}
      </span>
    </div>
  );
}
