# 548 — cargo-deny Linux runner boundary

Date: 2026-07-21

## Failure

The dependency workflow invoked `EmbarkStudios/cargo-deny-action` from its
macOS dependency-freshness job. GitHub rejected the step before the audit:
container actions run only on Linux. Runs `29774248464` and `29774596945`
therefore reported failure without evaluating `deny.toml`.

## Repair

Dependency responsibilities now have explicit platform jobs:

- macOS continues latest-release and action-pin freshness checks;
- Ubuntu 24.04 owns the pinned cargo-deny container action.

Both jobs independently check out the exact source revision and install the
pinned stable Rust toolchain. No audit is skipped or weakened.

The first real Linux audit then exposed a pre-existing policy mismatch: UniFFI
0.32 was adopted and documented as MPL-2.0, but `deny.toml` omitted MPL-2.0 from
the explicit allowlist. The allowlist now matches the already-approved
dependency/license decision; no dependency or license changed.

## Verification

The workflow YAML preserves the exact current action pins. The pushed GitHub
Actions run is authoritative because local execution cannot reproduce the
hosted container-action dispatch boundary.

No external product influenced this CI repair.
