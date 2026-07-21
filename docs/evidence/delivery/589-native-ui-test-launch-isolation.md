# Native UI-test launch isolation

Date: 2026-07-21

## Failure

Native checkpoint run 29845160476 passed Rust, Swift package, binding, and
universal-XCFramework gates. Its canonical Xcode plan then ran 14 UI tests:
three passed and eleven failed after repeated application launches. The first
new dirty-tab check reached the editor but selected the SwiftUI `Menu` through
an assumed Button role; later launches commonly exposed no expected workbench
element. The archive and metadata gates correctly skipped.

## Correction

- Every UI-test launch now supplies `-ApplePersistenceIgnoreState YES`, so a
  self-hosted macOS runner cannot restore window state from another test or
  prior run into an otherwise isolated fixture root.
- Query-tab action menus now carry stable identifiers, and the interaction test
  selects the menu by semantic accessibility identity rather than an
  implementation-dependent SwiftUI element role.

The production app retains automatic restoration. Only the UI-test process
opts out, preserving the separate requirement for an explicit real-relaunch
restoration proof.

## Verification

- `cargo test --workspace --lib --bins`: 497 passed, 3 ignored.
- `git diff --check`: passed.
- This workstation has Command Line Tools but no Xcode application, so the
  corrected canonical Xcode plan remains pending on the serialized hosted
  macOS runner.

## Provenance

No external product reference influenced this test-isolation correction.
Evidence comes from TableRock's hosted Xcode log, accessibility contract, and
macOS test-process behavior.
