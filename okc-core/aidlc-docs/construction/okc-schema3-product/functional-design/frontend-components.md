# Functional Design — CLI/TUI Frontend Components

OKC has a terminal frontend, not a web frontend. HTML-specific test IDs and
browser state patterns do not apply.

## CLI component tree

| Component | Commands/responsibility |
|---|---|
| Root parser | `--project`, TTY/no-command dispatch, current-only command grammar |
| Project commands | create, source add, AI route set |
| Provider commands | add, list, show, test, remove |
| Integration commands | integrate, status |
| Taxonomy review | show, export, approve optional edited full taxonomy |
| Cluster review | list, show, export, approve exact rationales, regenerate feedback |
| Artifact commands | compile directory, verify directory, explain one output |
| Local utility | doctor and receipt-aware update |
| Output/error adapter | human/JSON output, terminal sanitization, exit classes |

## TUI screens

| Screen | State and user interaction |
|---|---|
| Workspace | Discover/select/create the current project |
| AI Connection | Configure/test provider kind, profile, endpoint, model, and credential reference |
| Vaults | Select 1–10 discovered/manual sources and edit source IDs |
| Preflight | Display document/block/input/request estimates, findings, routes, and consent boundary |
| Taxonomy | Review/edit title/path and approve the complete taxonomy hash |
| Clusters | Three-pane proposal/evidence/finding review, rationales, approval, regeneration feedback |
| Build | Select absent output and compile the approved plan |
| Verify | Independently verify the compiled directory |
| Provenance | Inspect current explanation/provenance status |
| Settings | Language, ASCII mode, high contrast, and project/provider context |

## State model

- `Model` owns the selected screen, language, accessibility flags, viewport,
  project/source/provider form state, current proposals/issues/rationales,
  progress, consent, and output.
- `AppEvent` is keyboard, resize, worker progress, or worker completion.
- `reduce(Model, AppEvent)` returns a new model and explicit `Effect` values.
- Effects perform project/source/provider/preflight/integration/review/compile/
  verify I/O through shared application services.

## Interaction rules

1. The minimum supported terminal viewport is 80 by 24.
2. Hostile C0/C1, ANSI/OSC, DEL, and bidi controls are rendered visibly/safely.
3. No untrusted source/provider text may set terminal titles, links, clipboard,
   or execute commands.
4. A secret is displayed with a fixed mask and redacted in debug output.
5. Remote disclosure requires an explicit confirmation state; it is not retained.
6. Busy operations show bounded progress; cancellation requests receive an
   explicit pre-barrier/barrier/idle outcome.
7. Required omission/minor-finding rationales are keyed to exact items.
8. Terminal state is restored on normal exit, panic, signal, cancellation, and provider failure.

## Accessibility and localization

- English and Korean labels are built into the screen model.
- High-contrast and ASCII rendering modes are explicit settings.
- Keyboard control is the current interaction mechanism; mouse input is out of scope.
- Long content is bounded/wrapped to the viewport and sanitized before display.

## Testability

- Reducer unit tests assert navigation and cancellation confirmation.
- Rendering tests use hostile controls and bounded viewport behavior.
- `tests/tui_pty_smoke.py` exercises the real binary against a synthetic
  loopback provider and asserts terminal restoration/source immutability.
- CLI help/parse tests are the executable contract for both present and absent command shapes.
