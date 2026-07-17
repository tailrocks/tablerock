# Quality And Verification

## Rule

No phase exits from implementation presence alone. It exits when the selected
architecture and behavior pass the evidence in this document. Known failure is
never accepted as a low-priority exception; it blocks the affected claim.

## Evidence pyramid

| Layer | Subject | Required evidence |
|---|---|---|
| Domain unit | IDs, values, capabilities, pages, safety, redaction | examples, boundaries, property tests |
| TEA reducer | Model + Message -> Model + Effects | deterministic transitions, stale events, modal/focus, cancellation, edits |
| TermRock widget | neutral rendering and interaction | direct Buffer, keyboard/mouse, Unicode, clipping, minimum rectangle |
| View | complete TableRock TUI screens | TestBackend at narrow/normal/wide sizes and all operation states |
| Effect adapter | TUI-to-engine command/event seam | fake port, bounded channel, timeout, cancellation, shutdown |
| Driver contract | each real database | pinned-server integration and failure injection |
| Persistence | Turso schema and async actor | migrations, crash recovery, corruption/disk/ownership handling, retention |
| Security | secrets, safety, files, processes, bridge | adversarial fixtures, redaction scans, permission and injection tests |
| PTY | Crossterm/TermRock terminal lifecycle | setup/restore, signals, panic, resize, paste, focus, mouse |
| UniFFI | Rust/Swift ownership and semantics | strict concurrency, lifetime/free stress, pages, cancel, panic, leaks |
| Native UI | SwiftUI/AppKit presentation | unit/UI/accessibility/IME/multi-window/restoration tests |
| Distribution | actual Release artifact | clean-machine signature, notarization, credentials, update/uninstall |

## TEA architecture checks

Architecture tests and review prove:

- one root model and root message/update path;
- all I/O represented as typed effects;
- no `await`, file, process, secret, database, telemetry, or clock access in
  reducers and views;
- all async completions validated by operation/session/context/revision identity;
- one root modal/focus precedence and no component-local event loop;
- complete view rendering from resident model data;
- bounded subscriptions/event queues with explicit resync on overflow;
- progress coalescing never drops state transitions or terminal outcomes.

## TermRock component gate

Every new neutral primitive added for TableRock must pass in TermRock before
TableRock advances its pin:

- product-neutral public API and caller-owned policy;
- borrowed render data and stable semantic IDs;
- no database, Tokio, process, secret, or TableRock/Jackin product dependency;
- canonical lookbook stories and deterministic previews;
- direct Buffer fixtures for empty/loading/error/disabled/focused/hovered/
  selected states;
- keyboard/mouse parity, focus, clipping, tiny rectangles, wide Unicode,
  grapheme/cell-width, and non-color tests;
- benchmark and allocation evidence for grid/editor/scroll hot paths;
- generated documentation and compatibility metadata;
- Jackin build/tests when an existing TermRock contract changes;
- DCO-signed direct commit pushed to TermRock `main`, never a branch or pull
  request.

TableRock then pins the full TermRock revision in a separate buildable direct
`main` commit and runs its own complete suite.

## Core properties

Property and model tests cover:

- stable ID uniqueness and serialization round trips;
- monotonic revisions and stale-event rejection;
- value distinction: NULL, empty, whitespace, zero, false, bytes, invalid,
  unknown, and truncated;
- page offsets/ranges/bounds and hostile encoded input;
- capability filtering and explicit unsupported states;
- mutation review-token scope/expiry, exact-once registry redemption, and no
  display-text execution;
- safety monotonicity: presentation cannot weaken Rust policy;
- redaction idempotence and absence of forbidden fields;
- cancellation state-machine legality and ambiguous-write non-retry;
- bounded overlapping-operation capacity, parent-scope containment, terminal
  retirement, and shutdown draining without invented outcomes;
- bounded hierarchical scope ownership with stale/future command rejection and
  monotonic revision advance;
- bounded multi-subscriber fan-out, late/future cursor handling, independent
  slow-subscriber resync, and explicit subscription retirement;
- eviction/resync without dangling page or selection identity.

## Real-server matrix

Run isolated real instances for each supported server row. Every adapter must
pass connect, authentication failure, TLS, context selection, catalog, query or
command, bounded streaming, typed values, page, timeout, cancel, disconnect,
reconnect, permissions, safe error, and ambiguous-write tests.

Engine additions:

- one object-safe adapter contract exercised by all three real-server suites;
- cross-engine request rejection and redacted adapter diagnostics;
- explicit cancellation support/unsupported truth and consuming shutdown;
- cancellation remains reachable during stream creation; dispatch transport
  and server confirmation remain separate facts, including PostgreSQL SQLSTATE
  confirmation through the real service path;
