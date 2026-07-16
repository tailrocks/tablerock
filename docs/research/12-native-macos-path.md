# Native macOS Path

## Decision

Build a direct-distribution native SwiftUI/AppKit application with the Rust
engine embedded through UniFFI. Swift owns Apple UI and operating-system
integration. Rust owns profiles, database behavior, sessions, catalogs,
results, edits, history, redaction, and safety.

There is no daemon, local RPC protocol, manual C ABI, WebView, or Rust-owned
AppKit object in the selected architecture. Direct Developer ID distribution,
hardened runtime, notarization, and stapling are the first macOS release path.
The Mac App Store is outside the first program.

## Native ownership

```text
SwiftUI App / WindowGroup / Commands / Settings
  windows, menus, toolbar, settings, native restoration
                       |
                       v
AppKit-backed controls through NSViewRepresentable
  catalog, large grid, SQL/command editor, native find and IME
                       |
                       v
thin generated UniFFI plus Swift adapter
  @MainActor presentation store, platform capability adapters
                       |
                       v
coarse Rust command/event/page facade
                       |
                       v
tablerock-engine and three database adapters
```

Use SwiftUI's [`App`](https://developer.apple.com/documentation/swiftui/app),
[`WindowGroup`](https://developer.apple.com/documentation/swiftui/windowgroup),
[`Settings`](https://developer.apple.com/documentation/swiftui/settings), and
[commands](https://developer.apple.com/documentation/swiftui/menus-and-commands)
for application structure. Use
[`NSViewRepresentable`](https://developer.apple.com/documentation/swiftui/nsviewrepresentable)
to host AppKit controls where database-workbench interaction requires them.

Use `NSOutlineView` for the lazy catalog, `NSTableView` for large editable
results, and `NSTextView`/TextKit for SQL and
Redis command editing. SwiftUI remains the application/layout shell; AppKit is
not a parallel product architecture.

## Swift allowance

Swift is allowed for:

- scenes, windows, menus, commands, settings, toolbars, sheets, alerts, focus,
  drag/drop, pasteboard, and file panels;
- AppKit catalog/grid/editor adapters and native accessibility;
- `@MainActor` presentation stores and immutable view projections;
- Keychain, LocalAuthentication, security-scoped files, app activation,
  entitlements, signing, and notarization;
- the narrow bridge wrapper and buffer decoding needed to call Rust safely.

Swift must not implement:

- database connections, SQL execution, Redis command classification, catalog
  semantics, mutation generation, or editability rules;
- profile truth, result retention, history policy, reconnect, cancellation
  truth, redaction, or safety policy;
- an independent SQL parser/completion engine or per-engine value model;
- per-cell calls into Rust or duplicated caches that become authoritative.

Presentation-only date, size, and accessibility labels may be formatted in
Swift. Any label that changes database semantics or safety comes from a
Rust-owned typed fact.

## Stable bridge facade

UniFFI exposes one coarse facade:

```text
open(profile) -> SessionId
submit(CommandEnvelope) -> OperationId or rejection
next_events(cursor, maximum) -> bounded event batch
fetch_page(result_id, range, revision) -> encoded immutable page
cancel(operation_id) -> request outcome
shutdown(deadline) -> shutdown outcome
```

IDs, revisions, commands, events, values, pages, redaction, and cancellation
outcomes are stable Rust contracts. Rust driver objects, Tokio handles, Swift
objects, and borrowed rows do not cross the seam. One event/page batch crosses
per call; never one object or callback per cell.

## UniFFI bridge

Use UniFFI-generated Swift bindings over a Rust `staticlib`, packaged for Apple
architectures as an XCFramework. Keep the exported facade synchronous and
coarse; Rust owns Tokio and long-running work, while Swift polls bounded event
batches from a non-main actor. This avoids relying on UniFFI-generated async
functions while its
[Swift guide](https://mozilla.github.io/uniffi-rs/latest/swift/overview.html)
documents partial Swift 6 support and async `Sendable` limitations.

The UniFFI bridge must prove:

- Swift 6 strict-concurrency build without blanket unchecked `Sendable`;
- explicit Rust runtime ownership and idempotent destruction;
- typed errors, panic containment, and valid state after recoverable failure;
- operation-ID cancellation independent of dropping a Swift task;
- buffer ownership, bounded decoding, leak freedom, and low allocation count;
- generated-artifact determinism and universal signed packaging, using an
  [XCFramework](https://developer.apple.com/documentation/xcode/creating-a-multi-platform-binary-framework-bundle)
  as the selected packaging shape;
- stable page latency and scrolling in Instruments at measured result sizes.

Failure of this gate blocks native implementation and triggers a new recorded
architecture decision; no unplanned bridge is carried as a parallel path.

## Concurrency and cancellation

Keep all native view mutation on
[`MainActor`](https://developer.apple.com/documentation/swift/mainactor). Decode
bounded Rust event/page buffers off the main actor, then publish one immutable
snapshot. Rust callbacks never touch AppKit and Swift never holds a UI lock
while entering Rust.

Swift task cancellation submits `Cancel(OperationId)` and continues observing
the Rust operation's terminal outcome. It does not claim that PostgreSQL,
ClickHouse, or Redis stopped. The shared cancellation state machine is defined
in [14-shared-client-contract.md](14-shared-client-contract.md).

## Credentials and distribution

Ship the native application as a direct Developer ID distribution with the
hardened runtime, notarization, stapling, update/uninstall tests, and signing of
the embedded Rust framework. The Rust CLI and native Rust engine resolve
`op://` references through the 1Password CLI. The native client also keeps a thin
[`Keychain Services`](https://developer.apple.com/documentation/security/keychain-services)
adapter as a platform capability while Rust owns secret-source mapping,
redaction, lifetime, and connection use.

## Accessibility and native quality

Use standard controls first. Wrapped/custom controls expose correct
[AppKit accessibility](https://developer.apple.com/documentation/appkit/accessibility-for-appkit)
roles, values, selection, and actions. Verify VoiceOver, complete keyboard
operation, menu discoverability, focus restoration, IME/marked text, reduced
motion, contrast, large text, light/dark appearance, and multi-window state.

## Acceptance

- one Rust behavior contract drives terminal and native clients;
- embedded Rust through UniFFI passes signed Release-build evidence;
- Swift contains only native presentation/platform integration and bridge code;
- no database client, policy, result truth, or executable mutation is rebuilt
  in Swift;
- cancellation and ambiguous writes remain honest across the boundary;
- no per-cell FFI call;
- clean-machine signing, notarization, credentials, restoration,
  accessibility, performance, and crash-recovery gates pass.

Primary-source detail and citations are collected in
[13-platform-architecture-sources.md](13-platform-architecture-sources.md).
