# Phase 2 Immutable Page Contract Evidence

## Checkpoint

The third Phase 2 tracer adds the std-only immutable page language used between
future driver assemblers, the result store, the TUI service adapter, and the
native bridge encoder. A `PageEnvelope` carries result identity, aggregate
revision, engine, global row range, known or unknown total, declared column and
byte dimensions, partial/final delivery, and compact safe warning facts.
`PageEnvelope::validate` is the mandatory logical pre-allocation gate. It returns
a non-constructible `ValidatedPageEnvelope` token and rejects an
unsupported layout version, excessive row/column/arena/metadata dimensions,
row-range overflow, impossible known totals, rows without columns, and a cell
count unsupported by the target address space.

`PageBuffers` groups immutable column metadata, column-major cell offsets, a
null bitmap, value-kind tags, truncation facts, and one value byte arena.
`ResultPage::from_parts` requires that validation token and structurally
consistent owned buffers. `ResultPage::cell` returns a borrowed `CellRef`; it
does not allocate or create a retained object per cell. Pages are independently
owned and disposable. No driver row, client type, borrowed driver lifetime, UI
type, runtime handle, or database connection crosses this boundary.

## Bounds, failure, cancellation, and redaction

- Operation-owned `PageLimits` bound rows, columns, value-arena bytes, and
  column/type descriptor text bytes. Fixed decision 31 still starts table pages
  at 500 rows; Phase 2 measurements will set total metadata and process-memory
  budgets. There is no unlimited mode in the owning result store.
- The token makes envelope validation mandatory before page-buffer acceptance.
  A later decoder/bridge checkpoint must prove it validates before allocating
  vectors. The owned constructor then rejects mismatched counts and declared lengths,
  non-monotonic/out-of-arena offsets, a nonzero first offset, a wrong final
  offset, nonzero null-bitmap padding, null/kind/byte/truncation disagreement,
  invalid known truncation lengths, descriptor-text overflow, nonnullable nulls,
  noncanonical value encodings, unsupported truncation kinds, and column/page
  engine disagreement.
- Null uses zero bytes; booleans use one byte (`0` or `1`); signed, unsigned,
  and Float64 values use eight big-endian bytes; decimal and text use UTF-8;
  binary, invalid, and unknown retain arbitrary bytes. Only text, binary,
  invalid, and unknown may be truncated. Null remains distinct, consumes no
  arena bytes, obeys column nullability, and can never be truncated.
- Page delivery and warnings do not claim operation success, server-side
  cancellation, or a final database outcome. Those belong to the upcoming
  revisioned result/event and cancellation contracts.
- `ResultPage`, `PageBuffers`, `ColumnMetadata`, and validation errors expose
  only safe IDs, engines, dimensions, positions, kinds, and counts through
  `Debug`/`Display`. `CellRef` intentionally has no `Debug`; cell bytes and
  column/type text are never emitted by default diagnostics.

## Encoding boundary and remaining work

This checkpoint establishes the authoritative logical columnar layout and its
version/bounds validation. It does not yet claim the UniFFI transfer encoding.
The native bridge gate will serialize the metadata, offsets, null bitmap, kind
tags, truncation facts, and arena into one versioned `Vec<u8>`, then run hostile
Swift decoding and semantic-equivalence tests away from `MainActor`. Arrow and
per-cell bridge calls remain excluded.

Completion/failure/cancellation states, warning diagnostics, batch assembly,
eviction, resync, and process-memory accounting remain Phase 2 blockers and are
not inferred from an immutable page's presence.

## Evidence

- Public seam tests prove the accepted 500-row boundary, row-limit rejection,
  row-range overflow, unknown version rejection, known/unknown total behavior,
  partial delivery, and safe warning retention.
- A two-row/two-column fixture proves canonical big-endian signed encoding,
  column-major projection, null and empty byte ranges, text-kind preservation,
  out-of-page access rejection, and
  absence of cell text from page debug output.
- A table-driven matrix proves accepted canonical bytes for every value kind and
  rejects invalid Boolean, fixed-width numeric, Float64, decimal, and text bytes.
- Hostile fixtures prove rejection of out-of-arena offsets, null-kind mismatch,
  nonnullable nulls, invalid fixed-width/UTF-8 encoding, unsupported truncation,
  invalid truncation length, column engine mismatch, and nonzero bitmap padding.
- The core architecture test includes the page module and continues to reject
  runtime, presentation, driver, network, and clock dependencies.
- No compatibility shim or alternate row representation remains.

## Verification record

- `cargo test -p tablerock-core --test page`: 6 passed.
- `cargo clippy -p tablerock-core --all-targets --all-features --locked -- -D
  warnings`: pass.
- `cargo test --workspace --locked`: 51 passed, 3 ignored.
- Workspace format, clippy, rustdoc, `cargo deny`, `gitleaks`, English-script,
  and complete-diff gates: pass. The already-allowed `hashbrown` duplicate is
  unchanged.

External concepts: checked integer ranges, owned vectors, slices, and bitmaps
Public sources: <https://doc.rust-lang.org/std/primitive.u64.html>,
<https://doc.rust-lang.org/std/vec/struct.Vec.html>, and
<https://doc.rust-lang.org/std/primitive.slice.html>
TableRock requirements: research 10, 14, 30, 31, and 32
Implementation source: TableRock core architecture and independent tests
Copied code/assets/text: none
