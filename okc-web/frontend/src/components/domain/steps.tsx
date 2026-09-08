import { CircleCheck, CircleDashed, CircleDot, Loader2, XCircle } from "lucide-react";
import { cn } from "@/lib/cn";
import { phaseLabel } from "@/lib/format";

export type StepState = "done" | "current" | "pending" | "failed";

// Micro run-progress: vertical Steps list (design-system §13). NEVER block style.
export function RunSteps({
  phases, currentPhase, jobState, className,
}: {
  phases: readonly string[];
  currentPhase?: string | null;
  jobState?: string;
  className?: string;
}) {
  const idx = currentPhase ? phases.indexOf(currentPhase) : -1;
  const failed = jobState === "failed" || jobState === "cancelled";
  const complete = jobState === "completed";
  return (
    <ol className={cn("space-y-2.5", className)} aria-label="실행 진행">
      {phases.map((p, i) => {
        let state: StepState =
          complete ? "done" : i < idx ? "done" : i === idx ? (failed ? "failed" : "current") : "pending";
        if (failed && i === idx) state = "failed";
        return (
          <li key={p} className="flex items-center gap-2.5" data-testid={`run-step-${p}`}>
            <StepIcon state={state} />
            <span
              className={cn(
                "text-sm",
                state === "current" && "font-medium text-fg",
                state === "done" && "text-muted",
                state === "pending" && "text-faint",
                state === "failed" && "text-danger-text",
              )}
            >
              {phaseLabel(p)}
            </span>
          </li>
        );
      })}
    </ol>
  );
}

function StepIcon({ state }: { state: StepState }) {
  if (state === "done") return <CircleCheck className="size-4 text-ok" />;
  if (state === "current") return <Loader2 className="size-4 animate-spin text-accent" />;
  if (state === "failed") return <XCircle className="size-4 text-danger" />;
  return <CircleDashed className="size-4 text-faint" />;
}

export { CircleDot };
