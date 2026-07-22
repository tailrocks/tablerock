# Evidence 650: native SSH configuration and session tunnel

## Claim

TR-SCR-055 now has a native SwiftUI connection-editor projection backed by the
existing Rust profile and `russh` tunnel architecture. Native can enable a
bastion, edit host/port/user, choose SSH agent/password/OpenSSH-private-key
authentication, and supply the mandatory OpenSSH `known_hosts` path. Host-key
verification remains fail closed.

`BridgeProfileDraft` carries typed SSH configuration and write-only secret
inputs. Profile reads return only stored-secret presence flags. Save validates
required fields, port, auth mode, and explicit local-plaintext acknowledgement;
an unchanged edit preserves existing SSH secret material inside Rust. Native
profile open resolves the saved SSH auth in Rust, opens the shared local-forward
tunnel below every database client, rewrites only the driver endpoint, and
retains the tunnel for the registered session lifetime.

Status remains `partial`: hosted XCUITest for the editor's SSH test action and
the native live password/key/agent/host-key failure matrix remain open. The
underlying shared tunnel's real bastion matrix is evidence 260–269.

## Verification

```text
mise exec -- cargo test -p tablerock-ffi --test conformance --locked
mise exec -- cargo clippy -p tablerock-ffi --all-targets --locked -- -D warnings
mise exec -- ./scripts/generate-swift-bindings.sh
mise exec -- ./scripts/build-native-app.sh --configuration Release
mise exec -- ./scripts/verify-native-profile-editor.sh
```

Conformance passed 21 tests, including native SSH profile round-trip, secret
non-projection, and unchanged-edit preservation. Clippy and native Swift 6
Release build passed. Runtime editor audit rendered the enabled SSH section,
agent choice, fail-closed host-key policy, and stable automation identifiers.

## Clean-room provenance

Current public TablePro workflow search was performed before this connection
screen checkpoint; no accessible SSH-specific public documentation or screenshot
was found. TablePro influenced only the broad expectation that advanced
connection configuration belongs with the connection editor. No TablePro
source, tests, identifiers, text, assets, colors, geometry, measurements, or key
bindings were read or copied. Requirements, UI expression, bridge design, and
tests are TableRock-owned and derive from its product docs and existing Rust SSH
contracts.
