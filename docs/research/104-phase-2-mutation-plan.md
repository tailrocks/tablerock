# Phase 2 Typed Mutation Plan

## Decision

Rust owns the immutable executable mutation structure. Each plan carries an
opaque identity, operation scope, revision, typed target, exact typed changes,
finite limits, and a database-truthful execution model. Display preview text is
never executable input.

Targets are PostgreSQL relations, ClickHouse tables, or Redis logical-database
keys. Relational changes are typed inserts, updates, and deletes. The current
Redis foundation supports binary-safe string replacement, key deletion, and
expiration changes. Later hash, list, set, sorted-set, and stream support must
add exact variants rather than approximate them through strings.

## Value and bound safety

Construction enforces nonzero limits for changes, fields per change, aggregate
text bytes, aggregate value bytes, and review lifetime. It rejects empty plans,
empty field sets, duplicate fields, cross-engine changes, zero expiration,
mixed ClickHouse insert/mutation work, and unsafe target state.

`Invalid`, `Unknown`, truncated, and `Structured` values are inspectable but
not executable. Null assignments remain valid; null locators do not. One
`OwnedValue::encoded_byte_len` implementation supplies accounting to mutation
plans and every engine adapter, removing competing local estimators.

## Outcome truth

- PostgreSQL executes an atomic transaction.
- ClickHouse inserts are progressive and non-transactional.
- ClickHouse updates/deletes are asynchronous mutations and non-transactional.
- Redis commands are sequential and make no rollback claim.

## Review and authorization

Review consumes the exact plan and returns a non-cloneable reviewed wrapper.
Authorization consumes that wrapper and fails closed on time, operation-scope,
or revision drift. Editing the plan necessarily creates a new review.

The core now supplies a bounded Rust-owned single-use token registry. It purges
expired entries, rejects duplicate tokens and capacity overflow, supports
explicit revocation, and removes authority before validating an authorization
attempt. The future service and UniFFI boundaries must expose this owner.
Serialized plan bytes are copyable and therefore do not preserve the
in-process move-only guarantee.

## Evidence

`tablerock-core/tests/mutation.rs` proves target/change compatibility, bounded
accounting, executable-value restrictions, ClickHouse model separation, Redis
binary/expiration semantics, review expiry and scope checks, and diagnostic
redaction.

Primary behavior sources:

- [PostgreSQL data manipulation](https://www.postgresql.org/docs/current/dml.html)
- [ClickHouse UPDATE mutations](https://clickhouse.com/docs/managing-data/update_mutations)
- [ClickHouse INSERT INTO](https://clickhouse.com/docs/sql-reference/statements/insert-into)
- [Redis SET](https://redis.io/docs/latest/commands/set/)
- [Redis EXPIRE](https://redis.io/docs/latest/commands/expire/)
