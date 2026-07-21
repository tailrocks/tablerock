# PageV1 pre-loop cardinality validation

Date: 2026-07-21

## Failure

Hosted native run 29841286500 reached the hostile row/column product fixture
but returned `truncated` instead of `sizeOverflow`. The arithmetic check
existed, yet it ran after the decoder's `column_count`-controlled metadata
loop. With widened diagnostic limits, the loop consumed the short hostile
fixture before the product could be checked.

## Structural correction

The full PageV1 decoder now derives and validates cell count, offset count, and
bitmap numerator immediately after the fixed header and before every
attacker-controlled body loop. Validation order now matches the
allocation/iteration threat model.

The existing hostile fixture directly requires `sizeOverflow` for maximum
32-bit row and column counts under widened individual limits.

## Verification

- The hosted failure log identifies
  `testHostileRepresentationalOverflowFailsClosed` and the exact
  `truncated`/`sizeOverflow` mismatch.
- `git diff --check` passes.
- Hosted Xcode 26.6 rerun is pending after push.

## Provenance

No external product reference influenced this safety repair. The cause and
correction come from TableRock's wire contract, hostile test, and hosted log.
