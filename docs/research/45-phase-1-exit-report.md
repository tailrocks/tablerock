# Phase 1 Exit Report

## Decision

Phase 1 passes. This report audits the current code and evidence against every
Phase 1 checkpoint and the applicable quality matrix; it does not infer broad
completion from aggregate test success.

## Evidence identity and platform

- TableRock implementation/evidence commit:
  `c7f96047588f0257b9436ffaea21a8591bc5aa51` (contains architecture guard
  `d2cccec30767672c5de93a6a4cae5582e1a0e2c4` and causal PTY evidence
  `5d90f93ec8543e896c07dfbc965b4150db36b00b`).
- TermRock dependency/main commit:
  `9099b3db0c3318fd183d076c4e8f8002a877be6a`, crate `0.6.0`.
- Toolchain: Rust/Cargo `1.97.0`; workspace minimum Rust `1.95`; Git `2.55.0`;
  RTK `0.43.0`.
- Platform: Apple arm64, macOS `26.5.2` build `25F84`; host Ghostty
  `1.3.2-main-+c5a21edfc`; PTY TERM contract `xterm-256color` through
  `portable-pty 0.9.0`.
- Runtime tuple: Tokio `1.52.3`, Crossterm `0.29.0`, Ratatui core `0.1.2`,
  Ratatui Crossterm `0.1.2`, Ratatui widgets `0.3.2`.
- Database/server versions: not applicable. Phase 1 opens no database client;
  the PostgreSQL, ClickHouse, and Redis version matrix begins at driver phases.

## Executed verification and artifacts

| Command or fixture | Result | Artifact |
|---|---|---|
| `cargo fmt --all -- --check` | Pass | Workspace sources |
| `cargo test --workspace --locked -- --test-threads=1` | 34 pass; 3 ignored child fixtures are selected and executed inside parent PTY tests | [`pty_lifecycle.rs`](../../crates/tablerock-cli/tests/pty_lifecycle.rs), [`run.rs`](../../crates/tablerock-cli/src/run.rs), [`ingress.rs`](../../crates/tablerock-cli/tests/ingress.rs), [`shell.rs`](../../crates/tablerock-tui/tests/shell.rs) |
| `cargo test -p tablerock-tui --test architecture --locked` | 2 pass; reducer/view capabilities and manifest dependencies structurally constrained | [`architecture.rs`](../../crates/tablerock-tui/tests/architecture.rs) |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Pass | Workspace targets |
| `cargo doc --workspace --no-deps --locked` | Pass | Generated local rustdoc; not distributed |
| `cargo deny check` | Advisories/bans/licenses/sources pass; two allowed transitive `hashbrown` versions reported | `deny.toml`, `Cargo.lock` |
| `gitleaks detect --source . --no-banner --redact` | Pass, no leaks | `.gitleaksignore` plus repository history |
| `rg -n '[\p{Han}\p{Hiragana}\p{Katakana}\p{Hangul}]' README.md ROADMAP.md crates docs --glob '*.rs' --glob '*.md'` | Zero matches | Repository Rust/Markdown sources |
| `git diff --check` | Pass | Complete exit-report diff |
| `git status --short --branch`, `git diff --stat`, and full `git diff` review | Only intended Phase 1 exit docs/status files; no unrelated edits, copied expression, competing architecture, or forbidden data | Current exit checkpoint |

No benchmark is applicable to the empty shell. The first service/result and UI
latency benchmarks are mandatory in their owning phases; this report makes no
performance claim beyond bounded-memory ingress proofs.

## TableRock checkpoint audit

| Requirement | Authoritative evidence | Result |
|---|---|---|
| Exact TermRock/Ratatui/Rust tuple | `Cargo.toml` and `Cargo.lock` pin TermRock `371ff94e`, Ratatui core `0.1.2`, Ratatui Crossterm `0.1.2`, Rust `1.97`; `130` records current compatibility | Pass |
| One Crossterm input/lifecycle path | CLI creates one `EventStream`; TermRock `Session` is the only raw/full-screen lifecycle owner; no second terminal stack exists | Pass |
| Normal/error/signal/panic safety | Real PTYs cover semantic exit, SIGTERM, returned error, panic, and private input; `41`-`43` record lifecycle and raw-termios proof | Pass |
| One root TEA flow | `tablerock-tui` has one Model/Message/Update/Effect/Subscription/View boundary; reducer/view tests and `39` prove deterministic projection | Pass |
| Bounded engine subscription seam | `ENGINE_EVENT_CAPACITY` is 256; type-distinct ingress linearizes events/progress/overflow/closure, coalesces only progress, and exposes resync; `44` records evidence | Pass |
| Bounded full-frame rendering | Executable renders the complete view only after reducer render requests and refreshes frame-scoped hit geometry atomically | Pass |
| Responsive shell and minimum state | `TestBackend` covers wide/medium/narrow/too-small layouts; PTY resize covers tiny then normal projection | Pass |
| Focus, discovery, keyboard/mouse parity | Reducer/input/render tests cover deterministic focus/action order, press/repeat/release policy, rendered hit targets, click/drag/wheel, non-color labels, and visible actions | Pass |
| Test seams | Reducer, `TestBackend`, TermRock direct-buffer, process contract, and real-PTY suites exist at their public boundaries | Pass |

## TermRock checkpoint audit

