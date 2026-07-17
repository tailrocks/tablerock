# Phase 2 Redis TLS and Authentication Evidence

Date: 2026-07-17

## Decision

Redis endpoint facts remain separate from transient security material.
`RedisConnectionSecurity` borrows optional ACL credentials and custom TLS
material only for connection construction. Debug projections expose presence
and credential byte lengths plus TLS trust/identity presence, never usernames,
passwords, certificates, private keys, or server text. Credentials are limited
to 4 KiB each. Each CA bundle,
certificate chain, and private key is limited to 64 KiB before allocation by
redis-rs.

`RedisTlsMode::Require` constructs only `TcpTls` addresses with hostname
verification enabled. TableRock exposes no insecure-certificate or insecure-
hostname mode and never falls back to plaintext. Platform/custom trust roots
and optional certificate/key identity are independent typed choices, then
supplied atomically through redis-rs
1.4.0 `TlsCertificates`; absent custom roots use the platform verifier selected
by the same rustls client feature.

ACL username/password facts are written through `RedisConnectionInfo` setters,
not interpolated into a URL. One bounded unmanaged handshake runs before the
reconnecting managers are created. This makes invalid credentials a terminal
`Authentication` fact without redis-rs's generic reconnect loop repeatedly
trying them. Only a successfully authenticated initial client enters
future-call reconnect policy. TLS construction failures are `TlsConfiguration`; handshake
trust/name failures are redacted connectivity failures.

## Evidence

Testcontainers Rust 0.27.3 runs immutable official Redis 7.4.9 and 8.8.0
images. For both RESP2 and RESP3, separate server-only TLS and required-mTLS
fixtures prove:

- custom-CA verification and ACL username/password authentication succeed;
- the mTLS fixture requires the generated client certificate and private key;
- a wrong password returns `Authentication` in under one second without a
  reconnect retry loop;
- an unrelated CA and a hostname mismatch fail closed;
- plaintext against the TLS-only endpoint cannot connect or trigger fallback;
- killed TLS-authenticated connections restore credentials, trust roots, client
  identity, protocol, and logical database on future-call reconnect;
- blocking cancellation crosses the retained TLS/auth control connection and
  remains server-confirmed;
- security Debug output excludes every synthetic secret marker; and
- empty/oversized credentials, oversized TLS material, and TLS material paired
  with disabled TLS fail before network I/O.

The real suite uses generated, ephemeral rcgen material and synthetic ACL
credentials only. No production credential, endpoint, captured certificate, or
server diagnostic is stored. Private-key and ACL fixture files are mode 0600,
owned by the unprivileged Redis process before startup. The full Redis real-server suite and object-safe
adapter error mapping remain green.

This closes the Phase 2 Redis TLS/custom-root/mTLS/ACL-authentication tracer. It
does not close the product Test Connection row: profile secret resolution,
server identity/version and elapsed-time projection, live credential
revocation/restart authentication-stop behavior, platform-root server
fixtures, SSH composition, UI/native presentation, DNS/server-restart races,
Pub/Sub, write ambiguity, and clean-machine release evidence remain open.

Context7 selected the official `/redis-rs/redis-rs` documentation. Exact API
behavior was cross-checked against the pinned redis-rs 1.4.0 source.

## Provenance

External concept: verified Redis TLS and ACL authentication
Public sources: <https://docs.rs/redis/1.4.0/redis/struct.Client.html>,
<https://docs.rs/redis/1.4.0/redis/struct.TlsCertificates.html>, and
<https://redis.io/docs/latest/operate/oss_and_stack/management/security/encryption/>
TableRock requirements: research 03, 06, 10, 14, 20, 30, 31, 32, 53, 54, 55,
and 90
Implementation source: TableRock-owned transient security contracts and
independent generated Testcontainers fixtures
Copied code/assets/text: none
