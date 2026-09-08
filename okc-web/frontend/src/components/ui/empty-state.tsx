import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

export function EmptyState({
  icon: Icon, title, description, action,
}: { icon: LucideIcon; title: string; description?: string; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-line bg-surface px-6 py-14 text-center">
      <Icon className="size-8 text-faint" />
      <div>
        <div className="font-medium">{title}</div>
        {description && <p className="mt-1 max-w-sm text-sm text-muted">{description}</p>}
      </div>
      {action}
    </div>
  );
}
