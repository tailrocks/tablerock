# Evidence 652: native startup-command editor

## Claim

TR-SCR-056 now has a native connection-editor projection over the shared Rust
startup-action contract. Operators can add, edit, remove, and reorder up to 16
commands; select Read only, Write, or Dangerous safety; set bounded timeout;
and choose reconnect participation. Non-read-only rows carry persistent text
and symbol review warnings.

`BridgeProfileDraft` transports typed startup records. Rust validates statement
size, count, safety class, timeout, reconnect policy, and order before durable
save. Profile reads round-trip the same bounded intent. Native profile open runs
the existing engine-specific startup executor after the database connection is
established: only Read only actions auto-run; Write and Dangerous actions remain
`SkippedNeedsReview` under the core contract.

Status remains `partial`: native presentation does not yet expose reviewed
execution of skipped actions or a connect/reconnect outcome list, and hosted
reconnect replay remains open.

## Verification

```text
mise exec -- cargo test -p tablerock-ffi --test conformance --locked
mise exec -- cargo test -p tablerock-ffi --test bridge_real --locked \
  bridge_postgres_open_probe_fetch_shutdown -- --ignored --nocapture
mise exec -- cargo clippy -p tablerock-ffi --all-targets --locked -- -D warnings
mise exec -- ./scripts/generate-swift-bindings.sh
mise exec -- ./scripts/build-native-app.sh --configuration Release
mise exec -- ./scripts/verify-native-profile-editor.sh
mise exec -- cargo test -p tablerock-core --test screen_manifest --locked
```

Conformance passed 21 tests including ordered startup-action persistence beside
write-only SSH fields. Live PostgreSQL bridge proof confirmed a Read only
startup action changed session state while a following Write action did not
execute. Clippy, generated-binding equality, native Swift 6 Release build,
editor runtime audit, and manifest validation passed.

## Clean-room provenance

TablePro was checked for this connection-screen family only as a broad workflow
reference. No accessible startup-command-specific public material was found.
No TablePro source, tests, identifiers, text, assets, colors, geometry,
measurements, or key bindings were read or copied. Requirements and expression
derive from TableRock product docs and existing Rust startup-action contracts.
