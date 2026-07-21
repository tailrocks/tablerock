# 0018 retire support facts

Phase 15 support bundles accept only closed Rust-owned diagnostic and operation
outcome types. The unused `support_facts(fact_key TEXT, fact_value TEXT)` table
could persist arbitrary messages, SQL, values, paths, endpoints, or credentials
if accidentally adopted.

This forward migration drops that dormant table. It had no production reader or
writer, so no supported user state is removed. Future durable support retention
requires a new typed schema with closed numeric discriminants and explicit
bounds; migration 0002 remains immutable history.
