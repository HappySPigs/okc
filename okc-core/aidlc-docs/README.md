# OKC AI-DLC Documentation

This directory is the retrospective AI-DLC record for the current OKC Schema 3
development product. It documents the code that exists at commit
`e448bf28ee3dcb43d18428eb1bdb9ac70163bf34`; it does not define a second product
contract and does not promote future work into the current compiler.

## Authority

When this directory differs from the repository contract, use the precedence in
[`AGENTS.md`](../AGENTS.md): current specifications, stable algorithms, then
accepted ADRs. Current implementation and release evidence are tracked in
[`docs/CURRENT_STATE.md`](../docs/CURRENT_STATE.md) and
[`docs/TRACEABILITY.md`](../docs/TRACEABILITY.md).

The labels used throughout these artifacts are deliberate:

- **Implemented** means current source and direct evidence exist.
- **Partial** means a current slice exists but one or more named gates remain open.
- **Absent** means the public product correctly does not expose the capability.
- **Future** means a normative-future requirement exists.
- **Proposed** means ADR-0028 through ADR-0031 have not been accepted or implemented.

## Lifecycle index

### Inception

- [Requirements](inception/requirements/requirements.md)
- [Reverse-engineering index](inception/reverse-engineering/overview.md)
- [Execution plan](inception/plans/execution-plan.md)
- [Application design](inception/application-design/application-design.md)
- [Unit of work](inception/application-design/unit-of-work.md)

User Stories is explicitly skipped because this continuation is an as-built,
documentation-only task. Existing session-grounded functional requirements are
mapped directly to the single product unit in
[`unit-of-work-story-map.md`](inception/application-design/unit-of-work-story-map.md).

### Construction

- [Construction index](construction/README.md)
- [Functional design](construction/okc-schema3-product/functional-design/business-logic-model.md)
- [NFR requirements](construction/okc-schema3-product/nfr-requirements/nfr-requirements.md)
- [NFR design](construction/okc-schema3-product/nfr-design/nfr-design-patterns.md)
- [As-built implementation record](construction/okc-schema3-product/code/implementation-summary.md)
- [Build and test summary](construction/build-and-test/build-and-test-summary.md)

Infrastructure Design is explicitly skipped: OKC is a local native
library/application workspace and this task introduces no cloud or deployed
service infrastructure. CI and packaging topology are documented as build
infrastructure, not invented as a runtime architecture.

## Checkout layout

The current checkout has two relevant roots:

| Root | Current path | Purpose |
|---|---|---|
| Git checkout | `/Users/sihun/workspace/projects/okc` | Git metadata and GitHub Actions workflows |
| Cargo/product workspace | `/Users/sihun/workspace/projects/okc/okc-core` | application source, normative docs, guide, tests, and this AI-DLC record |

The workflow files at `../.github/workflows/` set their default working
directory to `okc-core`. Commands in the construction documents are run from
the Cargo/product workspace unless a step explicitly says otherwise.

## Current product boundary

The implemented artifact is a Schema 3 Markdown directory containing canonical
notes, redirect stubs, and four `.okc/` audit files. Attachment/Canvas/Base
materialization, complete link rewriting, a current OKCPack writer, complete
semantic scale, and stable release qualification remain open. Their presence in
an historical ADR or proposed design is not implementation evidence.
