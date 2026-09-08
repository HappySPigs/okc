# Requirements — okc-hooks "Watcher"

**Status**: Draft for review (INCEPTION → Requirements Analysis)
**Date**: 2026-09-07
**Depth**: Comprehensive (greenfield, cross-system, security-relevant tradeoffs, a not-yet-built server dependency)

---

## 1. Intent Analysis

| Attribute | Determination |
|---|---|
| **User request (verbatim intent)** | Build a locally-installed **Watcher** app that periodically monitors an Obsidian vault and, when it changes, uploads the vault to an OKC web service (built on `../okc-core/`). The web service does not yet exist. Uploads may be large, so the upload protocol must be designed deliberately. |
| **Request type** | New Project (greenfield) |
| **Scope** | Cross-system — a local desktop client plus a defined contract against a future web service built on okc-core |
| **Complexity** | Complex — deterministic hashing/dedup, resumable large transfers, offline/backpressure resilience, cross-platform packaging, and explicit security tradeoffs |
| **Primary user-stated concern** | The upload protocol for potentially large vaults |

### 1.1 System Context (grounded in okc-core study + compilation probe)

- okc-core ingests a vault as a **whole, immutable, atomic snapshot** from a **local path** (directory / `.zip` / `.tar.zst`). There is **no delta-ingest and no byte-stream upload API** — a server must materialize uploaded bytes to its own disk, then bind that path for ingestion.
- okc-core is **fully SHA-256 content-addressed**: each file has a `raw_sha256` (identical to `sha256sum`) and the whole vault has a `vault_content_id`. This natively enables **content-addressed have/want deduplication at whole-file granularity**. There is **no sub-file chunking / rolling-hash / binary-delta** capability.
- okc-core **SafetyLimits**: 20 GiB total, 2 GiB per file, 100k files, **10 sources per project** (a "source" is one whole vault/archive input path aggregated into a server project — a project-level cap, not a per-vault client concern).
- okc-core has **no inbound authentication and no multi-tenancy** — both must be defined by the future web service.
- "Compile" is a **3-stage pipeline**: (1) deterministic ingest → in-memory IR; (2) an **AI `integrate` stage** (LLM + embeddings, network, API keys) requiring **human approval of the taxonomy and of every cluster** → a sealed `ApprovedIntegrationPlan`; (3) deterministic offline materialization → a "Compiled Vault" directory. Compile **cannot run without a sealed, human-approved plan**. Therefore an unattended background Watcher **cannot** produce a compiled artifact, and local compilation offers **no privacy benefit** (the plan embeds the full verbatim corpus, including sensitive content). **The Watcher uploads the raw vault; the server owns integrate + approval + compile.**
- **Fail-closed approval on change**: okc-core invalidates any dependent human approval when the content hash changes — a changed-vault submission makes the prior `ApprovedIntegrationPlan` stale. Combined with auto-sync this has a material consequence captured as RISK-02 (§7).

---

## 2. Scope

### 2.1 In Scope
- A locally-installed, long-running **Watcher** process for **macOS, Windows, and Linux** (cross-platform desktop) [Q4].
- Detecting Obsidian-vault changes and uploading the **raw vault** to the OKC web service via a deliberately-designed upload protocol [A1, Q5].
- Client-side pieces of the upload protocol: snapshot/manifest computation, size preflight, have/want negotiation, resumable large-file transfer, consistent-snapshot transfer, commit, offline queueing/retry, upload history.
- Cross-platform packaging, auto-update with rollback, status indication, and error notification.

### 2.2 Out of Scope (this project)
- The **web service itself** (does not yet exist) — captured here only as a **contract / downstream dependency** (§8).
- Server-side **integrate (AI) + human approval + compile** — owned by the server/user out-of-band, never by the Watcher.
- Building compiled-vault publishing from the Watcher (explicitly declined via A1=A; see §9 for the deferred advanced option).

### 2.3 Extension Configuration (from Requirements Analysis opt-in)
| Extension | Enabled | Effect on this document |
|---|---|---|
| Security Baseline | **No** [Q1=B] | SECURITY rules not enforced. Consequences captured as **explicit accepted risks** (§7). |
| Resiliency Baseline | **Yes** [Q2=A] | Applicable RESILIENCY rules mapped into requirements; cloud-only rules marked N/A (§6). |
| Property-Based Testing | **Yes (full)** [Q3=A] | Deterministic components carry mandatory PBT properties (§5.5). Framework selection is a PBT-09 obligation **deferred to the NFR Requirements stage**; **proptest** applies *if* the core is confirmed as Rust (see NFR-17). |

---

## 3. Functional Requirements

