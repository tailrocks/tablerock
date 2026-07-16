# Latest Dependency Policy

## Decision

TableRock starts every library, toolchain, CI action, and development tool at
the latest stable upstream release. Exact versions and commit SHAs provide
reproducibility only. They are refreshed forward immediately when a newer stable
release is discovered; backward compatibility is not a reason to retain an old
dependency or API.

Before adoption and at every implementation checkpoint, verify the current
release, enabled features, MSRV, maintenance status, advisories, licenses, and
duplicate graph. Daily CI rejects outdated workspace dependencies and audits the
complete graph. Trunk-only policy means agents apply every detected update
directly to `main`, never through an automated pull request.

## 2026-07-17 refresh

- `bytes` advanced from 1.11.1 to current 1.12.1.
- Tokio advanced from 1.52.3 to current 1.52.4.
- TermRock advanced from `4c3adac` to current `7f24de4`; the intervening changes
  affect its documentation site, not its Rust API, and TableRock still verifies
  the complete `crossterm` and `serde` feature surface.
- TermRock then advanced to `78d9671`, raising its MSRV to Rust 1.97 and updating
  its ANSI parser, Unicode segmentation, CI actions, and development toolchain.
  TableRock moved to Rust 1.97 and repinned immediately. Public Rust API changes
  in that range are documentation improvements and formatting only, so no
  migration entry applies.
- TermRock advanced to `7c01dff` to provision current nightly Rust for its API
  tooling. The change is CI-only with no consumer migration; TableRock repinned
  immediately.
- TermRock advanced to v0.11.0 at `50d67f1`. TableRock adopted migrations 0016
  (semantic strong text and viewport emphasis) and 0017 (canonical scrollable
  block helpers) immediately. No existing TableRock call site used a replaced
  surface, so no compatibility layer remains.
- TermRock advanced to `a4f513c`, completing migration 0017 with neutral dialog
  shell, dialog-body, scroll-input, scrollbar, and line-width helpers. TableRock
  repinned immediately; no current call site requires adaptation.
- Every other adopted and planned baseline version in dependency decision `20`
  matches the current crates.io release.
- Current official ClickHouse 0.15.1 and upstream `main` still require
  `polonius-the-crab` 0.5.0, which transitively uses unmaintained `paste` 1.0.15.
  The narrow `RUSTSEC-2024-0436` audit exception records that proven upstream
  constraint. It is not a security-vulnerability waiver and must disappear as
  soon as the latest official client removes the dependency.

Context7 was attempted first and reported its monthly quota exhausted. Versions
were therefore verified from Cargo/crates.io; TermRock and ClickHouse state was
verified from their upstream Git repositories; GitHub Action releases and SHAs
were verified through the GitHub API.

## Evidence

The locked workspace builds and tests against the refreshed graph. The daily
workflow installs the current `cargo-outdated`, rejects stale direct
dependencies and CI action revisions, and runs `cargo-deny`. Action revisions
are immutable SHAs annotated with their current release tags.

External concepts: dependency freshness automation, immutable CI action pins
Public sources: <https://doc.rust-lang.org/cargo/commands/cargo-update.html>, <https://docs.github.com/code-security/dependabot>, <https://github.com/EmbarkStudios/cargo-deny>
Implementation source: TableRock-owned policy, workflow, and dependency pins
Copied code/assets/text: none
