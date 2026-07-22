# Evidence 644: streaming CSV import foundation

Date: 2026-07-22

## Outcome

The shared file layer now scans CSV without whole-file buffering:

- a fixed 64 KiB read buffer and caller-bounded row batches keep resident data
  independent of file size;
- quoted commas, escaped quotes, embedded newlines, chunk boundaries, UTF-8,
  row width, unique headers, file/row/cell limits, and formula-like literal
  values retain the existing import contract;
- progress reports bytes, rows, and formula-like cell counts after each batch;
- callback cancellation stops before another batch is delivered;
- no SQL or engine operation exists in the scanner.

This is the structural prerequisite for `TR-SCR-053`; Rust-owned asynchronous
apply, engine-aware cancellation, native polling, and bounded error summaries
remain before that screen can be marked proven.

## Verification

```text
mise exec -- cargo test -p tablerock-files csv_import --locked
11 passed, including an 80,000-row multi-megabyte scan with a 257-row maximum
resident batch and cancellation after the first two-row batch

mise exec -- cargo clippy -p tablerock-files --all-targets --locked -- -D warnings
passed
```

## Clean-room provenance

TablePro's current public import/export documentation was checked only for the
broad expectations of explicit progress, cancellation, and terminal error
policy. No source, tests, identifiers, strings, assets, screenshots, layout
measurements, colors, or key bindings were copied. The scanner and its limits
derive from TableRock's plan 016, product requirements, Rust I/O behavior, and
direct tests.
