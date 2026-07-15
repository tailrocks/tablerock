# Open Decisions

## Fixed

| Decision | Outcome |
|---|---|
| Product | standalone TableRock repository |
| Scope | PostgreSQL, ClickHouse, Redis only |
| First UI | Rust CLI/TUI using Tailrocks TUI |
| ClickHouse | official `ClickHouse/clickhouse-rs` |
| Redis | `redis-rs/redis-rs` |
| Credentials | 1Password item mapping preferred |
| Plaintext | dangerous local-test fallback |
| Native | SwiftUI/AppKit over Rust/service contracts |
| Tunnels and AI | excluded from first program |
| References | concepts only, no copied expression |

## 1. Name/legal clearance

**Default:** TableRock remains the working/product repository name. Complete
trademark/category/domain/App Store/package review before release.

**Deadline:** before public product announcement/package publication.

## 2. Profile scope

**Default:** global profile definitions with stable IDs plus optional per-project
visibility/order bindings. Bindings are convenience, not agent authorization.

**Deadline:** profile schema PR.

## 3. Secret abstraction

**Default:** TableRock-local OpRef/SecretResolver and `op` CLI adapter. Do not
extract a shared secrets crate without a second consumer and security review.

**Deadline:** profile phase.

## 4. Embedded persistence

**Default:** select one adapter through measured Turso-versus-focused-SQLite
spike for startup, migration, FTS/history, concurrency, package size, platforms,
maintenance, and license.

**Deadline:** Phase 0.

## 5. Redis deployment

**Default:** use the selected `redis-rs/redis-rs` client. Standalone Redis first;
Valkey only as tested compatibility; cluster/Sentinel deferred. The integration
spike still has to prove binary/SCAN/logical DB/TLS/reconnect/cancel contracts.

**Deadline:** Phase 0 Redis spike.

## 6. Server support floors

**Default:** oldest real project version plus latest stable, and promise only the
pinned CI matrix.

**Deadline:** candidates Phase 0; publish Phase 6.

## 7. Result budgets

**Starting measurements:** pages around 500 rows, arbitrary query cap around
10,000 rows, independent byte/memory caps. No unlimited first-program mode.
These are not defaults until measured across all engines.

**Deadline:** preliminary Phase 0; validate before Phase 2 ships.

## 8. ClickHouse result format and writes

**Default:** correctness with official `fetch_bytes()` self-describing lines,
then measure RowBinaryWithNamesAndTypes. Read-only then batch INSERT; mutations
only after query/mutation identity and status are proven. No premature Arrow.

**Deadline:** format Phase 0; write scope Phase 4.

## 9. Redis writes

**Default:** string/hash/set/sorted-set/TTL, explicit list operations, streams
read-only. Preserve/change TTL intentionally. Modules read-only without tests.

**Deadline:** Phase 5 review.

## 10. SQL parser depth

**Default:** sqlparser tokens + last valid AST + schema index. Add tree-sitter SQL
only after dialect/generated-artifact/license audit and measured accuracy gain.

**Deadline:** Phase 3 baseline; revisit from fixtures.

## 11. Grid/editor ownership

**Default:** TableRock-local first. Promote neutral presentation primitives to
Tailrocks TUI only for a second real consumer with neutral naming, lookbook, and
no database semantics.

**Deadline:** component by component.

## 12. Daemon and native interop

**Default:** in-process terminal behind serializable contracts, daemon after
three engines stabilize and before native. Daemon RPC owns live sessions;
compare then-current UniFFI and narrow C ABI only for justified embedded work.

**Deadline:** after Phase 6.

## 13. Safety and future automation

**Default:** Confirm writes; encourage Read only for production; destructive
operations always require specific confirmation. No agent/AI/MCP access in this
program. Any future automation needs separate least-privilege, approval, limits,
audit, secret isolation, prompt-injection, and destructive-operation design.

**Deadline:** safety in profile phase; automation has no deadline.
