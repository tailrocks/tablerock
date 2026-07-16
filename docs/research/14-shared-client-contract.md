# Shared Rust Core And Client Contract

## Decision

One Rust application service owns TableRock behavior for both clients. The TUI
calls it in-process. The native macOS app calls the same commands/events/pages
through an embedded UniFFI bridge.

Swift owns native Apple presentation and platform integrations. It never owns
database drivers, SQL/Redis semantics, profiles, session truth, result storage,
mutation plans, history policy, reconnect, redaction, or safety decisions.

The selected architecture has no daemon or local RPC. Direct Developer ID
distribution embeds and signs the Rust library inside the native application.

## Ownership

| Concern | Rust | TUI | Swift/AppKit |
|---|---|---|---|
| Profiles and profile schema | authoritative | edit projection | edit projection |
| Credentials | source references, resolution policy, transient secret envelope | prompt/1Password interaction | Keychain/native-auth adapter where selected |
| Connections and pools | authoritative | state snapshot | state snapshot |
| Catalog | authoritative, revisioned | tree projection | outline projection |
| SQL/Redis parsing and completion | authoritative service | editor buffer/popup | native editor buffer/popup |
| Queries/commands | execution, limits, cancel, progress | intent and rendering | intent and rendering |
| Results | bounded immutable batches/pages | viewport cache | coarse page cache |
| Edits | typed mutation queue, review, apply | selection/input | selection/input |
| History/restoration | durable policy and data | terminal-only focus/layout | window/focus/layout plus Rust intent |
| Safety/redaction | mandatory enforcement | cannot bypass | cannot bypass |
| Accessibility/focus | semantic data | terminal focus/hints | VoiceOver/native focus |
| Windows/menus/files/pasteboard | no Apple objects | terminal equivalents | authoritative native objects |

## Rust layers

```text
tablerock-core
  owned IDs, capabilities, values, commands, events, pages, errors

tablerock-engine
  application service, policy, sessions, result store, history, drivers

tablerock-tui
  TermRock-based Model/Message/Update/View and in-process effect adapter

tablerock-cli
  binary, Tokio runtime, terminal/process/platform adapters

later tablerock-ffi
  coarse UniFFI facade and generated Swift bridge
```

The bridge facade is introduced only after the command/event contract has run
all three engines in-process. FFI-safe ownership is a core-contract constraint
from day one.

## Application service seam

The presentation boundary is coarse and asynchronous:

```text
submit(CommandEnvelope) -> OperationId or immediate rejection
subscribe(SessionScope, EventCursor) -> bounded event stream
fetch_page(PageRequest) -> ResultPage
cancel(OperationId) -> CancelRequestOutcome
shutdown(ShutdownMode) -> ShutdownOutcome
```

The Rust application-service trait implements these semantics for both adapters.
TUI code never calls a driver directly. Swift never calls a driver or
reconstructs a database operation.

Below that service, the engine's dyn-compatible `DriverSession`/`DriverPageStream`
traits use boxed futures and owned bounded requests/pages. Both clients see core
contracts only; PostgreSQL, ClickHouse, and Redis client types never enter the
service or presentation boundary.

The core coordinator now owns bounded operation submission, parent-scope
containment, cancellation transitions, per-operation delivery queues, terminal
retirement, and graceful/cancel-active draining. Shutdown stops new submission
immediately but remains `Draining` until every active operation reports a legal
terminal outcome; it never manufactures cancellation success. A bounded
application/profile/session/context registry owns aggregate revisions, requires
parent-before-child registration, and validates command expectations before
submission.

Subscriptions use opaque IDs and independent bounded per-operation queues.
Late cursors receive explicit resync, future cursors fail, and one slow client
cannot force another client's queue to drop or block. Retirement requires every
subscriber to drain and unsubscribe.

### Command envelope

Every command carries:

- core/FFI schema version where encoded;
- request ID for correlation, not automatic write retry;
- profile/session/context/tab IDs as required;
- expected aggregate revision for conflict/stale detection;
- typed intent and bounded arguments;
- deadline/budget and optional parent cancellation scope;
- safety context or review token where required;
- redaction classification.

