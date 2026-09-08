# Requirements — Obsidian MCP for authoring an OKC input Vault

**Status**: Requirements Analysis output — **awaiting explicit user approval**. This document incorporates all answered verification (11), clarification (2), and resiliency-extension (2) questions. It supersedes the draft at [`../requirements.md`](../requirements.md). The **product / Construction gate remains in force**: approving this document approves the requirements only, not implementation, testing, or packaging.

---

## 1. Intent Analysis

- **User request**: Build an installable local Obsidian MCP that authors the best possible *input* Vault for OKC — i.e. a source-Vault authoring tool, not an adapter for querying compiled OKC results. Proceed from Inception through implementation under AWS AI-DLC (v1.0.1 classic).
- **Request type**: New product (brownfield workspace — unapproved draft code in `src/`/`tests/` exists but must not be used to infer or retroactively fix scope).
- **Scope estimate**: Multiple components within a single deliverable (authoring, updates, discovery, quality audit, installation/diagnostics).
- **Complexity estimate**: **Complex** — security-critical trust boundary, in-place mutation of real user files, YAML/frontmatter preservation, and correctness of the OKC-input contract.
- **Requirements depth**: Comprehensive.

---

## 2. Confirmed Product Direction

An installable, local, **single-Vault**, **filesystem-direct** (stdio) Obsidian MCP that helps a user author and tidy Markdown notes so they are the best possible *input* for OKC. It respects existing Vaults, never performs destructive file operations, treats note content as untrusted data, and hands the source path to OKC — which owns snapshotting, integration, review, and compilation.

The **first Unit's primary user journey is organizing/structuring existing notes**, scoped to **in-note tidying only** (see REQ-013). File-level reorganization (move/rename/merge) is intentionally deferred to a follow-up Unit.

---

## 3. Decision Log

| Ref | Decision | Source |
|---|---|---|
| D1 | Single Vault in the first version; multi-Vault authoring/comparison is an **explicit follow-up Unit**. | Verify Q1=C |
| D2 | Filesystem-direct access over stdio; Obsidian runtime / Dataview / plugin integration is **not required**. | Verify Q2=A |
| D3 | Node/TypeScript stdio MCP installed via **local npm tarball**. | Verify Q3=A |
| D4 | First-Unit top-priority journey = **organizing/structuring existing notes**, limited to in-note tidying (no move/rename/merge). | Verify Q4=B + Clarify Q1=A |
| D5 | **Improve existing Vaults first**; no forced migration; shallow folder structure is optional. | Verify Q5=A |
| D6 | Quality auditing is **heuristic** in the first Unit; real OKC ingestion testing is a follow-up Unit. | Verify Q6=A |
| D7 | **No delete/move/rename** tools; safe partial updates only; conflict-aware writes + external backup. | Verify Q7=A |
| D8 | Adopt all **5 success criteria** (Section 6) as-is. | Verify Q8=A |
| D9 | **security-baseline extension disabled** as a blocking ruleset — but the product's own security requirements **REQ-008 / REQ-011 remain fully in scope**. | Verify Q9=B + Clarify Q2=A |
| D10 | **resiliency-baseline extension enabled** as directional design-time guidance (Section 9). | Verify Q10=A |
| D11 | **Property-based testing: partial** — PBT applied to pure functions + serialization round-trips only. | Verify Q11=B |
| D12 | **Recovery/data protection**: single pre-change backup outside the Vault + manual recovery; no generational retention or auto-cleanup (RPO = last save point, RTO = manual). | Resiliency Q1=A |
| D13 | **Release governance**: lightweight — version tagging + CHANGELOG + release notes. | Resiliency Q2=A |

---

## 4. Functional Requirements

