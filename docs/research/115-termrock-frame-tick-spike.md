# TermRock Immutable Frame-Tick Spike Adoption

## Published boundary inspected

TableRock refreshed its exact TermRock `main` pin from
`a25145bb82f584bc40138ba4d419df95846f1f7e` to
`51910bf4e9495578ad5a2d5bc278a4c195439a4f`. The published change is a lookbook
runtime spike; it does not yet alter the TermRock library API or add a migration.

The sibling TermRock worktree contains another agent's uncommitted lookbook
focus work. TableRock inspected only published history and did not modify,
stage, or overwrite those files.

## Direction adopted

The spike selects one immutable `FrameTick` sampled once per frame. Update and
render consume the same time value; widgets never read clocks. Elapsed time,
not render count, drives indeterminate progress. Manual ticks make TTL and
animation tests deterministic without sleeping. Deadline-aware polling remains
consumer/runner policy and must not reintroduce an executor or subscription
stack into TermRock.

No TableRock source adaptation is needed because the spike remains lookbook-
local. When TermRock publishes the graduated runner migration, TableRock will
adopt it immediately and pass the same immutable tick through its single root
TEA flow instead of creating a competing clock abstraction.

## Verification

The complete TableRock workspace builds, tests, lints, and documents against
the exact new revision. Dependency and source-policy gates remain part of this
checkpoint.

Primary source: TermRock's published `plans/031-frame-clock-design.md` at
revision `51910bf`.
