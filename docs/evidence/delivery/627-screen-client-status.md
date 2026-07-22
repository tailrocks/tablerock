# Evidence 627: per-client screen status

Date: 2026-07-22

## Claim

Screen traceability now records TUI and native status independently for every
canonical surface. The verifier rejects:

- missing or extra client-status IDs;
- invalid `proven`, `partial`, `missing`, or `n/a` values;
- `n/a` on an applicable client;
- active status on an inapplicable client;
- missing or inapplicable status paired with implementation/test links;
- partial or proven status without implementation/test links;
- aggregate manifest status inconsistent with per-client status.

Existing screens remain partial. Native-only surfaces explicitly mark TUI
`n/a`. Ten newly inventoried surfaces retain TUI `partial` and native
`missing`; no broad native source/test path is used as substitute evidence.

## Root cause

One aggregate status could not distinguish a complete TUI seam from absent
native presentation. That made cross-client closure unverifiable. Joining a
separate exhaustive per-client status table removes this ambiguity without
duplicating the main surface inventory.

## Verification

```text
mise exec -- rtk cargo test -p tablerock-core --test screen_manifest
cargo test: 1 passed (1 suite, 0.00s)

mise exec -- rtk cargo clippy -p tablerock-core --test screen_manifest -- -D warnings
cargo clippy: No issues found

rtk git diff --check
exit 0
```

## Residual

`partial` still means direct screen/state replay is incomplete. No row becomes
`proven` until both its applicable client implementation and direct tests are
linked and its aggregate gap is `none`.
