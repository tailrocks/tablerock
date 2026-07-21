# Structured JSON number identity

Date: 2026-07-21

## Failure

Hosted native checkpoint runs 29839995931 and 29841286500 both failed in
`Run Swift bridge and feature tests`. Local direct Foundation probing reproduced
the architectural cause: JSONSerialization returns JSON numbers as NSNumber,
and NSNumber values `0` and `1` also satisfy Swift's `as Bool` cast. A
type-cast-ordered renderer therefore changed valid JSON numbers into booleans.

## Structural correction

`StructuredValueTree` now handles the shared NSNumber representation once and
uses CoreFoundation type identity to distinguish the CFBoolean singleton type
from numeric NSNumber values. This removes the ambiguous cast ordering rather
than special-casing textual `0` or `1`.

The deterministic tree test now covers numeric zero and one beside boolean
false and true in one object. It requires exact labels, values, and depth.

## Verification

- Direct local Foundation probe proves NSNumber 1 casts to Bool on the current
  macOS toolchain, explaining the failed expectation.
- `git diff --check` passes.
- Full XCTest rerun is pending on the hosted Xcode 26.6 checkpoint after push;
  the local host has Command Line Tools only and cannot import XCTest.

## Provenance

No external product reference influenced this correctness repair. The behavior
comes from TableRock-owned tests and direct Foundation runtime evidence.
