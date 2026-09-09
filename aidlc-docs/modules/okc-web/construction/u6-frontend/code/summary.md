# U6 — Frontend SPA — Code Summary

**Wave**: W4 · **Unit**: U6 (`frontend/`) · React 19 + Vite 6 + TS + React Router v6 + Tailwind v4 · Epics E1–E5 · MVP-only, lean · **Mode**: AUTOPILOT
**Plan**: [`../../plans/u6-frontend-code-generation-plan.md`](../../plans/u6-frontend-code-generation-plan.md)

_Persisted by the orchestrator from the `u6-frontend` agent's verified report (the agent's harness blocked its own summary write; content authoritative, independently re-verified at the W4 wave barrier)._

## What was built
A lean admin SPA + contributor upload shell wired to the frozen FastAPI backend, served from `frontend/dist` by `backend/app/main.py`. 61 source files: typed API client, Radix-token design system, 2-tier RBAC app shell, and all 24 §1 screens (3 FOCAL polished, rest plain).

## Stack / deps (lean)
React 19 + Vite 6 + TS + React Router v6 + **Tailwind v4** (`@tailwindcss/vite`, CSS-first `@theme`); Radix Colors CSS (slate+indigo+semantic); Radix primitives only for a11y overlays (dialog, dropdown-menu, switch, tabs, tooltip); lucide-react, sonner, react-markdown, clsx+tailwind-merge. **Deliberately NOT installed**: Tremor (tiny hand-built Tracker/Steps/ProgressBar/Metric/CategoryBar primitives instead — avoids Tremor↔Tailwind-v3 coupling, smaller bundle), react-hook-form/zod (small inline controlled-form validation). 301 packages total. Fonts: system stack aliased to Inter/Geist Mono (no self-hosted Geist — cosmetic deviation).

## Layout
- `src/lib/` — `api.ts` typed fetch client (`credentials:'include'`, `ApiError{code,category,retryable,retryAfterMs,httpStatus}`, endpoint groups auth/accounts/projects/tokens/upload/review/serving/jobs, global 401 hook), `types.ts` (wire DTOs mirrored from backend models), `hooks.ts` (`useAsync`, `useJobPoll` keyed on job id), `format.ts`, `cn.ts`.
- `src/components/ui/` — Button, Card, Badge, Table, Alert, Input, Skeleton, Dialog/ConfirmDialog, Sheet, Tooltip, Tabs, Switch, Dropdown, Toaster(Sonner), CopyButton, EmptyState, page helpers.
- `src/components/domain/` — badges (§6 severity/status), pipeline-tracker (macro 7-block checkpoint), steps (micro run phases), progress, metric, category-bar/SlotBar, callout.
- `src/app/` — AppShell (sidebar 2-tier nav + RBAC gating + topbar run-state pill + PROJECT_BUSY/stale banners + ProjectContext data loader), auth provider, RequireAdmin guard, project context.
- `src/screens/` — Login, Forbidden(403), NotFound; upload/{Portal,Page,Done,Shell}; projects/{ProjectsList,Overview,Sources,Tokens,Integration}; review/{ReviewGate,Taxonomy,ClusterReview,Regenerate}; serving/{Compiled,Provenance,Serving,Contract}; settings/{Users,Account}.
- `src/index.css` — Radix slate+indigo+semantic → design-system §1.2 semantic CSS vars → Tailwind v4 `@theme inline`; borders-over-shadows, light default, restrained (transform/opacity) motion + `prefers-reduced-motion` off-switch.

## Screens (focal vs plain)
- **FOCAL (polished)**: E3-5 Integration (pipeline Tracker + vertical run-Steps + live job-event log polling `GET /jobs/{id}` + Sonner phase-stream + Provider/Disclosure gate), E4-3 ClusterReview (3-panel workbench, severity badges, Major/Critical disabled Waive+Tooltip, compile-reject pulse banner, "no winner" contradiction split, approve disabled while blocking>0), E5-2 Provenance (SVG lineage draw-in, Verified scale-in, deterministic-hash emphasis, no-winner split, always-on authenticity callout).
- **PLAIN (functional, wired)**: Login (pw+token tabs), 403, upload shell (Portal/Page/Done), ProjectsList(+create), Overview, Sources(+freeze), Tokens(+issue/reveal), ReviewGate, Taxonomy, Regenerate, Compiled(+compile trigger+browser), Serving(+publish), Contract. Settings Users/Account functional-lite (routes/nav present).

## Auth / error handling
Cookie auth (`credentials:'include'`). `ApiError` branched on code/category (never message). Global 401/SESSION_EXPIRED → drop session + redirect `/login` (session-expired banner). `PROJECT_BUSY`/`RESOURCE_LIMIT` → busy warning + Retry-After (body ms preferred, header fallback). `useJobPoll` stops on terminal state.

## Verification (agent-reported; re-verified at the W4 wave barrier)
- `npm install` → 301 packages (2 moderate transitive advisories, no forced fix).
- `npm run build` (tsc --noEmit + vite build) → `frontend/dist` (index.html + CSS 34KB/gz 7.8KB + JS 651KB/gz 198KB); one non-fatal >500KB chunk warning (react-markdown/radix; code-splitting is a future optimization).
- `npm run test` (vitest) → 8 passed (2 files): `lib.test.ts` (ApiError code/category/isAuth/isBusy/retryAfter precedence + format) + `badges.test.tsx` (SeverityBadge per level + fail-closed on unknown).

## Deviations / friction (flagged, not worked around)
1. **`/u/{token}` collision**: backend owns `GET /u/{token}` (JSON API) → contributor SPA portal moved to `/upload/:token`,`/upload/:token/file`,`/upload/:token/done` (API calls still hit `/u/{token}`); Vite proxy key tightened to `/u/`; token reveal hands out `{origin}/upload/{token}`.
2. **SPA history-fallback**: `main.py` `_mount_spa` used `StaticFiles(html=True)` → deep-link entry/refresh 404s under FastAPI serving. **RESOLVED by the orchestrator at the W4 barrier** with a minimal SPA history-fallback in `main.py` (non-`/api`,`/u` 404 → `index.html`); dev proxy already worked.
3. No `GET /sources` endpoint → E3-4 Sources derives registered sources from `GET /tokens` rows (`registered_source_id`).
4. Machine serving reads serve only when published → E5-1/E5-2 gated behind publish with an honest "publish to browse" state.
5. No integration-job cancel route → E3-5 renders a disabled Cancel + explanatory Tooltip.
6. Opaque `Any` payloads (contradictions/taxonomy/synthesis) → defensive best-effort extractors with pretty-JSON fallback.

## data-testid
Interactive elements carry stable `{screen}-{role}` test ids per automation-friendly rules.
