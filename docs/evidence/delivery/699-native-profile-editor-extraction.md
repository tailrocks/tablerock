# Native profile-editor extraction

Date: 2026-08-13

Checkpoint: P4b — extract connection profile editing

## Decision

The production connection-profile editor now lives in
`ProfileEditorSheet.swift`. General connection fields, credential-source
selection, TLS, SSH tunnel configuration, ordered startup commands, validation,
and save lifecycle form one presentation-owned surface.

The sheet edits a draft and delegates persistence to its async save closure. It
does not acquire backend or persistence authority.

## Bounds and failure truth

- Field structure, validation, security acknowledgements, startup-command
  ordering, accessibility identifiers, sheet sizing, and save/dismiss behavior
  moved unchanged.
- Secret values remain draft-only presentation inputs. Existing backend and
  redaction boundaries are unchanged.
- No fixture symbol, generated FFI type, backend import, AppKit adapter, or
  Design Lab dependency entered the editor file.

## Evidence

- `scripts/build-native-app.sh` passed with strict concurrency and warnings as
  errors.
- `TableRockAppTests`: 24 passed, zero failures.
- Profile-editor structural/runtime gate passed.
- `scripts/verify-native-source-ownership.sh` passed.
- XcodeGen regenerated the app target and `git diff --check` passed.
- `TableRockApp.swift` decreased from 6,997 to 6,754 lines.

## Remaining work

Continue extracting connection sheets, workbench surfaces, and narrow AppKit
adapters. Then enforce the Presentation module and remove Release
fixture/scripted-backend membership.

## Clean-room provenance

This checkpoint changes ownership only. TablePro public user-visible
organization informed the confirmed workbench direction; no TablePro source,
tests, comments, bundle internals, branding, assets, copy, or proprietary
fixtures were used.
