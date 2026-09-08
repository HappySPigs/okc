import * as T from "@radix-ui/react-tabs";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

export function Tabs({
  value, defaultValue, onValueChange, children, className,
}: {
  value?: string;
  defaultValue?: string;
  onValueChange?: (v: string) => void;
  children: ReactNode;
  className?: string;
}) {
  return (
    <T.Root value={value} defaultValue={defaultValue} onValueChange={onValueChange} className={className}>
      {children}
    </T.Root>
  );
}

export function TabsList({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <T.List className={cn("flex items-center gap-1 border-b border-line", className)}>{children}</T.List>
  );
}

export function TabsTrigger({
  value, children, className, ...rest
}: { value: string; children: ReactNode; className?: string } & Record<string, unknown>) {
  return (
    <T.Trigger
      value={value}
      className={cn(
        "relative -mb-px border-b-2 border-transparent px-3 py-2 text-sm text-muted transition-colors",
        "hover:text-fg data-[state=active]:border-accent data-[state=active]:text-fg data-[state=active]:font-medium",
        className,
      )}
      {...rest}
    >
      {children}
    </T.Trigger>
  );
}

export function TabsContent({ value, children, className }: { value: string; children: ReactNode; className?: string }) {
  return (
    <T.Content value={value} className={cn("focus:outline-none", className)}>
      {children}
    </T.Content>
  );
}