Representative commands:

```text
TestProfile / SaveProfile / RemoveProfile
Connect / Disconnect / SelectContext / RefreshCatalog
OpenObject / Execute / FetchPage / Cancel
StageMutation / ReviewMutations / ApplyMutations / DiscardMutations
LoadHistory / SaveQuery / Export / Import
```

### Event envelope

Every event carries enough identity to reject stale delivery:

```text
profile/session/context/query/result/operation IDs
aggregate revision and event sequence
safe event kind and bounded payload
completion/truncation/cancellation/failure state
safe diagnostic code and optional operator action
```

Operation identity uses the same typed application/profile/session/context
scope as its originating command. No optional-ID envelope or context-only
special case exists.

Progress may be coalesced only as cumulative consecutive updates. State
transitions, review requirements, terminal results, and failures cannot be
silently dropped. The bounded core queue converts capacity exhaustion or a
producer gap into an explicit resync requirement instead of growing storage or
guessing state.

## Result/page contract

A page is one immutable owned object:

```text
ResultPage
  result_id, result_revision
  global row range and known/unknown total
  column metadata and engine type facts
  encoded value buffer plus offsets/null/truncation metadata
  completion and warning facts
```

Requirements:

- no database client row or borrowed lifetime crosses the service boundary;
- no call, callback, or Swift object per cell;
- byte and row limits are checked before allocation and decoding;
- unknown/unsupported values remain inspectable and typed as unknown;
- binary and text stay distinct;
- database-native containers use a distinct bounded `Structured` projection;
  presentation never guesses structure from ordinary text;
- pages are independently disposable and safe to decode off the main thread;
- the native adapter publishes only the final immutable page to `@MainActor`;
- the TableRock versioned columnar byte-arena encoding is the sole native page
  format; Arrow is not part of this architecture.

## Cancellation and ambiguous outcomes

Cancellation is a state machine, not a dropped task:

```text
Running
  -> CancelRequested
  -> ClientStopped | ServerConfirmed | CompletedBeforeCancel | Unknown
```

- Swift task cancellation and TUI escape/Cancel actions submit the same Rust
  cancellation command.
- Dropping an FFI call or Swift task does not claim server cancellation.
- Reconnect never repeats an ambiguous write.
- A late terminal event is retained against the operation record even if the
  originating tab/window closed.
- Disconnect and engine termination preserve “outcome unknown” when proof is
  impossible.

## Errors and panics

- Recoverable failures use typed Rust errors with safe engine code/severity/
  position and a separately classified diagnostic message.
- Raw driver errors are redacted before any presentation/transport boundary.
- SQL, Redis arguments, credentials, and cell values are absent by default.
- Bridge decoding rejects unknown required fields, excessive sizes, invalid
  state transitions, and unsupported versions.
