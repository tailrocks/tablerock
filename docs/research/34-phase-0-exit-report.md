# Phase 0 Exit Report

## Decision

Phase 0 is approved as of 2026-07-16. The operator's end-to-end implementation
authorization approves every fixed decision in
[`31-fixed-decisions.md`](31-fixed-decisions.md) and authorizes Roadmap Phases
0-15. Application and dependency checkpoints may begin.

The TableRock name and `tablerock-*` namespaces are the sole implementation
identity. The separate trademark, market, domain, package-registry, and
distribution clearance in [`05-product-identity.md`](05-product-identity.md)
remains a public-release gate; it does not introduce a second implementation
name or block pre-release engineering.

## Approved baseline

- TableRock baseline: `14149c40117f86e95ef8c8691016b3657b292a02`.
- Phase 0 decision checkpoint:
  `b56fbaed030e19d9f8fe9c7734af6c05d056f6a8`.
- Product boundary: PostgreSQL, ClickHouse, and Redis only.
- Architecture: Rust-owned engine and contracts; TEA TUI over TermRock,
  Ratatui, and Crossterm; synchronous coarse UniFFI for native macOS.
- Persistence: local-only `turso` through one serialized async actor.
- Distribution: direct Developer ID, hardened runtime, notarization, stapling.
- Delivery: forward-only checkpoints on `main`, with no branches or pull
  requests.
- Clean room: concepts-only external-product evidence; implementation from
  project requirements, primary sources, selected dependency documentation,
  and independent tests.

## Roadmap-to-ledger traceability audit

| Roadmap scope | Functional ledger coverage | Result |
|---|---|---|
| Phases 1-3: shell, profiles, connections | Ledger Connection management plus Responsive layout | Traced |
| Phases 4-5: browse, query, editor, grid, history | Ledger Workbench, SQL/editor, and Grid/value rows | Traced |
| Phases 6-8: PostgreSQL, ClickHouse, Redis | Ledger Grid/changes, Schema/admin, and Engine-specific parity | Traced |
| Phases 9-10: movement and parity expansion | Ledger Workbench, Schema/admin/data movement, and Later rows | Traced |
| Phase 11: TUI release gate | Every Core, Parity, Later, and Excluded ledger row | Traced |
| Phases 12-14: native proof and parity | Ledger Native macOS parity table plus every shared workflow row | Traced |
| Phase 15: closure | Ledger Closure rule: implemented, excluded, or visible blocker | Traced |

## Decision-to-evidence traceability audit

| Fixed decision | Primary evidence or planned adoption spike | Result |
|---|---|---|
| Product/scope/clean room | `00`, `01`, `03`, `05`, `06`; public references listed there | Traced |
| TEA, TermRock, Ratatui, Crossterm | Ratatui/Crossterm/TermRock sources in `07`, `13-platform`, `13-termrock`; Phase 1 T0/T1 | Traced |
| Rust contracts and bounded pages | Rust/UniFFI sources in `10`, `13-platform`, `14`; Phase 2 contract harness | Traced |
| PostgreSQL/tokio-postgres/rustls | Official sources in `13-platform` and `20`; Phase 2 PostgreSQL spike | Traced |
| ClickHouse official client | Official sources in `03`, `13-platform`, `20`; Phase 2 ClickHouse spike | Traced |
| Redis redis-rs | Official sources in `03`, `13-platform`, `20`; Phase 2 Redis spike | Traced |
| Local-only Turso | Official sources and compatibility gate in `13-platform`, `20`, `31`; Phase 2 storage proof | Traced |
| Secrets and 1Password | Official sources in `10`, `13-platform`, `14`; Phase 3 resolution tests | Traced |
| UniFFI/SwiftUI/AppKit/direct distribution | Apple/UniFFI sources in `12`, `13-platform`, `14`; Phase 12 proof | Traced |
| russh/SQL parser/tracing/OTLP | Official sources in `20`, `31`; Phase 10/5/11 adoption gates | Traced |
| Verification and trunk delivery | `30`, `32`, `33` | Traced |

The functional parity ledger maps every in-scope workflow to acceptance
evidence. Phase 1 begins with TermRock checkpoint T0 and an exact compatibility
pin. Later capability claims remain blocked until their phase-specific tests
pass; Phase 0 approval itself makes no implementation or server-support claim.

## Safety, provenance, and unsupported state

No application dependency, executable behavior, external product artifact, or
resolved secret is introduced by this checkpoint. Cancellation, redaction,
bounded results, ambiguous writes, and unsupported capabilities retain the
fixed semantics in `03`, `10`, `14`, `31`, and `32`.

External concept: none; decision-freeze documentation only  
Implementation source: TableRock research and operator approval  
Copied code/assets/text: none

## Verification

- All required research documents are present and linked from the research map.
- Documentation-only checks: `git diff --check`, required-file inventory,
  internal phase/status search, secret-pattern scan, commit-trailer parsing,
  and two-axis standards/spec review.
- No code fixtures, benchmarks, real-server tests, or binary artifacts apply to
  this documentation-only decision checkpoint.
- The repository is on `main` and matched `origin/main` at the approved
  baseline.
- The Phase 0 decision checkpoint was pushed to `origin/main`; local `HEAD` and
  `origin/main` both resolved to
  `b56fbaed030e19d9f8fe9c7734af6c05d056f6a8` after publication.
- Cross-document fixed-path searches found no competing database, persistence,
  terminal backend, TUI architecture, native bridge, or distribution path.
- At this Phase 0 checkpoint, Phase 1 remained visibly unimplemented and no
  working-client claim was made. Later evidence `35`-`45` records its delivery.
