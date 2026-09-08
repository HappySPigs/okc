import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { Card } from "@/components/ui/card";

export function Metric({
  label, value, hint, tone, className,
}: {
  label: string;
  value: ReactNode;
  hint?: ReactNode;
  tone?: "danger" | "ok" | "warn";
  className?: string;
}) {
  return (
    <Card className={cn("p-4", className)}>
      <div className="text-[13px] text-muted">{label}</div>
      <div
        className={cn(
          "mt-1 text-2xl font-semibold tabular",
          tone === "danger" && "text-danger-text",
          tone === "ok" && "text-ok-text",
          tone === "warn" && "text-warn-text",
        )}
      >
        {value}
      </div>
      {hint && <div className="mt-0.5 text-[12px] text-faint">{hint}</div>}
    </Card>
  );
}

export function MetricGrid({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("grid gap-3 sm:grid-cols-2 lg:grid-cols-4", className)}>{children}</div>;
}