- Rust panics never unwind over an FFI boundary. The
  [Rust Reference](https://doc.rust-lang.org/stable/reference/panic.html)
  documents undefined behavior for mismatched unwinding ABIs.
- An engine failure becomes a visible disconnected state and never triggers an
  automatic ambiguous-write replay.

## Secrets

Stable core snapshots contain secret references, never resolved values:

```text
OnePasswordReference
PromptOnConnect
HostEnvironmentReference
KeychainReference          # native-only source handle
DangerousPlaintextSecret   # explicit local-test exception
```

Resolution rules:

1. Rust requests only the fields needed for Test/Connect.
2. The 1Password CLI adapter lives in Rust process/service code under the
   selected direct distribution. Official
   [secret references](https://www.1password.dev/cli/secret-references) are
   resolved only at runtime.
3. A native Keychain implementation is a narrow Swift platform adapter because
   Apple's [Keychain Services](https://developer.apple.com/documentation/security/keychain-services)
   is a native communication surface. Swift returns a transient byte buffer to
   the Rust request and does not publish it in observable UI state.
4. Resolved secrets are zeroized where practical, excluded from debug output,
   never serialized into protocol events, and not cached beyond connection
   ownership requirements.
5. Engine logs, crash reports, history, telemetry, and diagnostics receive only
   safe source IDs and outcomes.

## TUI adapter

The first client runs `tablerock-engine` in the CLI process:

```text
TermRock update Effect
  -> in-process EnginePort command
  -> Tokio engine task
  -> bounded EngineEvent stream
  -> TUI Message
```

The adapter deliberately serializes through owned command/event/page types even
before wire encoding exists. Tests run the same contract harness against a fake
port and the real in-process port.

## Native macOS adapter

The native process is structured around current Apple-native seams:

- SwiftUI `App`, `WindowGroup`, `Commands`, and `Settings` own lifecycle,
  windows, menus, and preferences. Apple documents that
  [`WindowGroup`](https://developer.apple.com/documentation/swiftui/windowgroup)
  gives independent per-window state and native window management.
- A `@MainActor` workspace/store receives immutable Rust snapshots/events and
  owns only presentation state.
- An actor-owned bridge client performs bounded decoding, event polling,
  reconnection/resubscription where applicable, and cancellation away from the
  main actor.
- `NSOutlineView`, `NSTableView`, and `NSTextView` own catalog, grid, and editor
  presentation. SwiftUI integrates
  AppKit via
  [`NSViewRepresentable`](https://developer.apple.com/documentation/swiftui/nsviewrepresentable)
  and a coordinator; SwiftUI remains the layout owner.
- Standard AppKit controls provide accessibility by default; custom views must
  implement the required
  [AppKit accessibility](https://developer.apple.com/documentation/appkit/accessibility-for-appkit)
  roles and values.
- File panels, security-scoped access, pasteboard, drag/drop, Keychain, local
  authentication, app activation, and code signing remain thin native adapters.

Swift may format presentation-only labels, dates, and sizes. It may not parse
SQL for safety, classify Redis commands, infer editability, build executable
mutations, or apply independent limits.

## UniFFI bridge gate

UniFFI generates the Swift binding over one synchronous, coarse Rust facade.
Rust owns Tokio and long-running work; a non-main Swift actor polls bounded event
batches. Avoid generated async functions because UniFFI's current
[Swift guide](https://mozilla.github.io/uniffi-rs/latest/swift/overview.html)
documents partial Swift 6 support and async `Sendable` gaps. Apple documents
[XCFramework packaging](https://developer.apple.com/documentation/xcode/creating-a-multi-platform-binary-framework-bundle)
for the embedded multi-architecture library.

The bridge must prove strict concurrency, cancellation by operation ID,
callback/event lifetime, panic containment, deterministic destruction, bounded
buffer ownership, static-library/XCFramework packaging, signing, leak freedom,
and measured page/scroll performance. Failure blocks native work until this
single architecture decision is explicitly revised.

## Cross-adapter conformance

The in-process port and UniFFI bridge run the same tests:

- command validation and capability filtering;
- event ordering, stale revision rejection, and resync;
- page byte-for-byte semantic equivalence;
- cancellation terminal states;
- redaction and oversized-payload rejection;
- disconnect/reconnect/restart behavior;
- mutation review token use and expiry;
- ambiguous-write non-retry;
- shutdown with pending reads and writes.

The in-process mutation seam uses move-only reviewed and authorized plan
wrappers. The core registry already owns bounded opaque, expiring, single-use
handles and consumes a handle even when authorization fails. Because serialized
bytes can be copied, the native bridge will expose this registry rather than
plan bytes. Swift may render a bounded preview but never reconstruct executable
work from that preview.

The native client does not begin broad feature work until this suite passes for
all three engines through the in-process port and UniFFI bridge.

Transport selection and Apple-native ownership are detailed in
[12-native-macos-path.md](12-native-macos-path.md); primary-source evidence is
in [13-platform-architecture-sources.md](13-platform-architecture-sources.md).