| ID | Requirement | Acceptance criterion |
|---|---|---|
| REQ-001 | Installable local stdio MCP | Works without the Obsidian app, a REST plugin, an API key, or an OKC binary. Single Vault per session (D1, D2, D3). |
| REQ-002 | Respect existing Vaults | Connects without relocating folders or requiring migration to custom frontmatter (D5). |
| REQ-003 | Knowledge authoring | Creates Markdown with a title and body plus optional aliases, tags, and source; refuses to replace an existing file. |
| REQ-004 | Conflict-aware updates + backup | Rejects a mismatched `expectedHash`; before any mutation, writes a single pre-change backup **outside** the Vault. Recovery is manual; no generational retention/auto-cleanup (D12). |
| REQ-005 | Preserve structure | Partial frontmatter updates preserve unknown keys, body content, and YAML comments. Malformed YAML is rejected. |
| REQ-006 | Discovery | Deterministic path listing, literal search including Korean text, and reads that include a content hash. |
| REQ-007 | Audit OKC input (heuristic) | Reports YAML, path, link, duplicate, operational-noise, and unsupported-format issues **without** claiming compiler validation passed (D6). |
| REQ-008 | Bounded authority | Rejects access outside the registered Vault, symlinks, hardlinks, and hidden control paths; bounds file size, file count, and response size. **In scope regardless of D9.** |
| REQ-009 | Separate knowledge and tool state | Keeps configuration, backups, and templates outside the input Vault. Compiled artifacts and managed projects are not connected as editable Vaults. |
| REQ-010 | Reviewable installation | Configuration generation, diagnostics/self-check, absolute-path client-config output, and local tarball installation. |
| REQ-011 | Trust boundary | Treats notes as untrusted data. Exposes no internal AI invocation, automatic approval, shell, arbitrary HTTP, or delete tool. **In scope regardless of D9.** |
| REQ-012 | Lifecycle records | Records research, design, implementation, testing, operations/release status, and follow-up work in files. |
| REQ-013 | In-note organization (first-Unit primary journey) | Standardizes frontmatter (`title`/`aliases`/`tags`), fixes invalid YAML, reinforces sources and links, and audits/flags quality issues — **all within existing files**. Performs **no** file move, rename, or merge (those are follow-up Units) (D4, D7). |

---

## 5. Non-Functional Requirements

### 5.1 Security (product-level — enforced even though the security extension is off, per D9)
- **NFR-SEC-1**: All filesystem access is confined to the registered Vault; symlinks, hardlinks, and hidden control paths are rejected (REQ-008).
- **NFR-SEC-2**: Note content is untrusted input; no tool exposes shell execution, arbitrary HTTP, deletion, auto-approval, or internal AI invocation (REQ-011).
- **NFR-SEC-3**: File size, file count, and response size are bounded to prevent resource exhaustion (REQ-008).

### 5.2 Data protection & recovery (RESILIENCY-02 / -11 / -12, per D12)
- **NFR-DR-1**: Every mutation is preceded by an automatic backup written outside the Vault (RPO = last saved state).
- **NFR-DR-2**: Recovery is a documented manual restore from that backup (RTO = manual). No generational retention or automatic cleanup in the first Unit.
- **NFR-DR-3**: Conflict-aware writes (`expectedHash`) prevent silent overwrite of concurrently changed files.

### 5.3 Correctness & testing (per D11)
- **NFR-TEST-1**: Pure functions and serialization round-trips (YAML/frontmatter parse↔serialize, partial-update merge, content hashing) are covered by **property-based tests**.
- **NFR-TEST-2**: The quality audit is heuristic and must never assert that OKC compiler validation passed (REQ-007).

### 5.4 Portability & installation (per D3, D13)
- **NFR-OPS-1**: Installs from a local npm tarball on the target user's OS; a diagnostics command validates the environment and emits absolute-path client configuration.
- **NFR-OPS-2**: Releases are governed by lightweight version tagging + CHANGELOG + release notes (RESILIENCY-03).

### 5.5 Resource bounds & dependency isolation (RESILIENCY-09 / -10 analog)
- **NFR-PERF-1**: Filesystem operations are bounded (size/count/response) and must not block unboundedly; the only external dependency is the local filesystem — no network calls are made.

---

## 6. Success Criteria (adopted as-is, per D8)

1. A new user can install from documentation alone and author a first note in a chosen Vault.
2. An existing-Vault user can understand problems and proposed improvements without forced migration.
3. Knowledge can be added or updated without losing sources, conflicting claims, or links.
4. Input-structure problems and unnecessary collection noise are reduced when the Vault is handed to OKC.
5. Users can review changes, identify conflicts, and recover earlier content.

