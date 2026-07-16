# 0001 — Bootstrap ledger

## Before

No TableRock persistence file or schema existed.

## After

`schema_migrations` records each applied numeric migration exactly once with an
informational database-generated timestamp. The actor applies this bootstrap
in an immediate transaction only when no ledger table exists. An existing
ledger is validated as a contiguous supported prefix before any mutation.
