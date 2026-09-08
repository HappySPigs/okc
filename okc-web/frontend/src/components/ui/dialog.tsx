import * as D from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { Button } from "./button";

export const Dialog = D.Root;
export const DialogTrigger = D.Trigger;
export const DialogClose = D.Close;

export function DialogContent({
  children, className, width = "max-w-lg",
}: { children: ReactNode; className?: string; width?: string }) {
  return (
    <D.Portal>
      <D.Overlay className="fixed inset-0 z-40 bg-black/30 data-[state=open]:animate-in" />
      <D.Content
        className={cn(
          "fixed left-1/2 top-1/2 z-50 w-[calc(100vw-2rem)] -translate-x-1/2 -translate-y-1/2",
          "rounded-xl border border-line bg-surface p-5 shadow-lg focus:outline-none",
          width, className,
        )}
      >
        {children}
        <D.Close
          className="absolute right-3 top-3 rounded-md p-1 text-faint hover:bg-subtle"
          aria-label="닫기"
          data-testid="dialog-close"
        >
          <X className="size-4" />
        </D.Close>
      </D.Content>
    </D.Portal>
  );
}

export function DialogHeader({ title, description }: { title: ReactNode; description?: ReactNode }) {
  return (
    <div className="mb-4 flex flex-col gap-1 pr-6">
      <D.Title className="text-base font-semibold tracking-tight">{title}</D.Title>
      {description && <D.Description className="text-sm text-muted">{description}</D.Description>}
    </div>
  );
}

export function DialogFooter({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("mt-5 flex items-center justify-end gap-2", className)}>{children}</div>;
}

/** Destructive/confirm dialog (AlertDialog role). Controlled. */
export function ConfirmDialog({
  open, onOpenChange, title, description, confirmLabel = "확인", cancelLabel = "취소",
  destructive, loading, onConfirm, confirmDisabled, children, testid,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
  loading?: boolean;
  onConfirm: () => void;
  confirmDisabled?: boolean;
  children?: ReactNode;
  testid?: string;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader title={title} description={description} />
        {children}
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)} data-testid={testid ? `${testid}-cancel` : undefined}>
            {cancelLabel}
          </Button>
          <Button
            variant={destructive ? "destructive" : "primary"}
            loading={loading}
            disabled={confirmDisabled}
            onClick={onConfirm}
            data-testid={testid ? `${testid}-confirm` : undefined}
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
