# Plan 006 residual — group rename dialog UI

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `ActionId::RenameGroup` + Connections bar | done |
| Confirm dialog: paste new name (safe charset) | done |
| `Effect::RenameGroup` → persistence `rename_group` | done |
| Remove on group branch (`g:name`) → RemoveGroup | done |
| Unit: rename emit + remove group confirm | done |

## Decision

Persistence already supported `rename_group`. The residual was UI: select a
group tree node, RenGroup → confirm new name → actor updates all members.
Remove on a group branch now opens RemoveGroup (members become ungrouped)
instead of requiring a profile leaf.

## Evidence

```text
cargo test -p tablerock-tui --lib rename_group_dialog
cargo check -p tablerock-cli
```

## Remaining work

- Create-group dialog (if not already via editor group field)
