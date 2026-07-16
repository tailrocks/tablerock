# TermRock 0.8 Canonical API Migration

## Checkpoint

TableRock advances its exact TermRock pin from historical Phase 1 revision
`9099b3db0c3318fd183d076c4e8f8002a877be6a` (`0.6.0`) to published `main`
revision `da54a033f368ed0888af90ae43d19bcb96fb8581` (`0.8.0`). The new revision
contains the canonical widget refactor, forward-only migration policy,
sequential `0001`/`0002` migration guides, language-neutral Unicode fixtures,
and bound-free defaults for generic widget state.

TableRock follows both sequential migrations rather than retaining a facade:

- canonical `layout`, `interaction`, `scroll`, `widgets`, `runtime`, and `osc`
  namespaces replace the removed component facade;
- reusable widget state owns its interaction and painted regions;
- shared theme and semantic emphasis replace consumer-local reusable styling;
- incompatible library changes add migration files instead of compatibility
  aliases or duplicate paths.

The Phase 1 shell already used canonical namespaces. Its only required source
migration is `StatusBar`: slots now declare optional hover style, the bar owns
base style and alpha, and rendering is stateful through `StatusBarState`.
TableRock supplies these canonical fields directly and deletes no-op legacy
construction. `StatusBarState` supplies the painted slot regions; TableRock
maps those regions to its product-level footer focus target and no longer adds a
parallel whole-row hit region. Slots remain informational rather than separate
product actions. No local status widget or alternate interaction implementation
is introduced.

## Bounds, safety, and ownership

- TermRock remains presentation-only. No database type, I/O, secret, process
  policy, executor, or TableRock domain state moves into it.
- TableRock retains one root TEA model/update/effect/subscription/view flow and
  one TermRock/Crossterm session owner.
- The exact Git revision and `Cargo.lock` are committed; branch dependencies are
  forbidden. TermRock work is direct to `main` only.
- The dependency graph drops TermRock's former `similar` dependency. No new
  TableRock dependency is introduced.
- Historical Phase 1 evidence continues to identify the revision actually
  tested at that exit. This checkpoint is the forward compatibility evidence.
- Jackin remains read-only evidence and still pins TermRock `0.6.0`. A read-only
  source scan at Jackin `27c450e9` finds its product code still imports the
  removed `termrock::components` facade, so Jackin is explicitly not compatible
  with `0.8.0` yet. Its consumers must apply TermRock migrations `0001` and
  `0002`; TableRock imports none of that product code.

## Verification record

- Isolated migration-only `cargo test --workspace --locked`: 45 passed, 3
  ignored PTY child fixtures
  executed by their parent harnesses.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings` and `cargo doc --workspace --no-deps --locked`: pass.
- `cargo deny check`: advisories, bans, licenses, and sources pass; the known
  Ratatui graph retains two allowed transitive `hashbrown` versions.
- `gitleaks detect --source . --no-banner --redact`, English-script scan,
  `git diff --check`, exact-lock inspection, and complete migration diff review:
  pass.
- TermRock `da54a03` retains the API verified at `ac55f7d`: 174 tests, clippy,
  rustdoc, docs
  type/build/catalog checks, lookbook-current checks, and deterministic preview
  rendering before TableRock adopted it.

External implementation source: none
Public source: <https://github.com/tailrocks/termrock/tree/da54a033f368ed0888af90ae43d19bcb96fb8581>
Migration sources: TermRock `MIGRATING.md`, `migrations/0001-*`, and
`migrations/0002-*` at the pinned revision
Copied code/assets/text: none
