# Application Design Plan — As-Built

## Context

This plan documents existing component and service boundaries. It does not
select a new architecture. Questions are unnecessary because current
specifications, Cargo dependencies, public APIs, and the user's batch
continuation request fully constrain the result.

## Execution checklist

- [x] Read current requirements, reverse-engineering artifacts, architecture specification, and traceability.
- [x] Identify compiler, AI, application, interop, CLI/TUI, and language-adapter components.
- [x] Record component responsibilities and prohibited ownership.
- [x] Record current public method groups and schema boundaries.
- [x] Model application services and orchestration flows.
- [x] Build the internal dependency matrix and state/data-flow diagrams.
- [x] Generate `components.md`.
- [x] Generate `component-methods.md`.
- [x] Generate `services.md`.
- [x] Generate `component-dependency.md`.
- [x] Generate consolidated `application-design.md`.
- [x] Validate that no future adapter, Pack, non-Markdown materializer, or proposed ADR is presented as current.

## Constraints

- Dependencies point inward to `okc-core`.
- Providers have proposal authority only.
- `okc-app` owns shared CLI/TUI workflow and project state.
- `okc-interop` owns runtime-neutral binding semantics.
- Adapters do not own canonical state or compilation policy.

## Output

All generated artifacts live in `aidlc-docs/inception/application-design/`.
