import * as M from "@radix-ui/react-dropdown-menu";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

export const DropdownMenu = M.Root;
export const DropdownTrigger = M.Trigger;

export function DropdownContent({ children, align = "end", className }: { children: ReactNode; align?: "start" | "end"; className?: string }) {
  return (
    <M.Portal>
      <M.Content
        align={align}
        sideOffset={6}
        className={cn(
          "z-50 min-w-44 rounded-lg border border-line bg-surface p-1 shadow-lg focus:outline-none",
          className,
        )}
      >
        {children}
      </M.Content>
    </M.Portal>
  );
}

export function DropdownItem({
  children, onSelect, destructive, disabled, ...rest
}: {
  children: ReactNode;
  onSelect?: () => void;
  destructive?: boolean;
  disabled?: boolean;
} & Record<string, unknown>) {
  return (
    <M.Item
      disabled={disabled}
      onSelect={onSelect}
      className={cn(
        "flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm outline-none",
        "data-[highlighted]:bg-subtle data-[disabled]:opacity-50 data-[disabled]:pointer-events-none",
        destructive && "text-danger-text",
      )}
      {...rest}
    >
      {children}
    </M.Item>
  );
}

export function DropdownLabel({ children }: { children: ReactNode }) {
  return <M.Label className="px-2 py-1.5 text-[12px] text-faint">{children}</M.Label>;
}

export function DropdownSeparator() {
  return <M.Separator className="my-1 h-px bg-line-subtle" />;
}
