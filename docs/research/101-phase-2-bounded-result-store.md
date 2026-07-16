# Phase 2 Bounded Result Store

## Decision

The Rust core owns resident result pages. A caller must explicitly open a
`PageIdentity` before admitting pages. Admission never creates, revives, or
changes a result implicitly. This makes late pages after close/reconnect and
pages from unannounced future revisions structural errors rather than UI races.

Each open result fixes result ID, revision, and engine. Opening a newer revision
atomically invalidates all older pages, including pinned pages, and returns their
stable `PageKey`s for selection/viewport resynchronization. Older revisions,
foreign engines, duplicate starts, and overlapping ranges are rejected.

## Bounds and eviction

`ResultStoreLimits` requires nonzero finite result-slot, page-count, and
resident-buffer-byte limits. `ResultPage::resident_buffer_bytes` counts the
actual heap capacities owned by column metadata/text, cell offsets, null bitmap,
kind tags, truncation facts, and the value arena. Store/container allocator
overhead is separately bounded by result/page counts and remains part of later
whole-process measurement.

Admission computes every required eviction before mutation. Unpinned pages use
global least-recently-used order with stable `PageKey` tie-breaking. If pinned
pages prevent admission, the incoming page is rejected and all counters/pages
remain unchanged. Explicit close releases the slot and reports every invalidated
key. Access-counter exhaustion renormalizes resident order rather than wrapping.

No page value, column name, or type text appears in store debug/error output.
The store performs no I/O and owns no driver type.

## Evidence

Public seam tests prove finite limits; explicit opening; unopened, stale,
future, and engine-mismatch rejection; deterministic cross-result LRU; pinning
and transactional failure; revision replacement; duplicate/overlap rejection;
result-slot limits; exact buffer-byte rejection/accounting; close invalidation;
and redacted debug output.

External concepts: bounded page cache, LRU eviction, revision invalidation
Public sources: TableRock architecture decisions `10`, `14`, `31`, and delivery/quality decisions `30`/`32`
Implementation source: TableRock-owned core contract and tests
Copied code/assets/text: none
