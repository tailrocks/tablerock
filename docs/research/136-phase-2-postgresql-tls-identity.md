# Phase 2 PostgreSQL TLS And Client Identity Evidence

Date: 2026-07-17

## Decision

The private PostgreSQL adapter now has one explicit TLS binary: `Disabled` uses
`tokio_postgres::NoTls`; `Required` requires a rustls handshake. The former
`Prefer` state was removed because it could silently downgrade to plaintext and
has no corresponding state in the approved core `TlsPolicy` contract. No
backward-compatibility shim or alternate connector remains.

Required TLS can use native system roots through
`MakeRustlsConnect::with_native_certs` or caller-supplied bounded PEM material.
Custom material accepts 1–16 CA certificates, an optional 1–8 certificate
client chain, and exactly one unencrypted private key. Each PEM input is capped
at 65,536 bytes before parsing. Empty, malformed, excessive, incomplete, and
multi-key inputs return the message-free `TlsConfiguration` category. Parsed
keys from a rejected multi-key input are explicitly zeroized.

TableRock directly pins latest stable rustls 0.23.42 with only `ring`, `std`,
and `tls12`. It constructs a deterministic ring provider without installing
process-global crypto state. Latest stable rcgen 0.14.8 is development-only and
generates disposable CA/server/client material in memory for real-server tests;
its `zeroize` feature is enabled. Context7 was attempted first and reported its
monthly quota exhausted, so APIs and versions were verified from Cargo registry
metadata and the exact pinned official sources.

`TlsServerName` is independent from the TCP endpoint. A private connector
wrapper replaces only rustls verification/SNI input while `tokio-postgres`
still connects to the configured host and port. This supports tunnels and
mapped test ports without weakening hostname verification. Configuration and
TLS-material Debug output exposes only lengths, booleans, port, and mode.

The session retains the exact connector used at connect time. Every PostgreSQL
cancel request clones that connector, preserving the same roots, server name,
and client identity. Cancellation can neither fall back to native roots nor
lose mTLS identity.

## Real-server matrix

Testcontainers Rust 0.27.3 starts official `postgres:17.10-alpine` and
`postgres:18.4-alpine`. Each container receives freshly generated material,
installs owner-only server keys during initialization, enables TLS 1.2+, and
uses ordered `pg_hba.conf` rules for a root-verified TLS role and a
certificate-authenticated `postgres` role.

Both supported lines prove:

- verified custom CA and independent `database.internal` server-name/SNI;
- bounded query streaming after the verified handshake;
- plaintext rejection rather than TLS downgrade;
- hostname mismatch and wrong-root rejection;
- missing-client-certificate rejection for the mTLS role;
- duplicate-private-key rejection before network dispatch;
- successful client certificate authentication; and
- SQLSTATE-confirmed cancellation over a new connection using the identical
  custom-root, server-name, and client-identity connector.

Fixtures contain no committed private key, password, production endpoint, or
captured database value. Certificate and key bytes never enter public errors,
adapter diagnostics, Debug output, core contracts, or default logs.

## Remaining PostgreSQL Phase 2 gates

This closes verified roots, client identity, server-name override, plaintext
downgrade prevention, and TLS cancellation across both supported PostgreSQL
lines. Encrypted private-key handling, authentication taxonomy, parameters,
notices, multiple statements, COPY, connection
loss/reconnect, and ambiguous-write evidence remain open. Native-system-root
loading is implemented but is not represented as a disposable custom-root
fixture claim.
Cancellation completion races are subsequently closed by research 155.

## Provenance

External concepts: PostgreSQL TLS, certificate authentication, cancel requests
Public sources: <https://docs.rs/tokio-postgres/0.7.18>,
<https://docs.rs/tokio-postgres-rustls/0.14.0>,
<https://docs.rs/rustls/0.23.42>, <https://docs.rs/rcgen/0.14.8>,
<https://www.postgresql.org/docs/current/ssl-tcp.html>, and
<https://www.postgresql.org/docs/current/auth-cert.html>
TableRock requirements: research 03, 10, 20, 30, 31, and 32
Implementation source: TableRock-owned connector wrapper, bounded parser, and
ephemeral real-server fixtures
Copied code/assets/text: none
