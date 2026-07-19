# Plan 021 — native multi-window ownership

Date: 2026-07-19

## Ownership

The application owns one synchronous UniFFI `BridgeClient`. Every SwiftUI
`WindowGroup` instance owns a distinct `BridgeModel`, query/object tab graph,
active-session projection, errors, and cancellation state. A window exposes
session actions only for the session it opened; a profile connected elsewhere
is rendered as such instead of granting cross-window controls.

Each scene is keyed by a persistent UUID. AppKit configures a shared tabbing
identifier with preferred native tabbing. SwiftUI automatic restoration keeps
the UUID stable across scene restoration.

## Intent restoration

Migration 0017 keys intent by window UUID, with the associated saved profile.
Thus two windows using one profile cannot overwrite each other's selected tab,
database, titles, or SQL text. Relaunch restores intent-only editor state and
identifies the saved profile, but never reconnects silently or restores results,
operation handles, credentials, or pending writes.

## Evidence

| Gate | Result |
|---|---|
| persistence migration and forward-migration suites | pass; 38 tests |
| UniFFI same-profile/two-window isolation and delete isolation | pass; 20 tests, 5 ignored |
| generated Swift bridge + native release build | pass |
| two real SwiftUI windows with shared bridge/distinct models | pass |
| UUID restoration and preferred AppKit tabbing structural checks | pass |

## Remaining boundary

The gate proves process-local independent model ownership and durable intent
isolation. It does not claim automatic credential resolution, silent reconnect,
or restoration of volatile database state. Those are intentionally excluded.

## Provenance

TablePro was used only to confirm the broad concepts of independent database
windows, native window tabbing, and intent restoration. No source, tests, text,
screenshots, layouts, measurements, colors, assets, or key bindings were copied
or translated.
