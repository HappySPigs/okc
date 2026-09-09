# Reverse Engineering — Interaction Diagrams

## Transaction 1: project integration and review

```mermaid
sequenceDiagram
    actor Curator
    participant Surface as CLI TUI or SDK
    participant Project as ProjectStore
    participant Service as IntegrationService
    participant Provider as ProviderClient
    participant Core as okc-core
    Curator->>Surface: Select project sources and routes
    Surface->>Project: Persist validated source and route revisions
    Surface->>Service: Preflight
    Service->>Core: Build PreparedCorpus
    Service->>Service: Scan blocks and metadata
    Surface->>Service: Integrate with explicit remote consent
    Service->>Provider: Embedding and organizer requests
    Provider-->>Service: Structured responses
    Service->>Project: Append recordings and tasks
    Service->>Core: Validate taxonomy proposal
    Curator->>Surface: Approve complete taxonomy hash
    Surface->>Service: Record taxonomy approval
    Service->>Provider: Synthesis and critic requests per cluster
    Provider-->>Service: Proposals and findings
    Service->>Core: Validate complete evidence closure
    Curator->>Surface: Approve omissions waivers and cluster
    Surface->>Project: Append hash-bound approvals
    Service->>Core: Seal ApprovedIntegrationPlan
```

Text alternative: all source/config mutations are validated and invalidate
dependent authority; preflight precedes provider calls; every provider result
is recorded; taxonomy and each cluster require separate exact approvals before
the integration plan can be sealed.

## Transaction 2: offline compile, verify, and explain

```mermaid
sequenceDiagram
    actor Caller
    participant App as okc-app or direct Rust caller
    participant Core as okc-core
    participant Stage as Sibling staging directory
    participant Output as Final absent destination
    Caller->>App: Compile approved plan
    App->>Core: Validated ApprovedIntegrationPlan
    Core->>Stage: Create new stage and materialize files
    Core->>Stage: Synchronize and independently verify
    Core->>Output: Atomic no-replace publication
    Core-->>App: CompiledVaultManifest
    Caller->>App: Verify output
    App->>Core: Verify complete plan-derived inventory
    Core-->>Caller: Manifest
    Caller->>App: Explain one safe output path
    App->>Core: Verify whole artifact then select record
    Core-->>Caller: ProvenanceRecord
```

Text alternative: compile validates one complete plan, writes and verifies a
sibling stage, then atomically publishes only if the requested destination is
absent. Verify and explain reconstruct allowed bytes from the embedded plan;
explain verifies the whole artifact before returning one record.

## Transaction 3: Python/Node job execution

```mermaid
sequenceDiagram
    actor Host as Python or Node host
    participant Adapter as Thin native adapter
    participant Interop as okc-interop
    participant Scheduler as Bounded scheduler
    participant App as okc-app
    Host->>Adapter: Submit explicit-path operation
    Adapter->>Interop: Validate DTO and path
    Interop->>Scheduler: Reserve project and enqueue
    Scheduler->>App: Run operation with cancellation token
    App-->>Scheduler: Progress and terminal result
    Scheduler-->>Interop: Retain at most 64 events and separate result
    Interop-->>Adapter: Typed result or structured error
    Adapter-->>Host: snake_case or camelCase projection
```

Text alternative: language hosts submit absolute-path operations; interop
validates and schedules them with same-project exclusion; progress is bounded
and the terminal result is retained independently; adapters only translate
names/types/errors.

## Failure interactions

- A source or configuration change starts a fresh active run and makes old
  approvals unavailable; it does not edit historical objects.
- A remote cache miss without both required consent booleans fails before
  disclosure.
- Critical/major critic findings block approval; every minor finding needs an
  exact waiver.
- Cancellation is honored before publication and becomes too late at/after
  the barrier.
- Any unplanned file, symlink, malformed control file, stale hash, or publish
  race fails closed without replacing the winner.
