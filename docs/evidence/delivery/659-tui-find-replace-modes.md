# Evidence 659: TUI find/replace modes via shared core engine

Date: 2026-08-09

## Checkpoint

Close the plan-018 residual “TUI word/regex/scope parity” for `TR-SCR-049`.

## Decision

- Add `tablerock_core::find_replace` as shared authority for modes:
  `Literal` (case-insensitive), `CaseSensitive`, `WholeWord`,
  `RegularExpression`, scopes `Document` / `Selection`, 10_000 match cap,
  finite zero-width regex advance.
- Adopt workspace pin `regex = 1.13.1` (MIT OR Apache-2.0) for core only.
- TUI paste confirm protocol: `find=>replace=>[all]=>[mode]=>[scope]` with
  legacy `i`/`ci`/`all` preserved; modes `word`/`regex`/`cs` and scopes
  `sel`/`doc`.
- `QueryEditorModel::replace_with_mode` drives the core engine (no local
  reimplementation of match rules).

## Bounds and failure truth

- Empty pattern / invalid regex / empty selection scope → `FindReplaceError`,
  surfaced as status text; editor text unchanged on error.
- Replace-all beyond 10_000 matches fails closed.
- Native AppKit engine remains in Swift for IME/selection integration; mode
  contract is matched by unit tests against the same cases.

## Evidence

```text
cargo test -p tablerock-core find_replace
cargo test -p tablerock-tui find_replace
cargo test -p tablerock-ffi --test workflow_equivalence
```

## Remaining work

- Hosted native XCUITest replay of the find/replace sheet remains evidence 641.
- Optional: UniFFI export of core find_replace so native drops the Swift engine
  (not required for TUI residual close).
