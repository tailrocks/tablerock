# Evidence 647: shared streaming export service

## Claim

The bounded atomic streaming exporter is now owned by `tablerock-files`, not
the CLI adapter. Both clients can use one CSV/TSV/JSON encoder contract. Page
writes stay incremental, cancellation removes the same-directory temporary
file, failure cannot publish a partial destination, and successful completion
fsyncs then atomically renames through `AtomicFileWriter`.

`tablerock-cli` re-exports the shared API to preserve its public test seam and
uses the shared type directly in its streaming re-query effect. This checkpoint
does not claim native full-result export parity; operation/progress/UI wiring
remains the next TR-SCR-054 checkpoint.

## Verification

```text
mise exec -- cargo test -p tablerock-files -p tablerock-cli --lib --locked
mise exec -- cargo clippy -p tablerock-files -p tablerock-cli --all-targets --locked -- -D warnings
git diff --check
```

Results: 22 shared file tests and 9 CLI library tests passed; 3 controlled PTY
children remained intentionally ignored. Clippy and diff checks passed.

## Clean-room provenance

TablePro public workflow material established only the broad expectation that
database exports expose progress, cancellation, and a terminal outcome. No
TablePro source, tests, identifiers, product text, assets, colors, geometry,
layout measurements, or key bindings were read or copied. The implementation
comes from TableRock's existing atomic file contract and direct tests.
