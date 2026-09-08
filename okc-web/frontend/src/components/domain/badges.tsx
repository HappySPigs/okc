import {
  Ban, BadgeCheck, CircleAlert, CircleDashed, Clock, GitCompareArrows, Loader2,
  OctagonAlert, Radio, ShieldCheck, TriangleAlert, Undo2, UserCog, History,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { checkpointLabel } from "@/lib/format";
import type { Checkpoint, Severity } from "@/lib/types";

// Severity language (design-system §6 — the single icon dictionary).
export function SeverityBadge({ severity }: { severity: Severity }) {
  const s = (severity || "").toLowerCase();
  if (s === "critical")
    return <Badge tone="danger" fill="solid"><OctagonAlert className="size-3" /> Critical</Badge>;
  if (s === "major")
    return <Badge tone="danger"><TriangleAlert className="size-3" /> Major</Badge>;
  if (s === "minor")
    return <Badge tone="warn"><CircleAlert className="size-3" /> Minor</Badge>;
  // Unknown severity is fail-closed (blocking) on the backend — reflect that.
  return <Badge tone="danger"><TriangleAlert className="size-3" /> {severity}</Badge>;
}

export function WaivedBadge() {
  return <Badge fill="outline"><Undo2 className="size-3" /> Waived</Badge>;
}

export function ContradictionBadge() {
  return <Badge tone="contra"><GitCompareArrows className="size-3" /> 모순 · 승자 없음</Badge>;
}

export function ApprovedBadge() {
  return <Badge tone="ok"><BadgeCheck className="size-3" /> 승인됨</Badge>;
}

export function RoleBadge({ role }: { role: string }) {
  if (role === "admin")
    return <Badge tone="accent" fill="solid"><ShieldCheck className="size-3" /> admin</Badge>;
  return <Badge tone="neutral"><UserCog className="size-3" /> {role}</Badge>;
}

// Run-state pill / checkpoint badge (design-system §12.7).
export function CheckpointBadge({
  checkpoint, blocked, stale,
}: { checkpoint: Checkpoint | string; blocked?: number; stale?: boolean }) {
  if (blocked && blocked > 0)
    return <Badge tone="danger"><Ban className="size-3" /> 차단 {blocked}</Badge>;
  if (stale)
    return <Badge tone="warn"><History className="size-3" /> {checkpointLabel(checkpoint)} · stale</Badge>;
  const cp = checkpoint;
  if (cp === "verified" || cp === "ready_to_compile")
    return <Badge tone="ok"><BadgeCheck className="size-3" /> {checkpointLabel(cp)}</Badge>;
  if (cp === "needs_provider" || cp === "needs_sources")
    return <Badge tone="neutral"><CircleDashed className="size-3" /> {checkpointLabel(cp)}</Badge>;
  return <Badge tone="accent"><Loader2 className="size-3" /> {checkpointLabel(cp)}</Badge>;
}

export function ServingStatusBadge({ status }: { status: string }) {
  if (status === "live")
    return <Badge tone="ok" fill="solid"><Radio className="size-3" /> LIVE</Badge>;
  if (status === "stale")
    return <Badge tone="warn"><History className="size-3" /> STALE</Badge>;
  return <Badge tone="neutral"><Radio className="size-3" /> OFFLINE</Badge>;
}

export function TokenStatusBadge({ status }: { status: string }) {
  if (status === "active") return <Badge tone="ok">활성</Badge>;
  if (status === "expired") return <Badge tone="warn"><Clock className="size-3" /> 만료</Badge>;
  if (status === "revoked") return <Badge tone="danger"><Ban className="size-3" /> 폐기</Badge>;
  return <Badge tone="neutral">미사용</Badge>;
}
