# Native app-test module import

Date: 2026-07-21

## Failure

Native checkpoint run 29843185342 passed the Swift package suite and universal
XCFramework build, then the canonical Xcode plan failed while compiling
`BridgeModelScenarioTests`. The test directly constructs
`WorkbenchProfileDraft` and `WorkbenchOpenParams`, which are declared by
`TableRockFeature`, but imported only the application module. Imports are not
transitively re-exported by Swift, so the test target had no declaration scope
for those DTOs.

## Correction

`BridgeModelScenarioTests.swift` now explicitly imports `TableRockFeature`,
matching its direct use and the Xcode test target's declared dependency. No
application API was widened and no duplicate DTO was introduced.

## Verification

- Hosted run 29843185342 passed `Run Swift bridge and feature tests`, proving
  the JSON-number and PageV1 corrections before reaching this independent
  Xcode-only compile failure.
- The Xcode log names the missing DTOs and the `TableRockAppTests` target.
- Hosted Xcode rerun remains pending after push.

## Provenance

No external product reference influenced this build-graph correction. Evidence
comes from TableRock's target definitions and hosted compiler diagnostics.