### 3.1 Monitoring & Triggering
- **FR-01** The Watcher SHALL monitor a configured Obsidian vault using **filesystem-event watching** (FSEvents / ReadDirectoryChangesW / inotify) with a **debounce quiet-period**: an upload cycle SHALL trigger when no filesystem-change event has been observed for the configured quiet period T_debounce. [Q6=B]
- **FR-02** On each triggered cycle, the Watcher SHALL recompute a **vault manifest** (per-file `{relative_path, raw_sha256, size}` + whole-vault `vault_content_id`) using the **same SHA-256 scheme as okc-core**, and diff it against the last successfully-committed manifest to determine the changed set. [Q6, Q13]
- **FR-03** The Watcher SHALL maintain a **durable on-disk queue** of pending changes that survives process crash/restart. [B1=A, Q12=A]
- **FR-04** The Watcher SHALL run a **reconciliation scan** — re-hash the full vault and compare against the last successfully-committed manifest — **(a) on startup** (recovering changes missed while the process was down) **and (b) periodically at a configurable maximum interval T_recon while running**. The periodic scan is a **backstop against runtime filesystem-event loss** (inotify `IN_Q_OVERFLOW`, FSEvents coalescing/volume drops, ReadDirectoryChangesW buffer overflow, network/remote mounts that deliver no events) that pure event-watching (FR-01) cannot otherwise catch. This bounds the maximum window in which any change goes undetected to **≤ T_recon**. [B1=A]

### 3.2 Upload Protocol (see §4 for the detailed protocol; the explicit user concern)
- **FR-05** Before any transfer, the Watcher SHALL **preflight-validate** the vault against the okc-core per-vault SafetyLimits it can meaningfully check client-side (**≤20 GiB total, ≤2 GiB/file, ≤100k files**) and **reject locally** with an actionable report if exceeded, avoiding wasted large transfers. (The **≤10-sources** cap is a server/project-level aggregation limit — one monitored vault is a single source — so it is validated server-side, not by the Watcher; see DEP-04.) The server re-validates authoritatively. [Q8=A]
- **FR-06** The Watcher SHALL upload the **raw vault** (not a compiled artifact). [A1=A, Q5]
- **FR-07** The Watcher SHALL perform **content-addressed have/want negotiation**: send the manifest, receive the set of blobs the server is missing, and upload **only missing file blobs** (whole-file granularity). [Q7=B]
- **FR-08** The Watcher SHALL support **resumable, chunked upload** (tus-style: per-chunk integrity, resume from last acknowledged offset) so an interrupted upload resumes rather than restarts. It SHALL use chunked transfer for any blob larger than a **configurable threshold S**, and for any blob whose single-request upload fails or times out; smaller blobs MAY use a single request. [Q7=C]
- **FR-09** After all wanted blobs are present server-side, the Watcher SHALL send a **commit** referencing the `vault_content_id` + manifest; materializing bytes to server disk and binding the path for okc-core ingestion is a **server responsibility** (§8). [system context]
- **FR-10** Uploads SHALL be **idempotent** and content-addressed: unchanged files re-upload as a no-op, and a repeated commit for an already-present `vault_content_id` is a no-op. [Q7, Q13]
- **FR-22** The Watcher SHALL upload from a **consistent point-in-time snapshot**: the bytes transferred for a file SHALL match the `raw_sha256` recorded in that cycle's manifest even if the vault changes mid-cycle. It SHALL achieve this by freezing/snapshotting file contents at manifest time, or by re-verifying each blob's hash at upload and re-snapshotting on mismatch — closing the time-of-check/time-of-use gap between hashing (FR-02) and transfer (FR-08). okc-core expects an immutable atomic snapshot (§1.1). [upload-protocol correctness]

### 3.3 Offline, Backpressure & Retry
- **FR-11** On failure, timeout, or server backpressure (e.g. `PROJECT_BUSY`, queue-full), the Watcher SHALL retry using the durable queue with **exponential backoff**. [Q12=A, RESILIENCY-10]
- **FR-12** While offline or backing off, the Watcher SHALL **coalesce** pending changes to the latest snapshot (a newer state supersedes older queued states for the same file). [Q12=A]

### 3.4 Authentication & Consent
- **FR-13** The Watcher SHALL authenticate to the web service using an **API token/key** issued by that service, stored in the OS secure credential store (Keychain / Credential Manager / Secret Service), and sent on every request. [Q11=A]
- **FR-14** The Watcher SHALL operate under **standing consent**: the user approves upload once during setup, after which uploads proceed automatically. The Watcher SHALL store a reference to the consent grant and SHALL expose a way to view/withdraw it. Persisting, scoping, and **enforcing** the grant is a downstream dependency (§8, DEP-02); until the server exists, withdrawal is forward-only and cannot recall already-uploaded content (RISK-01). [Q9=A]
- **FR-15** The Watcher SHALL transmit the raw vault **without a local sensitive-content preflight** (no client-side redaction/exclusion). This is an accepted risk (§7). [Q10=D]

