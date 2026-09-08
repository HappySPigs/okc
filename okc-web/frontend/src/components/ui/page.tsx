import { TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";
import type { ApiError } from "@/lib/api";
import { cn } from "@/lib/cn";
import { Alert } from "./alert";
import { Button } from "./button";

export function PageHeader({
  title, description, actions, badge, className,
}: { title: ReactNode; description?: ReactNode; actions?: ReactNode; badge?: ReactNode; className?: string }) {
  return (
    <div className={cn("mb-5 flex items-start justify-between gap-4", className)}>
      <div>
        <div className="flex items-center gap-2">
          <h1 className="text-xl font-semibold tracking-tight">{title}</h1>
          {badge}
        </div>
        {description && <p className="mt-1 text-sm text-muted">{description}</p>}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </div>
  );
}

/** Uniform error surface — branches on the stable code/category (never message). */
export function ErrorState({ error, onRetry }: { error: ApiError; onRetry?: () => void }) {
  return (
    <Alert
      variant={error.isBusy ? "warning" : "destructive"}
      icon={<TriangleAlert className="size-4" />}
      title={error.code}
      action={onRetry ? <Button size="sm" variant="outline" onClick={onRetry}>재시도</Button> : undefined}
    >
      {error.message}
    </Alert>
  );
}

export function Section({ title, children, className }: { title?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={cn("space-y-3", className)}>
      {title && <h2 className="text-base font-semibold tracking-tight">{title}</h2>}
      {children}
    </section>
  );
}
