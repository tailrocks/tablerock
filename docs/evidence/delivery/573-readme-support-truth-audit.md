# 573 — README support-truth audit

Date: 2026-07-21

## Decision

The root README no longer describes the native app as future work or the
product as only a Phase 2 foundation. It now states the exact developer-preview
distribution shape, links the tested support matrix and visible parity gaps,
and names the external Developer ID/notarization boundary.

Support documentation now distinguishes the stateless TUI command from the
running native bridge collector/export. It also keeps engine-specific codes,
long-lived TUI collection, and crash-report sanitization visible as gaps.

## Verification

- Preview release `preview` has ten archive/checksum assets at source
  `349a6f4c42f6f6fad9ba9707e64d57c71c37b69d`.
- `homebrew-tablerock` formula and cask carry that exact source marker, version,
  and matching archive digests.
- Evidence 561 proves the pull-verified update workflow and install/audit test.
- Evidence 566–568 proves the safe schema, runtime outcomes, and typed adapter
  diagnostic path. Commits `e0d6326` and `1b84483` implement native atomic
  export and balanced save-panel access; hosted support-export XCUITest is
  still running and is not claimed here.

## Remaining boundary

The rolling preview is behind current `main` while current CI is queued. It may
advance only after the source commit's consolidated CI succeeds. Production
signing/notarization and complete parity remain unclaimed.

## Provenance

Implementation source: TableRock repository, release, tap, and evidence state.

TablePro influence: none; this is support-claim reconciliation.

Copied source, tests, identifiers, assets, strings, colors, geometry, layout
measurements, or key bindings: none.
