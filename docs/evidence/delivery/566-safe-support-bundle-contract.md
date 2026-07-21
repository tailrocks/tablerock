# 566 — Safe support-bundle contract

Date: 2026-07-21

## Decision

Phase 15 support diagnostics begin below presentation with a closed Rust-owned
schema. `SupportBundle` accepts only `SafeDiagnostic`, projects its enum/numeric
facts, retains at most 256 records, and reports saturation. Arbitrary diagnostic
messages cannot enter the type.

Platform identity is closed to operating-system and architecture enums. Client
versions accept at most 64 ASCII alphanumeric/version punctuation bytes;
anything else becomes `invalid`. Rendering is deterministic line-oriented text
so operators can inspect it before sharing.

`tablerock --support-bundle` exits before terminal initialization and prints an
empty current-process manifest. It does not inspect environment variables,
logs, profiles, history, SQL, cell values, filesystem paths, endpoints,
hostnames, or credentials.

## Verification

```text
cargo fmt --all --check
cargo test -p tablerock-core --test support_bundle
cargo test -p tablerock-cli --test process_contract
cargo clippy -p tablerock-core -p tablerock-cli --all-targets --locked -- -D warnings
```

Results: 3 support-contract tests and 4 CLI process-contract tests pass; clippy
reports no issues. The process test injects a credential-bearing
`DATABASE_URL` and proves none of its user, password, hostname, or database
components enter output.

## Remaining boundary

This establishes the fail-closed schema and operator-readable command. It does
not claim runtime diagnostic retention, native export UI, crash-report
sanitization, or complete tested support-matrix publication.

## Provenance

Implementation source: TableRock-owned safe-diagnostic contracts and tests.

TablePro influence: none; this is diagnostics/security infrastructure, not a
product workflow or visual-expression checkpoint.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
