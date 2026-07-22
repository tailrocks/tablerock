# Plan 021 screen-traceability gate

Date: 2026-07-22

## Gap evidence

`docs/prompt.md` requires a canonical machine-checkable screen manifest,
state/action/client/engine traceability, CI rejection of invalid completion
claims, two clean full replays, and a final TablePro-informed clean-room gap
review. Current repository search found no canonical manifest or verifier.

Plan 021 previously required a final ledger audit but did not own these exact
artifacts or replay gates. That allowed broad native progress without one
authoritative surface-by-surface closure mechanism.

## Plan correction

Checkpoint 17 now owns:

- stable requirement IDs for every interface surface and flow;
- full state, action, focus, client, and engine enumeration;
- links through product, plan, Rust, TUI, native, tests, and evidence;
- an always-on structural verifier;
- two consecutive full runtime/visual replays;
- final TablePro public-workflow review under the clean-room boundary.

No row or phase is marked complete by this planning checkpoint. Implementation
and both replay audits remain required.

## Verification

```text
rg -n "Canonical screen manifest|Two consecutive full manifest" \
  plans/021-native-parity-and-closure.md
# checkpoint and done gates present

git diff --check
# clean
```

## Provenance

No TablePro source, test, identifier, text, asset, screenshot, geometry, color,
or key binding was inspected or used. This checkpoint derives solely from
TableRock's authoritative goal prompt and current repository inventory.
