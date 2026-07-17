# Phase 2 PostgreSQL Network Projection Evidence

## Checkpoint

The PostgreSQL adapter now projects `inet`, `cidr`, `macaddr`, and `macaddr8`
binary values into bounded canonical text without exposing client types through
core contracts.

## Decision

`inet` and `cidr` validate the complete PostgreSQL binary envelope: address
family, prefix length, type-specific CIDR flag, address length, payload length,
and trailing-byte absence. IPv4 and IPv6 use deterministic lowercase standard
address formatting. `inet` omits only a full host prefix (`/32` or `/128`);
`cidr` always retains its prefix and rejects nonzero host bits.

`macaddr` and `macaddr8` require exactly six and eight wire octets respectively
and format lowercase two-digit octets separated by colons. Successful values
use the shared `Text` contract. Malformed envelopes become `Invalid` with exact
PostgreSQL type identity.

## Bounds and failure truth

- every canonical value is bounded by the caller cell limit;
- truncation records the canonical output's original byte length;
- unsupported address families, invalid prefix lengths, mismatched CIDR flags,
  wrong address lengths, non-network CIDR payloads, and trailing bytes become
  `Invalid`;
- wrong MAC payload lengths become `Invalid`;
- raw network values and canonical cell text are never logged.

## Evidence

Unit tests cover IPv4/IPv6, `inet`/`cidr`, 48/64-bit MAC addresses, canonical
forms, bounded output, invalid families/masks/flags/lengths, non-network CIDR,
trailing bytes, and exact failure identity. Testcontainers Rust 0.27.3 owns
official `postgres:17.10-alpine` and `postgres:18.4-alpine` fixtures. Both lines
prove host and network IPv4/IPv6 values plus `macaddr` and `macaddr8`, with exact
column type identity and complete canonical `Text` values.

## Remaining work

Additional scalar families remain separate typed-value work. Network editors
and schema metadata, public parameter plans and request bounds, service/UI
projection, and UniFFI remain open. The PostgreSQL driver still receives a
complete field before TableRock applies its cell bound; strict pre-driver field
allocation remains open.

Context7 was attempted first and reported its monthly quota exhausted. Current
behavior was therefore checked against PostgreSQL 18 primary network-type docs
and `REL_18_STABLE` send implementations plus pinned `postgres-protocol` 0.6.12
and `postgres-types` 0.2.14 source.

External concepts: PostgreSQL network binary envelopes and canonical address output
Public sources: <https://www.postgresql.org/docs/current/datatype-net-types.html>, <https://github.com/postgres/postgres/blob/REL_18_STABLE/src/backend/utils/adt/network.c>, <https://github.com/postgres/postgres/blob/REL_18_STABLE/src/backend/utils/adt/mac.c>, <https://github.com/postgres/postgres/blob/REL_18_STABLE/src/backend/utils/adt/mac8.c>, <https://docs.rs/postgres-protocol/0.6.12/postgres_protocol/types/fn.inet_from_sql.html>
Implementation source: TableRock-owned adapter and independent Testcontainers fixtures
Copied code/assets/text: none
