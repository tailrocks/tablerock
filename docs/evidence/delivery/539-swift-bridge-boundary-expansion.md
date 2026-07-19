# Swift bridge boundary expansion

Date: 2026-07-19

## Contract

Swift independently validates the PageV1 cell-offset table before slicing the
arena: the first offset is zero, offsets are monotonic, and the final offset
equals the declared arena length. Malformed offsets now fail explicitly rather
than rendering a misleading empty cell.

The durable Swift bridge suite also covers a valid typed text page, unsupported
versions, row/column/column-text header bounds, malformed operation IDs,
post-runtime-destruction calls, and 64 create/ensure/read/destroy cycles.

## Evidence

- `DYLD_LIBRARY_PATH=../target/release swift test -c release`: 12 tests in two
  suites pass.
- Valid page coverage proves column metadata, display text, and exact cell
  bytes.
- Three hostile offset shapes prove nonzero-first, descending, and
  out-of-arena rejection.
- Lifecycle coverage proves typed `bad-operation-id`, typed
  `RuntimeUnavailable`, and repeated bridge ownership cleanup.

## Remaining boundary

Versioned cross-engine fixture resources, all value kinds/truncation tags,
offset/count arithmetic overflow, cancellation/shutdown stress, and
`BehaviorProof` conversion remain checkpoint-11 work.

## Provenance

This testing structure follows current Swift Package Manager guidance and the
operator-supplied Apple testing model. TablePro is only a broad workflow
reference; no source, tests, text, screenshots, layouts, measurements, colors,
assets, or key bindings were copied or translated.
