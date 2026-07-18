# Plan 020 — page-decode retained-object proof

Date: 2026-07-19

## Structural isolation

The app-level leak scan in evidence 505 contained 14,400 bytes rooted entirely
in the system AppIntents/LNDaemon XPC graph. That could not prove whether the
Swift page decoder retained objects.

`BehaviorProof` now supports a bounded post-benchmark hold. The native page
performance verifier runs 2,000 decodes over one real 500-row × 2-column,
46,244-byte UniFFI page, waits until the metric is durably emitted, and invokes
the system `leaks` tool against that exact still-live process. It then terminates
only its owned proof process and container.

## Result

```text
PERF_PAGE_DECODE bytes=46244 rows=500 columns=2 iterations=2000 total_seconds=4.766278 mean_microseconds=2383.139
PERF_LEAK_SCAN Process 71433: 0 leaks for 0 total leaked bytes.
PERF_TRACE_BYTES 5869568
```

Host: MacBookPro18,2, Apple M1 Max, 64 GB; macOS 26.5.2; `xctrace` 16.0.
Raw trace, TOC, benchmark output, and full leak report remain in generated
directory `target/native-page-performance.ogIU4C/`. Reproduce with:

```sh
./scripts/verify-native-page-performance.sh
```

Together with evidence 505 and 507, this closes measured scroll/RSS, real-page
decode latency, and decoder retained-object attribution. The system framework
XPC cycles remain recorded separately and are not attributed to TableRock.

## Provenance

TablePro was used only for the broad concept of bounded database result pages.
No source, tests, text, screenshots, layouts, measurements, colors, assets, or
key bindings were copied or translated.