### 3.5 History & Observability
- **FR-16** The Watcher SHALL keep a **local, append-only upload history** — per upload: `vault_content_id`/snapshot hash, timestamp, status (success / failure / partial), bytes transferred, and error detail — and SHALL make it **queryable**. [Q13=A]
- **FR-17** The Watcher SHALL emit **structured logs** of its activity and errors. [B3=A, RESILIENCY-05]
- **FR-18** The Watcher SHALL show a **status indicator** (system tray / menu-bar icon) reflecting current state (idle / syncing / offline / error). [B3=A, RESILIENCY-06]
- **FR-19** The Watcher SHALL raise an **active notification** on this **closed set** of conditions: (1) authentication failure (token rejected / HTTP 401); (2) **N consecutive** upload-cycle failures (N configurable, default 3); (3) local preflight limit-exceeded (FR-05); (4) auto-update rollback (FR-20). Non-listed transient errors SHALL NOT raise a notification (they are logged per FR-17). [B3=A, RESILIENCY-15]

### 3.6 Lifecycle & Updates
- **FR-20** The Watcher SHALL support **automatic update**, with **automatic rollback** to the previous working version if the updated version fails a post-update health check. The health check SHALL at minimum verify the new version starts, reaches the idle state, and can read the credential store within a bounded time; failing any of these SHALL trigger rollback. [B2=A, RESILIENCY-04]
- **FR-21** The Watcher SHALL run as a **long-lived background process**. *(Assumption to confirm — not derived from any answer: start automatically on user login via the per-OS mechanism. Flagged in the completion note.)* [derived from the "installed process" intent]
- **FR-23** The Watcher SHALL enforce **single-instance** execution (e.g. an OS-level lock) so that at most one instance operates on the durable queue and last-committed manifest (FR-03) at a time, preventing duplicate uploads or state corruption. [protects FR-03; long-lived process]

---

## 4. Upload Protocol Design (URP) — the explicit user concern

Goal: move potentially large vaults efficiently and resiliently, reusing okc-core's native content-addressing and honoring its whole-snapshot ingestion model. **No sub-file delta** (okc-core has no such capability; whole-file dedup is the natural, supported granularity).

**Protocol phases (per upload cycle):**

1. **Snapshot & manifest** — Capture a consistent point-in-time snapshot (FR-22) and compute per-file `{relative_path, raw_sha256, size}` and the whole-vault `vault_content_id`, matching okc-core's scheme so client and server content-ids/dedup align. [FR-02, FR-22]
2. **Preflight** — Validate against the client-checkable SafetyLimits (size/per-file/count); reject early if exceeded. [FR-05]
3. **have/want negotiation** — POST manifest → server replies with the "want" set (blobs it lacks, by `raw_sha256`). Only wanted blobs are transferred; whole-file granularity. [FR-07]
4. **Transfer** — Upload each wanted blob. Blobs at or below the configurable threshold S MAY go in a single request; blobs above S (up to 2 GiB), or any blob whose single-request upload fails/times out, use **resumable chunked** transfer (offsets, per-chunk integrity, resume from last ack). [FR-08]
5. **Commit / bind** — Once all wanted blobs are present, send a commit referencing `vault_content_id` + manifest. The server materializes to disk and binds the path for okc-core ingestion. [FR-09]
6. **Idempotency** — All steps keyed by content hash; unchanged files and repeated commits are no-ops. [FR-10]

**Text diagram (client-side view):**

```
watch/debounce -> snapshot+manifest -> preflight(limits) -> have/want -> transfer(resumable) -> commit
      |                  |                    |                |               |                  |
   FR-01           FR-02/FR-22             FR-05            FR-07       FR-08 (chunked)         FR-09
   (durable queue + startup/periodic reconciliation span the whole cycle: FR-03, FR-04, FR-11, FR-12)
```

**Design notes / rejected alternatives:**
- **Snapshot consistency** (FR-22): the bytes uploaded must match the manifest hash even if the vault changes mid-cycle; freeze at manifest time or re-verify per blob and re-snapshot on mismatch.
- Whole-archive re-upload every cycle (simplest, 1:1 with okc-core input) was **rejected as the default** because it re-sends unchanged large media. It is documented **only** as a degraded-mode contingency if have/want is unavailable — it was **not** selected in Q7 (§9).
- Sub-file binary delta (rsync-like) was **rejected**: okc-core has no sub-file chunking, so it would be all-new implementation for limited benefit at whole-file dedup granularity. [Q7 note]