- PostgreSQL 17.10/18.4 require verified custom roots, independent server-name
  validation, plaintext downgrade rejection, optional mTLS identity, hostile
  PEM rejection, and cancellation through the identical retained connector;
- ClickHouse cancellation binds the active query ID, requires synchronous
  `finished` server status, reads no returned query text, and proves the
  terminal mapping across both pinned server lines and compression modes;
- bounded engine-owned task/control/event channels, cancellation under event
  backpressure, idempotent dispatch, authoritative task exit, and client-stop
  shutdown independent of slow event consumers;
- core-authoritative service/runtime mapping, terminal-event versus joined-exit
  agreement, immediate-cancel non-regression, rejected-submission cleanup, and
  real PostgreSQL service execution;
- one reusable core/runtime harness proving real bounded service execution for
  PostgreSQL, ClickHouse, and Redis across the pinned version/protocol matrix;
- one current-production-line performance harness enforcing 10,000-row,
  500-row-page first-page, completion, throughput, page-residency, and process
  RSS budgets through the shared adapter boundary;
- Redis blocking cancellation uses a separate control connection, waits until
  the operation connection is observably blocked, and proves both successful
  `CLIENT UNBLOCK` dispatch and the operation-side server error under RESP2/3;
- Redis multi-user live-revocation fixtures pipeline all administrative kills
  before awaiting replies, preventing stale reconnect activity from racing the
  remaining revocation commands;
- Redis initial and replacement Pub/Sub generations share one bounded,
  cancellable connection-attempt policy; required-TLS connection deadlines map
  to redacted connect failure while plaintext blackholes retain timeout truth;
- Redis raw TLS fixture administration uses explicit five-second connection and
  response budgets plus PING/PONG readiness instead of redis-rs's 500 ms raw
  response default; product timeout policies remain independently tested;
- simultaneous PostgreSQL, ClickHouse, and Redis submission before event
  consumption, with independent page identity/data, bounded receives, and
  observed completion for every operation;
- graceful versus cancel-active drain, bounded per-operation client-stop facts,
  slow-delivery-independent terminal reconstruction, premature completion
  rejection, and exactly-once runtime release;

- PostgreSQL: custom/unknown OIDs, composites/JSON/bytes, notices,
  parameters, COPY, multiple statements, transaction conflicts, cancel races;
  pinned real servers distinguish SQLSTATE-confirmed cancellation from a late
  successfully delivered cancel after normal query completion, and a bounded
  synchronization barrier prevents pending cancellation from escaping into the
  next operation;
  forced server loss before cancel-socket delivery remains a distinct redacted
  cancellation-transport failure followed by terminal session connection loss;
  prepared text, int8, binary, and boolean parameters retain exact typed values
  through bounded pages on both pinned lines;
  generic arrays retain dimensions, lower bounds, nesting, and NULL elements in
  bounded canonical structured values on both pinned lines;
  declared NULL remains null and `int4[]` retains structured array identity on
  both pinned lines;
  JSON and JSONB retain deterministic bounded structured projections with
  arbitrary-precision numbers and explicit invalid/over-ceiling behavior;
  native numeric retains arbitrary precision, scale/trailing zeros, scaled
  zero, NaN, infinities, and bounded malformed/over-cell-limit behavior;
  UUID retains canonical lowercase 8-4-4-4-12 text, exact truncation length,
  nil/maximum values, and malformed-length behavior;
  generic ranges retain explicit empty/unbounded/inclusive/exclusive truth in
  bounded canonical structured values; anonymous records retain bounded unknown
  binary bytes with exact type/truncation truth; large `bytea` remains binary;
  notices retain bounded severity/SQLSTATE/message, UTF-8 truncation truth,
  ordered capacity, redacted Debug, and explicit overflow on both pinned lines;
  optional notice detail/hint retain independent bounds, presence, truncation,
  and Debug redaction on both pinned lines;
  fixed multiple-statement batches retain ordered command/query outcomes and
  exact row counts without introducing a second typed-row path;
  bounded COPY OUT retains ordered chunk offsets and exact bytes without
  accumulation; bounded backpressured COPY IN returns server-confirmed rows;
  limit and malformed-input failures remain distinct, redacted, and recoverable;
  a dispatched write with an unobserved completion maps to unknown, an
  independent observer may see exactly one durable application, and neither
  session retries it;
  an explicit transaction whose deferred commit work outlives response
  observation remains unknown, may commit exactly once, and is never replayed;
  transport loss gated on active deferred COMMIT leaves old sessions terminal,
  requires refreshed endpoint facts for explicit recovery, may roll back, and
  never replays;
  the same active-COMMIT loss under custom-root required mTLS terminates old TLS
  sessions, rejects plaintext recovery, revalidates identity, preserves rollback
  observation, and never replays;
