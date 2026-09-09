# U6 — Frontend SPA (React + Vite) — Code Generation Plan

**Unit**: U6 (`frontend/`) · **Epics**: E1..E5 (admin app + contributor upload shell) · **Stories**: the §9 end-to-end demo path
**Depth**: lean/MVP · **Stack**: React 19 + Vite + TypeScript + React Router v6 + Tailwind v4 (ADR-0025)
**Grounding**: `aidlc-docs/inception/application-design/ui-screens.md` (24-screen inventory, §1 route map, §9 walkthrough, 3 FOCAL screens), `design-system.md` (Radix slate+indigo tokens §1.2, badge language §6, App Shell IA §7–13, focal strategy §14, motion §15, Do/Don't §16), backend routers/models under `backend/app/**` (READ-ONLY frozen contract).

> FROZEN (READ/IMPORT ONLY, never edit): entire `backend/**`, all `aidlc-docs/**` except this plan + `aidlc-docs/construction/u6-frontend/**`. The SPA is served from `frontend/dist` by `backend/app/main.py` `_mount_spa`, so `npm run build` MUST emit `frontend/dist`.

## Key design decisions (LEAN, reconciled against frozen contracts)
- **Lean deps**: Tailwind v4 (`@tailwindcss/vite`, CSS-first `@theme`), Radix colors CSS, Radix primitives ONLY for a11y-critical overlays (Dialog, Tooltip, Tabs, Switch, DropdownMenu), lucide-react, sonner, react-markdown, clsx+tailwind-merge. **No Tremor** (build tiny Tracker/ProgressBar/Metric/CategoryBar/Steps primitives ourselves — avoids the Tremor↔Tailwind-v3 coupling and keeps the footprint small). **No rhf/zod** (small controlled-form validation inline). Deviations noted in summary.
- **Same-origin API**, cookie auth (`okc_session`, HttpOnly): `fetch` with `credentials:'include'`. Typed `ApiError{code,category,message,retryable,retryAfterMs}`; branch on `code`/`category`, never message. Global `401/SESSION_EXPIRED`→redirect `/login`; `PROJECT_BUSY`→retry banner+Retry-After; `validation`→inline; `approval/conflict`→section link.
- **Dev proxy**: Vite proxies `/api` + `/u` → `http://localhost:8000`.
- **RBAC by shell separation**: admin App Shell (2-tier nav) vs contributor Upload Shell (`/u/:token` 3 screens, no shell). Unauth `/login`; `/403` + global 401 interceptor.
- **Frozen-contract friction (surfaced, not worked around)**:
  1. No `GET /sources` list endpoint → E3-4 Sources derives registered sources from `GET /tokens` rows where `registered_source_id != null`.
  2. Machine serving reads (`/api/serving/{pid}/files|verify|explain`) serve only when published → E5-1 tree + E5-2 verify gate behind publish with an honest "publish to browse served artifacts" state.
  3. Cluster contradictions are opaque `ClusterDetailView.contradictions` (`Any`) → rendered read-only, "no winner", best-effort shape parse.
- **Honest core constraints in UI**: no winner-select; Major/Critical waive-forbidden + compile-block; ≤10 sources; freeze-then-run staleness; read-only compiled vault; PROJECT_BUSY retry; OkcError code/category branching; `curator_id` unverified label; okc-mcp deferred (contract only).
- **3 FOCAL screens polished** (transform/opacity ≤250ms): E3-5 integration monitor, E4-3 cluster review (main hero), E5-2 provenance/verify. Everything else deliberately plain.
- `data-testid` on interactive elements, `{screen}-{role}` naming.

## Files
- [x] `frontend/package.json`, `tsconfig*.json`, `vite.config.ts`, `index.html`, `.gitignore`, `vitest.config`/setup
- [x] `frontend/src/index.css` — Radix scales + design-system §1.2 semantic tokens + Tailwind v4 `@theme`
- [x] `frontend/src/lib/api.ts` — typed fetch client, `ApiError`, 401/BUSY handling, `useJobPoll`
- [x] `frontend/src/lib/types.ts` — wire DTO types mirrored from backend models
- [x] `frontend/src/lib/format.ts`, `frontend/src/lib/cn.ts` — helpers (hash trunc, dates, className merge)
- [x] `frontend/src/components/ui/*` — Button, Card, Badge, Table, Dialog, Sheet, Tooltip, Alert, Tabs, Switch, DropdownMenu, Skeleton, Input, Textarea, Toaster, EmptyState, CopyButton
- [x] `frontend/src/components/domain/*` — Tracker (pipeline blocks), Steps (run progress), ProgressBar, Metric, CategoryBar, SeverityBadge, StatusBadge, Callout, PipelineTracker
- [x] `frontend/src/app/*` — AppShell (sidebar+topbar+banners), AuthGate, RBAC route guards, project context
- [x] `frontend/src/screens/**` — all §1 screens (focal polished, rest plain)
- [x] `frontend/src/App.tsx`, `frontend/src/main.tsx` — router + providers
- [x] `frontend/src/**/*.test.tsx` — vitest smoke: api error-mapping + a component render + severity gating

## Steps
1. [x] Write this plan.
2. [x] Scaffold Vite+React+TS, install lean deps, configure Tailwind v4 + proxy + vitest; verify `npm install`.
3. [x] `index.css` design tokens (Radix slate+indigo+semantic → §1.2 CSS vars) + typography + base.
4. [x] `lib/types.ts` + `lib/api.ts` (typed client, ApiError mapping, 401/BUSY, session, job poll) + `lib/format.ts`/`cn.ts`.
5. [x] UI primitives + domain primitives (badges/tracker/steps/progress/metric).
6. [x] App shell + auth gate + RBAC routing + all route registration (stubs for non-demo).
7. [x] Auth + upload-shell screens (E1-1, E1-4, E2-3/4/5) wired to real API.
8. [x] Orchestration screens (E3-1/2/3/4/6) + tokens (E2-1/2) plain, wired.
9. [x] **FOCAL E3-5** integration monitor (pipeline Tracker + run Steps + live job-event log poll + Sonner stream).
10. [x] Review screens: E4-1 gate, E4-2 taxonomy (plain) + **FOCAL E4-3** cluster review 3-panel workbench + E4-4 regenerate (plain).
11. [x] Serving: E5-1 compiled vault (compile trigger + tree), **FOCAL E5-2** provenance/verify (lineage draw-in), E5-3 publish, E5-4 contract.
12. [x] Settings stubs (E1-3 users, E1-5 account) — minimal functional, routes/nav present.
13. [x] Vitest smoke tests (api error mapping + severity gating + a render). 
14. [x] Verify GREEN: `npm run build` (tsc + vite → `frontend/dist`), `npm run test` (vitest), report exact output.
15. [x] Summary — handed to orchestrator (subagent harness blocks writing under `aidlc-docs/construction/u6-frontend/code/`).
</content>
</invoke>
