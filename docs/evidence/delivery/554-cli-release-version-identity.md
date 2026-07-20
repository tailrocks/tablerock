# 554 — CLI release version identity

Date: 2026-07-21

## Decision

The shipped binary is now `tablerock`. A compile-time
`TABLEROCK_VERSION_OVERRIDE` supplies rolling-preview identity while the normal
build falls back to the workspace package version. `--version` and `-V` print
that identity and exit before terminal initialization. Unknown arguments keep
the existing TUI behavior; this does not introduce a parser or wider CLI
surface.

## Verification

```text
cargo test -p tablerock-cli
# 21 unit + 18 integration/process/PTY tests passed; 7 real-server tests ignored by their declared CI cadence

cargo run -p tablerock-cli -- --version
# tablerock 0.1.0

TABLEROCK_VERSION_OVERRIDE=0.1.0-preview.1+abc1234 cargo build -p tablerock-cli
target/debug/tablerock --version
# tablerock 0.1.0-preview.1+abc1234

cargo fmt --all --check
cargo clippy --workspace --all-targets
# both exited 0; existing workspace warnings remain non-denied by repository policy
```

The process test exercises both version spellings with stdin disconnected and
asserts successful, stderr-free output. PTY/process callers use the renamed
Cargo binary identity.

## Provenance

The compile-time override shape was informed by the current public
`tailrocks/holla` preview workflow and independently implemented for
TableRock's release contract. No TablePro workflow or visual expression
influenced this non-UI checkpoint.
