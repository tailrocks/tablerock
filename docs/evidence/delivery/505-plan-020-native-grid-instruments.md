# Plan 020 — native grid Instruments evidence

Date: 2026-07-19

## Scope

`scripts/verify-native-performance.sh` builds the Release app, installs a
bounded 10,000-row × 8-column in-memory page snapshot, drives the AppKit grid
down and back through resident rows, and records the app with Xcode Time
Profiler. It samples RSS before the deliberately perturbing leak inspection.

The harness exposed and fixed a structural AppKit initialization defect:
`NSTableView` had its data source installed before its columns. Adding the first
column then caused AppKit to consult all 10,000 rows before viewport geometry
existed, hanging the main actor and growing RSS. The grid now installs columns
first, then attaches its delegate/data source, preserving view reuse and
viewport-bounded work.

## Measurement

- Release build, Swift 6 strict concurrency, macOS 26 target
- MacBook Pro (MacBookPro18,2), Apple M1 Max, 64 GB
- macOS 26.5.2 (25F84)
- `xctrace` 16.0 (17F113), Time Profiler, 6-second attached recording
- fixture construction: 0.005236 seconds
- two-way resident scroll: 1.554359 seconds
- peak pre-leak-scan RSS: 109,632 KiB, below the current 512 MiB guardrail
- trace bundle: 16,936,960 bytes; TOC export succeeded
- `leaks` reported 288 objects / 14,400 bytes rooted in the system
  AppIntents/LNDaemon XPC connection graph. This is recorded, not relabeled as
  zero; retained-object attribution remains part of the wider plan-020 gate.

Raw `.trace`, logs, RSS samples, TOC export, and leak output remain in the local
generated `target/native-performance.9jF5FV/` directory. They are reproducible
with:

```sh
./scripts/verify-native-performance.sh
```

This closes measured native grid scrolling only. UniFFI page-decode latency and
retained-object attribution remain open, so the combined Instruments done
criterion stays unchecked.

## Provenance

TablePro was used only to confirm the broad concept of a native database
workbench with a large scrollable result grid. No source, tests, text,
screenshots, layout measurements, colors, assets, or key bindings were copied
or translated.