| Requirement | Authoritative evidence | Result |
|---|---|---|
| T0 consumer pin | `35` records minimal public runtime/session/widget consumption and dependency policy | Pass |
| Neutral `Tree` | Published on TermRock `main`; caller-owned state, Unicode/minimum geometry, keyboard/mouse/wheel, docs/lookbook/tests recorded in `36` | Pass |
| Neutral `Form` | Published on TermRock `main`; validation/focus/scroll interaction, docs/lookbook/tests recorded in `37` | Pass |
| Neutral `SplitPane` | Published on TermRock `main`; bounded resize/hit geometry, docs/lookbook/tests recorded in `38` | Pass |
| Lifecycle hardening | Partial acquisition rollback, reverse cleanup, earliest-error retention, retry, idempotence, and compatibility recorded at exact descendant revision in `41` | Pass |
| Jackin compatibility | Read-only all-target compile evidence is recorded in `41`; no Jackin product internals enter TableRock | Pass |

At this historical Phase 1 exit, `origin/feat/canonical-widget-migration` was
not a published `main` revision and was not a TableRock dependency. It later
reached TermRock `main`; TableRock's forward re-audit and exact `0.8.0` pin are
recorded in [`49-termrock-0.8-migration.md`](49-termrock-0.8-migration.md).

## PTY/process matrix audit

| Required behavior | Evidence | Result |
|---|---|---|
| Explicit non-TTY contract | Process exits 1, no stdout, fixed safe stderr | Pass |
| Partial initialization rollback | TermRock fallible-writer matrix arms cleanup before every acquisition | Pass |
| Raw and writer-backed restoration | Pre/fault/post termios plus ordered alternate-screen, five mouse modes, paste, wrap, and cursor sequences | Pass |
| Idempotent sole ownership | TermRock exact-revision tests plus TableRock single-session architecture | Pass |
| Input fact matrix | Unit mapping plus PTY focus, Unicode-safe paste, tiny/normal resize, press/drag/release, wheel, and key policy | Pass |
| Starvation resistance | Signal-first/fair terminal arbitration; PTYs continuously publish progress and place terminal quit amid 128 mouse/resize facts | Pass |
| Diagnostic redaction | Paste debug exposes only size/truncation; private PTY paste and controlled fault text are absent from output; leak scan passes | Pass |

## Exit criteria

- The shell renders and restores on normal, returned-error, signal, and panic
  paths, including exact Unix termios restoration.
- Reducers and views remain synchronous and deterministic; repository
  architecture tests reject async/I/O/process/network/telemetry capabilities and
  restrict TUI dependencies to Ratatui core and TermRock.
- TableRock composes TermRock widgets and adds no duplicate generic widget
  layer.
- All known Phase 1 incomplete claims are either repaired here or remain
  explicitly historical, checkpoint-scoped statements.

## Supported and unsupported boundary

Supported now: interactive-TTY validation; one responsive empty shell;
deterministic focus/actions; keyboard, focus, paste, mouse, wheel, resize, and
signal mapping; frame-authorized hit testing; bounded post-mapping ingress;
explicit resync state; safe normal/error/signal/panic restoration.

Not yet supported: profiles, secrets, SSH, persistence, any PostgreSQL,
ClickHouse, or Redis connection, catalogs, queries, results, editing, history,
transfer, administration, telemetry, UniFFI/native UI, or distribution. These
are visible roadmap/ledger work, not Phase 1 defects and not exclusions unless
the fixed product boundary says so.

Cancellation and partial outcomes are intentionally narrow. Ctrl-C, SIGTERM,
and Quit cancel only the empty terminal session and restore local terminal
state; no server operation exists to cancel. An over-capacity state event is not
reported as applied: it collapses into visible `Resync required` state while
accepted events drain. Test-only reconciliation clears that state; the real
snapshot request, revision validation, and cancellation contract begin in
Phase 2.

Security: non-TTY and fault errors use fixed safe text; paste/fault contents are
absent from output; no credentials, SQL, cells, sockets, files, or telemetry
exist; leak scan passes. Provenance: `35`-`44` name public sources and record no
copied reference expression. Licensing: workspace and TermRock are Apache-2.0;
`cargo deny` passes the pinned graph.

Delivery governance: commits `3c7f95e`, `fe7a350`, `af98726`, `fba8eba`, and
`5d90f93` contain literal `\n` text around the intended Codex trailer, so Git
parses their DCO sign-off but not `Co-authored-by`. Published trunk history was
not rewritten. Forward repair `d2cccec` records those hashes and contains both
parseable `Co-authored-by: Codex <codex@openai.com>` and DCO trailers, verified
with `git interpret-trailers --parse`. All later commits use separate message
paragraphs for trailers.

The functional ledger was reviewed at this exit. No Core, Parity, or Later
product row is closed by an empty-shell phase; every unimplemented
connection/exploration/query/result/edit/history/safety/transfer/admin/native/
distribution row remains a visible blocker for its owning parity claim.
Explicit exclusions remain unchanged. The review record is appended to `06`.

Documentation updated: README status, roadmap Phase 1 status, research index,
historical checkpoint text, functional-ledger review log, and this report.

Phase 2 may now define authoritative owned IDs, revisions, commands, events,
pages, cancellation, redacted errors, result bounds, and driver feasibility.

External concepts: none beyond sources already recorded in `35`-`44`  
TableRock requirement: Roadmap and delivery-plan Phase 1 exit criteria  
Implementation source: current TableRock/TermRock public contracts and tests  
Copied code/assets/text: none