---

## 5. Non-Functional Requirements

### 5.1 Performance & Efficiency
- **NFR-01** Change detection latency (event → cycle start) SHALL be governed by a configurable debounce; unchanged files SHALL NOT be re-transferred (content-addressed dedup). [Q6, Q7]
- **NFR-02** Manifest computation and hashing SHALL scale to the okc-core ceiling (100k files / 20 GiB) without unbounded memory (stream/hash incrementally). [Q8, SafetyLimits]

### 5.2 Reliability & Resilience (Resiliency Baseline — §6)
- **NFR-03** No change already **observed/enqueued or committed** SHALL be lost across crash/restart (durable queue, FR-03). Any change not captured via filesystem events SHALL be detected within the reconciliation interval **T_recon** (FR-04). Acceptance criteria: (i) zero loss for enqueued/committed changes; (ii) detection lag for un-observed changes bounded to ≤ T_recon. [B1=A, RESILIENCY-02]
- **NFR-04** All network calls SHALL have **timeouts** and **exponential-backoff** retry; the Watcher SHALL degrade gracefully offline. [Q12=A, RESILIENCY-10]

### 5.3 Portability
- **NFR-05** The Watcher SHALL run on macOS, Windows, and Linux with platform-native filesystem watching, credential storage, and update mechanisms. [Q4=C]

### 5.4 Security (Security Baseline OFF — see §7 accepted risks)
- **NFR-06** Credentials SHALL be stored in the OS secure store, and all transport SHALL be over TLS. (These minimal controls are retained even though the Security extension is off.) [Q11]
- **NFR-07** No client-side sensitive-content filtering is performed (accepted risk, §7). [Q10=D]

### 5.5 Testability — Property-Based Testing (full) [Q3=A]
Framework: **proptest** *if* the core is confirmed as Rust (NFR-17); the concrete framework is finalized at the NFR Requirements stage per PBT-09. Mandatory properties (final identification during Functional Design, per PBT-01; PBT-08 requires shrinking, fixed seeds, and CI integration):
- **NFR-08** Manifest/snapshot hashing is **deterministic** and each file's hash **equals `sha256sum`** / okc-core's `raw_sha256` for arbitrary file contents.
- **NFR-09** have/want computation is correct and idempotent: `want == manifest_paths \ server_has`; re-running with the server now holding the wanted set yields an empty want.
- **NFR-10** Resumable chunk reassembly round-trips: concatenating chunks reconstructs the original blob **byte-for-byte**, and resuming from any valid offset yields an identical result.
- **NFR-11** Coalescing preserves "latest state wins" for any interleaving of queued changes.
- **NFR-12** Reconciliation `diff(last_committed, current)` returns **exactly** the changed set for arbitrary before/after vault states.
- **NFR-13** Serialization round-trips (manifest, queue entries, history records) are lossless.
- **NFR-14** SafetyLimits validation is boundary-correct and monotonic (adding files/bytes never flips a reject back to accept).
- **Stateful (PBT-06):** the durable queue (FR-03) — a stateful, crash-recoverable, coalescing component — SHALL additionally carry a **model-based / command-sequence property** (enqueue / dequeue / coalesce / persist / recover vs a reference model). Final form in Functional Design (PBT-01/PBT-06).

### 5.6 Observability & Operability
- **NFR-15** Structured logs, a status indicator, and critical-error notifications SHALL be provided (FR-17–FR-19). [B3=A, RESILIENCY-05/06/15]
- **NFR-16** Auto-update SHALL be health-checked with automatic rollback on failure. [B2=A, RESILIENCY-04]

### 5.7 Technology (assumption to confirm at NFR Requirements / tech-stack selection)
- **NFR-17** Tech stack is **not yet finalized**. **Assumption to confirm:** the deterministic core (hashing, snapshot, manifest, dedup) is implemented in **Rust**, so it (a) can adopt **proptest** for the PBT-09 obligation and (b) can reuse okc-core's primitives (`raw_sha256`, `vault_content_id`, snapshot) directly via `okc-interop` / bindings rather than re-deriving the hashing scheme. The cross-platform desktop shell (tray, notifications, updater) may wrap that core. **Confirmed in the NFR Requirements stage.** A non-Rust decision would require reselecting a PBT-09-compliant framework.

---

## 6. Resiliency Baseline Mapping [Q2=A]

Every RESILIENCY-01..15 rule is classified below (compliant/applicable, deferred, N/A, or exempt-with-rationale).

