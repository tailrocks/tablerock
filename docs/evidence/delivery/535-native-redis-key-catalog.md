# Native Redis typed key catalog

Date: 2026-07-19

## Contract

The shared catalog request now includes bounded Redis key listing. The Redis
adapter reuses its SCAN stream with row, cell, arena, and round limits, then
resolves TYPE for each resident key. It never uses `KEYS`. UTF-8 keys receive a
reversible `text:` identity; binary/control-byte keys receive lowercase `hex:`
identity. UniFFI strips identity framing only for display and retains opaque
catalog handles.

Only the session's selected logical database may expand. Selecting another
logical database fails explicitly and requests reconnect/context selection;
keys are never shown under a false database parent.

## Evidence

- Redis 8.8 live test: text and NUL/non-UTF-8 keys cross SCAN/TYPE into typed
  catalog nodes with exact reversible identities.
- UniFFI conformance: Redis logical database expands through opaque parent ID;
  text identity displays without framing and key nodes remain leaves.
- Engine library suite: 109 passed.
- Formatting and diff checks: pass.

## Remaining boundary

Typed Redis key-value pages/object tabs, namespace grouping, logical-database
context switching, key filtering, pagination continuation, and key mutation
screens remain.

## Provenance

TablePro established only the broad key-browser concept. No source, tests,
text, screenshots, layouts, measurements, colors, assets, or key bindings were
copied or translated. Redis behavior follows redis-rs documentation and direct
Redis 8.8 tests.
