# Plan 006 residual — continuous HealthTick for BoundedAutomatic

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| CLI `tokio::time::interval(30s)` → `Message::HealthTick` | done |
| Reducer emits `CheckSessionHealth` only with live session + BoundedAutomatic | done |
| Skips while status contains "reconnecting" | done |
| Manual preference: no continuous probe | done |
| HealthOk/Failed update `workbench.context.health_label` | done |
| Unit test matrix Manual / auto / reconnecting | done |

## Decision

Continuous health is preference-gated so Manual profiles stay quiet.
Interval is 30s with `MissedTickBehavior::Skip` to avoid backlog after
blocking work. First tick may fire immediately after start (tokio default);
no-op without session.

## Evidence

```text
cargo test -p tablerock-tui --lib health_tick
cargo check -p tablerock-cli -p tablerock-tui
```

## Remaining work

- Configurable interval preference (optional)