> "Best Vault" is assessed through OKC input quality and authoring UX in representative synthetic or consented user scenarios — not by folder preference or tool count.

---

## 7. Scope: First Unit vs Follow-up Units

**First Unit** — single-Vault Markdown authoring, updates, and heuristic quality audit, with the **primary journey being in-note organization** (REQ-013). Includes REQ-001..013 at the depth described above.

**Follow-up Units** (priority may change after review):
1. Rename/move/merge and recovery UX that preserves links (file-level reorganization deferred from D4/D7).
2. Multi-Vault authoring and comparison (deferred from D1).
3. Integration with real OKC ingestion / compiler corpus checks (deferred from D6).
4. Performance and OS validation on real user Vaults.
5. Publication and update/distribution paths beyond the local tarball.

---

## 8. Exclusions (first Unit)

Semantic search; Obsidian UI / Dataview execution; general-purpose deletion, rename, or move; automatic taxonomy moves; remote HTTP; provider execution; OKC review approval; attachment / Canvas / Base conversion; complete Obsidian link interpretation; multi-Vault operation. Authoring authority over live sources and immutability of OKC-sealed snapshots belong to different lifecycle stages.

---

## 9. Resiliency Extension Compliance Summary (RESILIENCY baseline — enabled per D10)

This is a **local, single-process, offline** authoring tool with no cloud deployment, no long-running service, and no multi-user/production runtime. Infrastructure-level rules are therefore N/A; data-protection rules apply to the user's Vault.

| Rule | Status | Rationale |
|---|---|---|
| RESILIENCY-01 (critical workload id) | Compliant (light) | Data-integrity-critical for the Vault it mutates; availability non-critical (on-demand local process). |
| RESILIENCY-02 (recovery targets) | Compliant | D12: RPO = last save point (pre-change backup), RTO = manual restore. No cloud availability SLA (N/A). |
| RESILIENCY-03 (change management) | Compliant | D13: lightweight version tag + CHANGELOG + release notes. |
| RESILIENCY-04 (deploy/rollback) | Deferred → NFR/Infra Design | Local tarball; rollback = reinstall prior version. Mostly N/A. |
| RESILIENCY-05 (monitoring/alerting) | N/A | No deployed service/observability platform; diagnostics command covers local self-check. |
| RESILIENCY-06 (health checks) | N/A | No service endpoints/load balancer; diagnostics ≈ shallow self-check. |
| RESILIENCY-07 (resiliency monitoring) | N/A | No deployed workload. |
| RESILIENCY-08 (multi-zone/region) | N/A | No cloud deployment. |
| RESILIENCY-09 (auto-scaling/capacity) | N/A | Single local process; resource bounds via REQ-008. |
| RESILIENCY-10 (dependency isolation) | Compliant (light) | Only external dependency is the local filesystem; no arbitrary HTTP (REQ-011); bounded operations (REQ-008). Circuit breakers N/A. |
| RESILIENCY-11 (DR strategy) | Compliant | Backup-and-Restore analog (manual), per D12. |
| RESILIENCY-12 (backup/replication) | Compliant | Automated pre-change backup outside the Vault (REQ-004); single generation (D12); cross-region N/A; backup encryption deferred to design (local, user-controlled). |
| RESILIENCY-13 (failover/recovery procedures) | Deferred → design | Recovery runbook = manual restore from external backup; documented at design. |
| RESILIENCY-14 (chaos/DR testing) | Deferred → NFR Design | PBT (partial) covers transform/round-trip resilience. |
| RESILIENCY-15 (incident response) | Deferred → NFR Design | Extension permits asking at NFR Design. |

No blocking resiliency findings at the Requirements stage.

---

## 10. Extension Configuration Recap

- **security-baseline** — Disabled (D9). Product security requirements REQ-008 / REQ-011 remain in scope.
- **resiliency-baseline** — Enabled (D10) as directional design-time guidance (Section 9).
- **property-based-testing** — Enabled: Partial (D11) — pure functions + serialization round-trips.
