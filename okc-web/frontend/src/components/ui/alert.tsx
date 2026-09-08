import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/cn";

type Variant = "info" | "warning" | "destructive" | "success";

const VARIANTS: Record<Variant, string> = {
  info: "bg-accent-soft border-accent-line text-accent-text",
  warning: "bg-warn-soft border-warn-line text-warn-text",
  destructive: "bg-danger-soft border-danger-line text-danger-text",
  success: "bg-ok-soft border-ok-line text-ok-text",
};

export interface AlertProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
  variant?: Variant;
  icon?: ReactNode;
  title?: ReactNode;
  action?: ReactNode;
}

export function Alert({ variant = "info", icon, title, action, className, children, ...props }: AlertProps) {
  return (
    <div
      role="alert"
      className={cn("flex items-start gap-3 rounded-lg border px-3.5 py-3 text-sm", VARIANTS[variant], className)}
      {...props}
    >
      {icon && <span className="mt-0.5 shrink-0">{icon}</span>}
      <div className="flex-1 min-w-0">
        {title && <div className="font-medium">{title}</div>}
        {children && <div className={cn(title && "mt-0.5", "text-fg/80")}>{children}</div>}
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </div>
  );
}
