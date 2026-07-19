# Native test-root isolation

Date: 2026-07-19

## Contract

- Native startup resolves environment input once into typed
  `AppConfiguration` and `AppPaths` values.
- Production persistence remains under
  `Application Support/TableRock/profiles.db`.
- Explicit test mode requires an absolute `TABLEROCK_TEST_ROOT`; malformed or
  incomplete test configuration fails closed.
- Existing `TABLEROCK_FIXTURE_*` launches automatically use a process-local
  temporary root instead of the developer's Application Support data.
- Scripted backend selection is typed and requires a named scenario. The live
  application rejects it until the replaceable backend boundary lands.
- Configuration changes only external capability routing. They do not duplicate
  Rust domain behavior in Swift.

## Evidence

- `DYLD_LIBRARY_PATH=../target/release swift test -c release`: 18 tests in four
  suites passed, including five `AppConfiguration` and path-isolation tests.
- `./scripts/build-native-app.sh`: direct `swiftc` application build passed with
  the importable feature module linked into the bundle.
- `./scripts/verify-native-accessibility.sh`: structural and runtime gate passed
  while fixture persistence was isolated from real user data.
- The direct build compiles the feature target with `-parse-as-library`, keeping
  SwiftPM and the ad-hoc development application on the same configuration code.

## Remaining testing-system scope

- Extract the replaceable workbench backend and scripted presentation scenarios.
- Inject Keychain, file-panel, pasteboard, clock, and UUID capabilities.
- Add the Xcode application/UI-test targets and cadence-specific test plans.
- Exercise the exact XCFramework-linked Release application separately.

## Provenance

The testing layers and isolation requirements derive from the operator-supplied
macOS/Rust testing model and this repository's architecture. TablePro establishes
only the broad implementation-workflow concept; no external source, tests,
identifiers, product text, assets, or code were copied or translated.
