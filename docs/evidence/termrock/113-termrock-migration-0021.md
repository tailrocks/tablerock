# TermRock Migration 0021 Adoption

## Published boundary inspected

TableRock refreshed its exact TermRock `main` pin from
`f802fcc48c4361ea477c5021b52a121f180d4b4d` to
`a25145bb82f584bc40138ba4d419df95846f1f7e`. Migration 0021 completes the
responsive narrow-width contract for determinate and indeterminate progress.

The sibling TermRock worktree contains unrelated uncommitted lookbook changes
from another agent. TableRock inspected only published history and did not
modify, stage, or overwrite that work.

## Old to new

Determinate `Progress` now owns percentage elision below 16 columns while
retaining label and glyph-track completion cues. Consumers must render it at
the available width instead of maintaining a parallel narrow-layout fallback.
An indeterminate progress value with `.frames(&[])` is now a true buffer no-op.

TableRock does not yet render this widget, so no source adaptation or
compatibility layer is required. Future query, import/export, and mutation
progress surfaces must use this responsive TermRock contract directly.

## Verification

The complete TableRock workspace builds, tests, lints, and documents against
the exact new revision. Dependency and source-policy gates remain part of this
checkpoint.

Primary source: TermRock's published
`migrations/0021-v0.11.0-responsive-progress-percentage.md` at revision
`a25145b`.
