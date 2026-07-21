# GitHub Actions runner policy

Every executable workflow uses one canonical YAML shape on all Linux lanes:

- `github` is the automatic and manual default and runs on `ubuntu-26.04`;
  never use `ubuntu-latest` or an unpinned Ubuntu label.
- `velnor` is an opt-in lane on `self-hosted,velnor-target-mvp`; select it
  only while that runner is registered and online.
- `both` executes identical jobs and steps on both lanes.

Use the `lanes` choice input and canonical inline `matrix.config` expression.
Only `matrix.config.writer` may gate mutating steps; it must guarantee exactly
one writer. Never branch step semantics by lane.

Rust compile jobs use mold and local-only sccache v0.16.0 with a 20 GiB bound.
The native adapter owns cache reporting. Do not combine target-directory
caches with sccache, compile CI tooling, or enable a remote cache backend.

Every job has a measured `timeout-minutes`; every workflow has concurrency and
an intentional cancellation policy. Checkouts are shallow and disable
credential persistence unless a documented writer step requires otherwise.

The GitHub lane is retained permanently so releases remain possible when the
Velnor fleet is unavailable. Changes to runner labels, lane matrices, actions,
or cache behavior must pass `velnor`, `github`, and `both` verification.

## TableRock-specific policy

Native Apple application builds remain on pinned macOS because they require
Xcode, AppKit, SwiftUI, and Apple packaging tools. Linux-capable work uses the
canonical lane matrix above.