- ClickHouse: nested/nullable/low-cardinality/decimal/large integer/binary,
  partial/late HTTP errors, compression, query IDs, parts, inserts, mutations;
- Redis: binary keys/values, SCAN families, RESP2/RESP3, logical DB isolation,
  per-command pipeline partial failures and transaction no-rollback truth,
  Pub/Sub, blocking commands, exact key TTL states, reviewed TTL mutation,
  post-dispatch cancellation.

The current Redis 7.4.9/8.8.0 RESP2/RESP3 matrix continuously proves binary
SCAN, HSCAN, SSCAN, and ZSCAN bounded pages. The same matrix proves
stable-throughout and absent-throughout guarantees during concurrent mutation,
while accepting legal duplicates and leaving transient membership undefined. A
pre-decode transport allocation cap remains a separate required gate. Accepted
decoded collection batches and all retained pending state have explicit entry
and byte bounds.

The Redis matrix also proves configured response timeout, distinct timeout and
connection-loss categories, confirmed-drop future-call reconnect, RESP3
proactive reconnect allowance, logical database retention, and disposable
blocking-operation client identity. Server restart, DNS change, and write
ambiguity remain separate required gates.

Redis 7.4.9/8.8.0 under RESP2/RESP3 also pass generated custom-root TLS and
required-mTLS fixtures. Wrong roots, hostname mismatch, and plaintext fallback
fail closed; wrong initial ACL credentials stop before reconnect policy and map
to a redacted authentication class. TLS-authenticated future-call reconnect and
blocking cancellation also pass. Password rotation followed by confirmed
user-connection termination stops the next future operation with bounded,
redacted authentication failure. Server-observed active channel and pattern
subscriptions also terminate as bounded authentication failure after the same
revocation. TLS/auth material has explicit
pre-I/O bounds and Debug redaction tests.

The Redis 7.4.9/8.8.0 RESP2/RESP3 matrix also proves dedicated Pub/Sub
connections, exact binary channel/payload delivery, continued ordinary-command
use, bounded queue configuration, client-stop cancellation, and unsubscribe
ownership release. The object-safe service path terminates cancellation as
client-stop; active drop permits a replacement generation; cancellation races a
server-paused setup without waiting for the server response timeout. Queue
overflow is an explicit resource-limit failure, never silent loss. Pattern
subscriptions additionally prove exact binary pattern/channel/payload delivery,
three-column and selector bounds, pre-queue field truncation with original-length
metadata, adapter transport, client-stop teardown, and zero remaining patterns.
Server replacement on the same endpoint additionally proves bounded
reconnect/resubscription, an ordered zero-row delivery-discontinuity page before
restored channel and pattern messages, per-attempt blackhole timeout, bounded
attempt exhaustion, and prompt cancellation during a subsequent outage. DNS
change and RESP2 pre-decode
transport allocation bounds remain required.

The Redis TLS-only 7.4.9/8.8.0 matrix under RESP2/RESP3 proves channel and
pattern Pub/Sub with custom roots, optional required client identity, ACL
credentials plus explicit channel patterns, exact binary pages, and
authenticated client-stop teardown. TLS/mTLS same-endpoint server replacement
also restores both subscription kinds with an ordered discontinuity page before
binary delivery and prompt cancellation. Untrusted and recredentialed
replacement servers terminate as distinct bounded connect/authentication
failures before any discontinuity page is emitted. A restricted `&allowed:*` user
also proves server-side denial across this matrix. Adapter-level denial remains
required: the latest redis-rs Pub/Sub setup method discards the server-error
value, and an administrative `ACL DRYRUN` preflight is not product evidence.

Fixed-port Redis restart fixtures require bounded protocol readiness after the
container log wait. They retry only connect, connection-loss, and timeout
availability failures; authentication, TLS-configuration, and protocol failures
remain immediate. Negative replacement deadlines cover their configured
connection, response, retry-count, and backoff ceilings.
Restart fixtures configure a nontrivial minimum reconnect backoff so immediate
connection refusal cannot consume the full retry budget before the intentional
same-endpoint replacement is protocol-ready.

Reviewed Redis TTL mutation consumes exact-once authorized plans and passes the
Redis 7.4.9/8.8.0 RESP2/RESP3 matrix. Missing/already-persistent no-change,
applied expiration/persistence, database and plan rejection before I/O, signed
duration bounds, and a write-applied-after-client-timeout ambiguity are covered.
Unknown write outcomes are never automatically retried.

A support claim is exactly the continuously passing real-server matrix.

## TUI render and interaction matrix

Each major screen has owned fixtures at:

- minimum supported, narrow, medium, wide, and very wide terminal sizes;
- empty, loading, partial, success, stale, disconnected, cancelled, failure,
  permission-denied, truncated, and pending-change states;