**Applicable / deferred — reflected in requirements:**
| Rule | Where addressed |
|---|---|
| RESILIENCY-01 (critical-workload identification & prioritization) | **Applicable** — the Watcher is a single deployable component; **criticality High** (its zero-loss sync goal, NFR-03); unavailability impact = local changes not uploaded until it resumes; upstream/downstream dependency = the future web service (§1.1, §8). |
| RESILIENCY-02 (RPO / RTO / availability-SLA, data-loss durability) | **RPO**: NFR-03, FR-03, FR-04 (durable queue + reconciliation; zero loss for enqueued/committed, ≤ T_recon detection lag). **RTO / availability-SLA**: **N/A** — a single-user, user-restartable local process serves no external traffic and relaunches on demand; no availability SLA applies. |
| RESILIENCY-04 (deployment automation / rollback) | FR-20, NFR-16 (auto-update + automatic rollback with health check) |
| RESILIENCY-05 (structured logging) | FR-17, NFR-15 |
| RESILIENCY-06 (health/status) | FR-18, NFR-15 |
| RESILIENCY-10 (timeouts + backoff) | FR-11, NFR-04 |
| RESILIENCY-15 (incident/error surfacing) | FR-19, NFR-15 |
| RESILIENCY-14 (resilience testing) | Deferred to NFR Design / Operations (crash-injection, offline-simulation, resume-interruption, event-drop-simulation tests) |

**Exempt (rationale) — mandatory user-decision rule:**
| Rule | Determination |
|---|---|
| RESILIENCY-03 (change-management process) | **Exempt** — a single-user local desktop tool has no shared production change-management process to adopt; treated as exempt rather than importing an org process. *(This is a mandatory user-decision point — surfaced for confirmation in the completion note; override if a formal change process is desired.)* |

