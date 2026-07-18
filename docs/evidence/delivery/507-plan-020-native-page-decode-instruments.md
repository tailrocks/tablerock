# Plan 020 — native page-decode Instruments evidence

Date: 2026-07-19

## Measurement

`scripts/verify-native-page-performance.sh` fetches a real bounded page through
the synchronous UniFFI facade, then repeatedly decodes the exact bytes with the
Swift `PageV1` decoder used by the app. It launches that decode loop under Xcode
Time Profiler and separately records deterministic metrics.

| Fact | Result |
|---|---:|
| database | PostgreSQL 18.4 |
| page shape | 500 rows × 2 columns |
| encoded page | 46,244 bytes |
| decodes | 2,000 |
| total decode time | 4.370202 s |
| mean page decode | 2,185.101 µs |
| Time Profiler trace | 5,832,704 bytes; TOC export passed |
| host | MacBookPro18,2, Apple M1 Max, 64 GB |
| OS/tool | macOS 26.5.2; `xctrace` 16.0 (17F113) |

The measured page is below the fixed 2 MiB per-page resident guardrail and the
decode remains off `MainActor` in production. The raw trace, TOC, logs, and
metrics remain in generated local directory
`target/native-page-performance.IT1yek/` and reproduce with:

```sh
./scripts/verify-native-page-performance.sh
```

## Retained-object residual

The Leaks template was also attempted directly against the unrestricted local
`BehaviorProof` executable. Unlike Time Profiler, it did not begin its timed
capture or honor the time limit; the target remained suspended during attach
until the owned profiler/target processes were terminated. The earlier app
`leaks` scan reports only the system AppIntents/LNDaemon XPC graph, but that is
not strong enough to close retained-object attribution. This residual remains
open; SIP was not disabled.

## Provenance

TablePro was used only to confirm the broad concept of paging large database
results into a native grid. No source, tests, text, screenshots, layouts,
measurements, colors, assets, or key bindings were copied or translated.