- short, long, combining, double-width, emoji, RTL-containing, invalid-byte,
  and control-character-safe projections;
- keyboard-only and mouse paths;
- focused/hovered/disabled/modal states;
- light/dark/limited-color terminal capability where supported.

Fixture output is authored from TableRock requirements. It is never derived from
reference-product screenshots, assets, colors, geometry, or text.

## Crossterm and PTY matrix

PTY/process tests prove:

- non-TTY behavior is explicit;
- partial terminal initialization rolls back acquired modes;
- normal exit, returned error, signal, and panic restore raw mode, alternate
  screen, mouse/paste modes, line wrap, and cursor;
- restoration is idempotent and has one TermRock/Crossterm owner;
- key press/release/repeat policy, paste, focus, resize, mouse press/drag/release,
  wheel, and tiny resize map into deterministic TEA messages;
- high-rate mouse/resize/progress input cannot starve terminal outcomes;
- terminal output contains no database value or secret diagnostics.

## Persistence matrix

Local Turso tests prove:

- fresh creation and every supported migration path;
- transaction rollback and restart after interrupted migration;
- single-actor ownership, serialized commands, flush, and clean shutdown;
- offline checkpointed backup, bounded authenticated manifest, tamper
  detection, absent-target restore, and independent restored-file health;
- the dependency graph contains `turso`, never `rusqlite` or `libsql`, and does
  not enable cloud sync;
- every schema/query feature passes the pinned Turso compatibility suite;
- disk full, permission denied, read-only filesystem, corrupt database, and
  integrity-check recovery UX;
- bounded history retention and private/disabled history;
- no resolved secret, result page, pending edit, or retryable ambiguous write;
- atomic intent restoration without connection storms.

## Security matrix

Test hostile profile URLs, identifiers, SQL parameters, Redis bytes, import
files, export paths, filenames, terminal control characters, error strings,
database notices, and bridge buffers.

Required assertions:

- no string concatenation builds executable mutations;
- read-only and destructive policy cannot be bypassed by TUI/native actions;
- resolved credentials never reach stable state, logs, history, telemetry,
  diagnostics, crash reports, or UniFFI events;
- `op read` uses argument arrays, account pinning, timeout, process kill/reap,
  and captured-output redaction;
- exports use atomic destination policy and imports remain bounded;
- SSH host keys fail closed, authentication secrets stay referenced/redacted,
  and tunnel loss cannot trigger ambiguous-write retry;
- unsupported/unknown operations fail closed;
- automatic reconnect never repeats an ambiguous write.

## UniFFI matrix

The synchronous bridge must pass:

- Swift 6 strict-concurrency Release build;
- one explicit engine/runtime owner and idempotent handle destruction;
- repeated create/open/execute/fetch/cancel/close/free stress;
- operation-ID cancellation independent of Swift task lifetime;
- one bounded transfer per event/page batch and hostile length/offset rejection;
- typed safe errors and contained Rust panics;
- worker actor to `MainActor` immutable publication;
- app/window close with pending reads/writes and process shutdown;
- Instruments allocations, latency, scroll, retained object, and leak runs;
- semantic conformance with the in-process port for all three engines.

## Native UI and accessibility matrix

Verify SwiftUI lifecycle/windows/commands/settings and AppKit catalog/grid/editor
for:

- VoiceOver roles, labels, values, selection, actions, and announcements;
- complete keyboard/menu operation, focus order/restoration, and no mouse-only
  action;
- text selection, marked text/IME, undo/redo, find, completion, and paste;
- large catalog/grid scrolling, resize, appearance, contrast, reduced motion,
  and large content;
- multi-window identity, session ownership, close/quit review, and restoration;
- file panels, security-scoped lifecycle where used, pasteboard, and drag/drop;
- no presentation path bypasses Rust capability/safety state.

## Performance evidence

Measure before publishing budgets:

- cold/warm CLI start and first frame;
- connect and first catalog page;
- query submit to first row and steady stream throughput;
- resident grid navigation/scroll frame time and allocation count;
- unloaded page fetch latency and cache eviction;
- completion latency after edit/context change;
- cancellation request to observed terminal state;
- process memory with multiple tabs and maximum configured pages;
- Turso write/flush and shutdown;
- UniFFI page decode and native grid scroll.

The release threshold is recorded with hardware, OS, terminal, server, dataset,
build profile, and sample method. A regression exceeding the recorded tolerance
blocks the affected phase until explained and approved.

## Phase exit report

Every phase exit records:

- exact commit and dependency/server/platform versions;
- tests/fixtures/benchmarks executed and links to artifacts;
- supported and unsupported capabilities;
- known cancellation and partial-outcome semantics;
- security/provenance/license result;
- remaining parity-ledger blockers;
- documentation updated with the behavior.
