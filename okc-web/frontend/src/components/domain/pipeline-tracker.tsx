import { cn } from "@/lib/cn";
import { checkpointLabel } from "@/lib/format";
import { PIPELINE, type Checkpoint } from "@/lib/types";
import { Tooltip } from "@/components/ui/tooltip";

// Macro pipeline-stage tracker: BLOCK style, 7 IntegrationCheckpoint cells
// (design-system §13). Distinct from the run-progress vertical Steps.
export function PipelineTracker({
  checkpoint, stale, blocked, onSelectStage, className,
}: {
  checkpoint: Checkpoint | string;
  stale?: boolean;
  blocked?: number;
  onSelectStage?: (cp: Checkpoint) => void;
  className?: string;
}) {
  const currentIdx = PIPELINE.indexOf(checkpoint as Checkpoint);
  return (
    <div className={className}>
      <div className="mb-1.5 text-[12px] font-medium text-faint">파이프라인 단계</div>
      <div className="flex gap-1" role="list" aria-label="파이프라인 단계">
        {PIPELINE.map((cp, i) => {
          const done = currentIdx >= 0 && i < currentIdx;
          const current = i === currentIdx;
          const isBlocked = !!blocked && blocked > 0 && cp === "needs_clusters";
          const color = isBlocked
            ? "bg-danger"
            : current && stale
              ? "bg-warn"
              : done
                ? "bg-ok"
                : current
                  ? "bg-accent okc-pulse"
                  : "bg-line-subtle";
          const clickable = onSelectStage && (cp === "needs_taxonomy" || cp === "needs_clusters" || done || current);
          return (
            <Tooltip key={cp} content={checkpointLabel(cp)}>
              <button
                type="button"
                role="listitem"
                disabled={!clickable}
                onClick={clickable ? () => onSelectStage!(cp) : undefined}
                data-testid={`pipeline-stage-${cp}`}
                className={cn("h-2 flex-1 rounded-full transition-colors", color, clickable && "cursor-pointer")}
                aria-label={checkpointLabel(cp)}
              />
            </Tooltip>
          );
        })}
      </div>
      <div className="mt-1.5 flex justify-between text-[11px] text-faint">
        <span>{checkpointLabel(PIPELINE[0])}</span>
        <span className="font-medium text-muted">{checkpointLabel(checkpoint)}</span>
        <span>{checkpointLabel(PIPELINE[PIPELINE.length - 1])}</span>
      </div>
    </div>
  );
}
