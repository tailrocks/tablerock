# 545 — Core architecture guard structural repair

Date: 2026-07-21

## Trigger

Checks run 29773657777 failed on both macOS and Ubuntu. The core architecture
test rejected the substring `redis` in `lib.rs` after the Rust-owned Redis
command planner was exported.

## Root cause and repair

The guard confused domain vocabulary with runtime coupling and inspected a
hand-maintained subset of source files. This both rejected valid engine-typed
contracts and allowed any newly added module to bypass the forbidden-runtime
scan.

The test now reads every Rust source file under `tablerock-core/src` and
rejects actual runtime/presentation/I/O APIs (`tokio`, Ratatui, TermRock,
Crossterm, `std::time`, `std::net`). The manifest dependency allowlist/count
continues to enforce the pure dependency boundary. PostgreSQL, ClickHouse, and
Redis names remain valid core domain facts.

## Verification

```text
cargo test -p tablerock-core --test architecture
```

Result: pass. The push Checks run remains the cross-platform authority.
