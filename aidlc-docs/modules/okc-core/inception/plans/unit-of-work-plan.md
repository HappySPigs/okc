# Unit of Work Plan — As-Built

## Decomposition decision

Use one development unit named `okc-schema3-product`. OKC is a modular local
native product with multiple packages, not a set of independently deployed
services. Keeping a single unit preserves the shared Schema 3 compatibility,
approval, deterministic-byte, and release-gate boundary.

## Module boundaries inside the unit

1. Deterministic compiler core (`okc-core`).
2. Provider capabilities and adapters (`okc-ai`).
3. Project/application orchestration (`okc-app`).
4. Runtime-neutral language facade (`okc-interop`).
5. Operator interface (`okc`).
6. Python adapter/package (`okc-python`).
7. Node.js adapter/package (`okc-node`).

## Execution checklist

- [x] Read requirements and application-design artifacts.
- [x] Confirm the system is not a microservice deployment.
- [x] Select the single-product unit boundary.
- [x] Preserve the seven internal package responsibilities.
- [x] Document inward dependencies and integration checkpoints.
- [x] Map every session FR and current REQ area to the unit/modules.
- [x] Generate `unit-of-work.md`.
- [x] Generate `unit-of-work-dependency.md`.
- [x] Generate `unit-of-work-story-map.md` using requirements because User Stories was skipped.
- [x] Validate that no requirement is assigned to an absent future component as implemented.

## Generation order

For source analysis and any future compatible change: core, AI, app, interop,
CLI/TUI and bindings, then integrated verification. This task changes no
application source.
