import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { cn } from "@/lib/cn";
import { toast } from "./toaster";

/** Mono value + copy affordance (design-system §16: hashes/paths/tokens are mono + Copy). */
export function CopyButton({
  value, label, mono = true, className, testid,
}: { value: string; label?: string; mono?: boolean; className?: string; testid?: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value);
    } catch {
      /* clipboard blocked; still show visual ack */
    }
    setCopied(true);
    toast.success("복사되었습니다");
    setTimeout(() => setCopied(false), 1200);
  };
  return (
    <button
      type="button"
      onClick={copy}
      data-testid={testid}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-md border border-line px-2 py-1 text-[13px] hover:bg-subtle",
        mono && "font-mono",
        className,
      )}
      title="복사"
    >
      <span className="truncate">{label ?? value}</span>
      {copied ? <Check className="size-3.5 text-ok-text" /> : <Copy className="size-3.5 text-faint" />}
    </button>
  );
}
