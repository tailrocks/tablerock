# Native macOS Path

The maintainable native architecture is SwiftUI/AppKit presentation over the
same Rust service contracts, not AppKit rewritten wholesale in Rust and not a
WebView shell.

## Alternatives

Direct AppKit through [`objc2`](https://github.com/madsmtm/objc2) is technically
viable but makes Rust own Objective-C lifecycle, delegates, selectors, reference
counting, and main-thread rules. Reserve it for narrow adapters or custom
rendering only.

Rust UI frameworks are useful for cross-platform/custom interfaces but do not
automatically provide AppKit controls, text behavior, accessibility, menus,
focus, drag/drop, or VoiceOver. They are not the primary macOS choice.

## Recommended ownership

```text
SwiftUI / AppKit
  lifecycle, windows, menus, tabs, focus, accessibility
  NSTableView/NSOutlineView grid/catalog
  native source editor, sheets, settings, Keychain/file/clipboard UI
                 |
                 v
thin Swift adapter or local daemon client
                 |
                 v
Rust TableRock core
  profiles, sessions, drivers, catalog, queries, pages, edits, history
```

Rule: Swift owns Apple objects. Rust owns database state and computation.

Do not pass NSView, SwiftUI View/Binding, windows, or table objects into Rust.
Do not expose client-library objects to Swift.

## Daemon-owned live sessions

Before the native app, introduce a versioned local protocol and
`tablerock-daemon`:

```text
tablerock TUI ----+
                  +--> tablerock-daemon --> Rust engine/sessions
native macOS app -+
```

The daemon owns connections, queries, result buffers, cancellation, history,
safety, and secret resolution. This prevents duplicate pools/state/policy.
Authenticate local peers, bound subscriptions, handle version mismatch, and do
not return raw credentials to Swift.

The first TUI stays in-process behind the same command/event/page abstraction.
Moving to daemon RPC changes the effect adapter, not domain or presentation.

## Grid and editor

Use NSTableView/NSOutlineView or a measured purpose-built AppKit view. Scrolling
requests coarse result pages. For large pages, use one immutable encoded buffer
plus metadata rather than thousands of FFI records/calls.

Keep text input, cursor, selection, IME, native find, accessibility, and
completion presentation in Swift/AppKit. Rust receives revisioned text/edits and
returns statement/token/diagnostic/completion/formatting data.

## Interop

Coarse operations:

```text
open profile -> session ID
subscribe -> events
execute -> query ID
fetch page -> encoded page
cancel -> outcome
stage/review/apply -> operation IDs/events
```

Avoid per-cell calls, retained callbacks without cancellation, unspecified
executors, Rust panics across FFI, borrowed-lifetime ambiguity, implicit
Swift/Tokio cancellation, and duplicate authoritative state.

The primary live path is local RPC. If embedded Rust is later justified, compare
then-current UniFFI with a narrow manual C ABI. UniFFI's MPL-2.0 license and
Swift 6 concurrency behavior require explicit review. Build XCFramework/binding
tooling only after choosing embedding.

## Acceptance

- one Rust query/result/edit contract drives terminal and native clients;
- one authoritative live session in normal mode;
- native AppKit controls and complete VoiceOver/keyboard behavior;
- no per-cell FFI/RPC;
- explicit Swift-to-Rust-to-server cancellation;
- documented credential/sandbox ownership;
- clear protocol/version failure instead of undefined behavior;
- no Swift database driver implementation.
