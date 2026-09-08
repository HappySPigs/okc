import type { Checkpoint } from "./types";

/** Shorten a content/manifest hash for display (keeps head+tail). */
export function shortHash(hash?: string | null, head = 8, tail = 6): string {
  if (!hash) return "—";
  if (hash.length <= head + tail + 1) return hash;
  return `${hash.slice(0, head)}…${hash.slice(-tail)}`;
}

export function formatDate(iso?: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    year: "numeric", month: "short", day: "numeric",
    hour: "2-digit", minute: "2-digit",
  });
}

export function formatTime(iso?: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleTimeString(undefined, { hour12: false });
}

const CHECKPOINT_LABELS: Record<string, string> = {
  needs_provider: "Provider",
  needs_sources: "Sources",
  needs_disclosure: "Disclosure",
  needs_taxonomy: "Taxonomy",
  needs_clusters: "Clusters",
  ready_to_compile: "Compile",
  verified: "Verified",
};

export function checkpointLabel(cp: Checkpoint | string): string {
  return CHECKPOINT_LABELS[cp] ?? cp;
}

/** The macro phase order the run stepper reflects (design-system §13). */
export const RUN_PHASES = ["preflight", "embedding", "candidate", "synthesis", "critic"] as const;

export function phaseLabel(phase?: string | null): string {
  if (!phase) return "—";
  return phase.charAt(0).toUpperCase() + phase.slice(1);
}