**N/A for a local desktop Watcher — belongs to the future web service (§8), not this client:**
RESILIENCY-07 (server resilience monitoring/alarms), 08 (multi-AZ/region topology), 09 (autoscaling), 11 (DR strategy), 12 (server data backup/replication — the Watcher's local state is reconstructible via the FR-04 reconciliation scan), 13 (failover runbooks). Rationale: these govern cloud production workloads; the Watcher is a single-user local process. Marking them N/A is a scoping determination, not a blocking gap.

---

## 7. Accepted Risks (explicit — Security Baseline OFF) [Q1=B, Q10=D, Q9=A, A1=A]

- **RISK-01 — Source disclosure / exfiltration (continuous & irreversible).** With Security OFF [Q1=B], **no client-side sensitive-content preflight** [Q10=D], **raw-vault** upload [A1=A], and **standing consent** [Q9=A], the entire vault — including any secrets, API keys, or personal data it contains — is transmitted to the server. This deliberately bypasses okc-core's own guardrail (local sensitive preflight + per-call, non-persisted consent) **at the Watcher upload boundary**.
  - **Continuous:** under standing consent + auto-sync, **every future vault change auto-uploads** under the one-time grant, with no further prompt — including secrets added later.
  - **Irreversible & no working revocation yet:** once uploaded, content **cannot be recalled**; FR-14's view/withdraw affordance has **no enforcing backend until the server exists** (DEP-02), so revocation is forward-only. Until then there is no effective way to stop or undo disclosure of already-uploaded content.
  - **Residual controls (accurately scoped):** **TLS** [NFR-06] mitigates *in-transit interception* only. The **API token** [Q11] controls *who may upload* (endpoint auth) but does **not** reduce disclosure to the server it is authorized to send to — it is not a mitigation for this risk, and the token is itself a stored credential whose theft is outside the retained controls.
  - **Local plaintext artifacts:** with Security OFF, the Watcher persists on disk **outside the OS secure store** — upload history (hashes + free-form error detail, FR-16), the manifest (`relative_path` + `raw_sha256`, FR-02), the durable queue (FR-03), and structured logs (FR-17). File paths can be sensitive, error/log content can embed secrets on failure, and SHA-256 of low-entropy secrets is dictionary-reversible. This local disclosure surface is unprotected and accepted.
  - **Server-side nuance:** okc-core's **non-overridable** block on remote disclosure of sensitive content still applies **server-side, at the AI `integrate` stage** (when the server sends content to a remote AI provider) — so sensitive docs may be blocked from *remote-AI* disclosure downstream, but this does **not** stop the Watcher uploading them and does **not** apply to non-AI server storage.
  - **Status:** Accepted by the user for this project's scope. Revisit if the vault may contain regulated/secret data or if Security Baseline is later enabled.

- **RISK-02 — Fail-closed re-approval thrash undercuts unattended sync.** okc-core is **fail-closed on content change**: every changed-vault submission (new `vault_content_id`) invalidates the server's prior `ApprovedIntegrationPlan`, forcing a fresh human `integrate` + per-cluster re-approval (§1.1). Under standing consent [Q9=A] + event-triggered auto-upload [Q6=B], **each debounced edit that uploads a new vault state invalidates the last human approval** — so frequent editing produces continual re-approval demand on the server side, materially weakening the "unattended background sync" value proposition.
  - **Partial mitigations already in scope:** debounce (FR-01), coalescing (FR-12), and content-addressed no-op commits (FR-10) reduce upload frequency; the periodic-not-per-keystroke cadence limits churn.
  - **Downstream requirement:** the server must tolerate high-churn re-approval (e.g. batch/queue approvals, or decouple raw-vault storage from approved-plan lifecycle); optionally the Watcher may throttle/batch uploads (DEP-07, §8).
  - **Status:** Accepted/known; the Watcher cannot resolve it alone because approval is a server + human concern.

---

## 8. Downstream Dependencies — Future Web Service (contract) [not built yet]

These are requirements **on the not-yet-built server** implied by the Watcher's design; the Watcher cannot function correctly without them:

- **DEP-01** Inbound **authentication**: issue/rotate/revoke API tokens (okc-core has none). [Q11]
- **DEP-02** **Persist, scope, and enforce standing consent** and provide a revocation path (okc-core default is per-call, non-persisted). [Q9, RISK-01]
- **DEP-03** **Upload protocol endpoints**: manifest/have-want negotiation, resumable chunked blob upload, and commit; then **materialize bytes to server disk** and bind the path for okc-core ingestion. [§4, FR-07–FR-09]
- **DEP-04** **Authoritative SafetyLimits** re-validation server-side, including the project-level **≤10-sources** cap. [Q8, FR-05]
- **DEP-05** **Multi-tenancy / isolation** if serving more than one user (okc-core has none). [system context]
- **DEP-06** Ownership of **integrate (AI) + human approval + compile** — never performed by the Watcher. [compilation probe]
- **DEP-07** **Tolerate high-churn re-approval** driven by auto-sync (batch/queue approvals or decouple raw-vault storage from approved-plan lifecycle). [RISK-02]

---

## 9. Deferred / Future Options
- **Compiled-vault publishing from the Watcher** — declined for now [A1=A]. If ever added, it is a **separate advanced opt-in** requiring the Watcher to act as a full OKC node that has already run integrate + human approval + compile locally; it carries no privacy benefit over raw upload and drops non-Markdown content, so it is explicitly not part of this baseline. [A1 option C]
- **Whole-archive fallback** upload path when have/want negotiation is unavailable — **not selected in Q7**; documented only as a degraded-mode contingency for provenance. [Q7]
- **Resilience testing** (crash injection, offline/resume simulation, event-drop simulation) — NFR Design / Operations. [RESILIENCY-14]

---

## 10. Requirement → Decision Traceability

| Decision | Answer | Requirements |
|---|---|---|
| Q1 Security extension | B (off) | §7 accepted risks; NFR-06/07 |
| Q2 Resiliency baseline | A (on) | §6 mapping; NFR-03/04/15/16 |
| Q3 PBT | A (full) | NFR-08…14 + PBT-06 queue property; framework per NFR-17 |
| Q4 Target OS | C (mac/Win/Linux) | NFR-05 |
| Q5 / A1 Upload target | A (raw vault) | FR-06; §9 |
| Q6 Trigger | B (event + debounce) | FR-01, FR-04 |
| Q7 Upload strategy | B baseline + C | FR-07, FR-08; §4 |
| Q8 Size-cap handling | A (preflight reject) | FR-05; NFR-02; DEP-04 |
| Q9 Consent | A (standing) | FR-14; DEP-02; RISK-01, RISK-02 |
| Q10 Sensitive content | D (no check) | FR-15; NFR-07; RISK-01 |
| Q11 Auth | A (API token) | FR-13; NFR-06; DEP-01 |
| Q12 Offline/backpressure | A (queue+backoff+coalesce) | FR-11, FR-12; NFR-04 |
| Q13 History | A (local history + query) | FR-16; FR-02 |
| B1 Crash/restart durability | A (persistent queue + scan) | FR-03, FR-04; NFR-03 |
| B2 Update/rollback | A (auto + rollback) | FR-20; NFR-16 |
| B3 Error surfacing | A (log + tray + alert) | FR-17…19; NFR-15 |

**Derived (not from a Q-answer) — flagged for confirmation:**
| Item | Basis |
|---|---|
| FR-21 (autostart-on-login) | **Assumption** from "installed process" intent — confirm or drop |
| FR-22 (consistent snapshot) | okc-core immutable-atomic-snapshot requirement (§1.1); TOCTOU correctness |
| FR-23 (single-instance lock) | Protects the durable queue (FR-03) for a long-lived process |
| FR-04 periodic reconciliation | Implements B1=A's zero-loss goal as a backstop to Q6=B event-watching |
| RESILIENCY-03 exempt | Mandatory user-decision point — confirm exemption or request a change process |
| RISK-02 (re-approval thrash) | Consequence of Q9=A + Q6=B against okc-core's fail-closed model |

---

## 11. Open Items for Later Stages
- Tech stack confirmation (NFR-17) — **NFR Requirements** stage.
- Concrete defaults: debounce quiet-period T_debounce, reconciliation interval T_recon, backoff schedule, chunk size, chunked-transfer threshold S, health-check timeout — **Functional / NFR Design**.
- Final PBT property list + generators, incl. the stateful queue property — **Functional Design** (PBT-01/PBT-06).
- Server contract detail (endpoint shapes, error codes, re-approval batching) — coordinated when the web service is designed.
- **Confirm-or-override items** surfaced from self-review: FR-21 autostart, RESILIENCY-03 exemption (see §6 and the completion note).

---

## 12. FQ-3=A 경미 수정 부록 (Application Design 정합)

**추가일**: 2026-09-08 · **근거**: Application Design 단계의 확정 답변 FQ-1=A(okc-core 코드 의존성 0), FQ-2=A(최신 상태 대체 모델), FQ-3=A(영향 요구사항을 단순 모델에 맞게 **경미 수정**).

> 아래는 §3~§10의 **원문을 대체하지 않는 오버레이**다. 원문은 사용자 승인 기록으로 보존하고, 단순화된 아키텍처와 어긋나는 문구만 각 ID별로 정정한다. 요구사항의 **의도(intent)는 모두 불변**이며, 달성 **메커니즘 표현**만 바뀐다.

### 12.1 FQ-1=A — okc-core 코드 의존성 제거 (표준 SHA-256)

| ID | 원문 표현 | FQ-3 정정 |
|---|---|---|
| **FR-02** | "okc-core와 동일한 SHA-256 스킴", 볼트 전체 `vault_content_id` 산출 | 클라이언트는 **표준 SHA-256(= `sha256sum`)** 으로 파일별 `{relative_path, raw_sha256, size}` + 로컬 멱등/no-op 판정용 `manifest_digest`만 계산한다. **권위 있는 `vault_content_id` 계산은 서버 소유**(FQ-1=A). 값은 sha256sum과 동일하므로 서버 dedup 정렬은 유지된다. |
| **NFR-08** | 해시가 "okc-core의 `raw_sha256`와 일치" | 해시가 **표준 `sha256sum`과 일치**(okc-core 재현/인용 삭제). 프로퍼티는 동일하게 유효. |
| **NFR-17** | "okc-core 프리미티브(`raw_sha256`, `vault_content_id`)를 `okc-interop`/바인딩으로 재사용" | **okc-core 의존성 0.** 결정적 코어는 표준 SHA-256을 Rust로 직접 구현(proptest 채택 근거는 불변). `okc-interop`/바인딩/골든벡터 하네스는 설계에서 **제거**. |
| **§5.5 NFR-09** | have/want 정확성 | 세부 키잉은 **Q6=C**: 전송은 `raw_sha256` 콘텐츠 중복제거, 커밋은 권위 있는 **경로→해시 맵 + `manifest_digest`**. 프로퍼티(`want == manifest_paths \ server_has`, 재실행 시 빈 want)는 불변. |

### 12.2 FQ-2=A — 최신 상태 대체 모델 (이벤트 큐 제거)

| ID | 원문 표현 | FQ-3 정정 |
|---|---|---|
| **FR-03** | "대기 중 변경의 **지속 온디스크 큐**" | **최신 상태 대체(latest-state-replacement)** 로 대체. 지속 상태 = **마지막 커밋 매니페스트 1개 + dirty 표시 + 재개 오프셋**(`SyncStateStore`). 이벤트별 지속 큐 없음. 매 트리거마다 폴더를 재스냅샷→diff(폴더가 진실의 원천). **무손실 의도(NFR-03)는 불변** — 크래시/재시작 후 dirty·재개 오프셋 복구 + 다음 재스냅샷이 미반영 편집을 자동 흡수. |
| **FR-12** | 오프라인/백오프 중 "대기 변경을 최신 스냅샷으로 **coalesce**" | 별도 coalesce 엔진 없음. 매 사이클 폴더 재스냅샷이 곧 최신 상태이므로 **"latest-state-wins"가 구조적으로 자동 성립**. 오프라인이면 사이클이 백오프로 재시도하고 그 사이 편집은 다음 스냅샷에 자연 포함. |
| **§5.5 NFR-11** | "Coalescing이 임의 인터리빙에 대해 latest-state-wins 보존" | 프로퍼티는 유효하되 대상이 **`SyncStateStore` 상태 전이**(재스냅샷→diff→커밋)로 이동. 큐 인터리빙이 아니라 "여러 편집 후 커밋된 상태 = 마지막 폴더 상태" 불변식. |
| **§5.5 NFR-13** | 직렬화 라운드트립: "manifest, **queue entries**, history records" | 대상에서 **queue entries 제거** → **manifest + sync-state(마지막커밋+dirty+재개오프셋) + history records** 라운드트립. |
| **§5.5 PBT-06 (stateful)** | "durable queue enqueue / dequeue / **coalesce** / persist / recover vs 참조 모델" | 상태 기반 프로퍼티 대상이 **`SyncStateStore` 상태머신**으로 이동: `commit_manifest / mark_dirty / set_resume_offset / persist / recover` 명령 시퀀스 vs 참조 모델. 크래시/재시작 무오염·무손실 보증 의도는 불변. |

### 12.3 불변(정정 없음) 확인
FR-01·FR-04(트리거·재조정), FR-05~FR-11·FR-22(업로드 프로토콜·프리플라이트·재검증), FR-13~FR-23, NFR-01~07·10·12·14~16, §6 Resiliency 매핑, §7 RISK-01/02, §8 DEP-01~07은 단순화 모델과 **정합하며 정정 없음**. §4 URP 프로토콜 6단계도 불변(전송/커밋 키잉은 Q6=C로 이미 정합).

**적용 결과**: 이 부록은 §14 이하 산출물(`application-design.md` §3의 FQ 매핑, `component-dependency.md` §5 네이밍 레지스트리의 "제거된 초안 컴포넌트")과 일관된다.

---

## 13. 토큰 저장 위치 확정 (Q11 정제)

**추가일**: 2026-09-08 · **결정**: 사용자 확정 "config에 둬" + 후속 선택 "config 기본 + 보안저장소 opt-in 유지". stories.md **미결 항목 #2**(토큰 저장 경로 confirm-or-drop)를 해소한다.

> §3.4 FR-13, §5.4 NFR-06의 **원문(승인본)을 대체하지 않는 오버레이**다. 원문은 "OS 보안 자격증명 저장소"를 저장 위치로 명시하나, nginx식 단일 JSON config 실행 모델(Q3=X) 확정에 따라 아래로 정정한다.

| ID | 원문 표현 | 확정 정정 |
|---|---|---|
| **FR-13** | 토큰을 "**OS 보안 자격증명 저장소**(Keychain / Credential Manager / Secret Service)"에 저장 | **1차·기본 저장 위치 = 단일 JSON config의 평문 `token` 필드**(+ env 변수 폴백; keyring-less). OS 보안 저장소는 **선택적 opt-in 강화 경로**로 유지(US-E4-02) — `secure_store` 옵션 on + 데스크톱 세션 가용 시에만 사용하고, 불가(헤드리스/데몬) 시 config/env로 안전 폴백. 매 요청 토큰 전송·발급 주체(서버)는 불변. |
| **NFR-06** | "Credentials SHALL be stored in the **OS secure store**, and all transport SHALL be over TLS." | **토큰 기본 저장 = config 평문 필드**(Security OFF 하 **RISK-01 수용**); OS 보안 저장소는 선택적 강화. **TLS 강제는 불변**(잔존 통제로 유지). |

**RISK-01 영향(§7)**: config 평문 `token`은 로컬 평문 산출물 집합에 **포함**된다(경로/히스토리/매니페스트/SyncState/로그 + **config 토큰**). 토큰 탈취·평문 노출은 Security OFF 하 명시적 수용 위험이며, 제거 시 정리 대상(US-E6-04, `Uninstaller`)이다.

**정합 확인**: 설계 5종(`components.md` CredentialProvider = config/env 1차 + secure-store 선택, `component-methods.md` `TokenSource{Config,Env,SecureStore}`·`resolve_token` 폴백)·`stories.md`(US-E4-01 config 1차 / US-E4-02 선택적 강화)·`personas.md`·`execution-plan.md` §6-B는 **이미 이 구조와 정합**하므로 정정 불필요. 이 부록은 그 구조를 요구사항 수준에서 확정한다.
