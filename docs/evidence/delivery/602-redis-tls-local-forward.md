# Redis TLS local endpoint forward

Date: 2026-07-22

## Failure class

Velnor run 29855858450 passed 36 of 43 initial real-server tests, including all
ClickHouse, PostgreSQL, and plaintext Redis cases. Its first Redis TLS
replacement case could reach Docker only through a non-loopback host, while
the intentionally narrow test certificate names `127.0.0.1`.

## Correction

Redis TLS container tests now expose a test-owned loopback endpoint. When
Docker is local, it uses the mapped port directly. When Docker is remote, a
bounded Tokio accept task forwards loopback TCP connections to Testcontainers'
authoritative host and port. A `JoinSet` owns active copies, and dropping the
endpoint aborts the forwarding task and its children.

This preserves strict CA and hostname verification. No insecure TLS option,
certificate widening, or production transport exception is introduced.
Fixed-port restart/recredential tests keep one forward across replacement
containers; ordinary TLS matrices receive an ephemeral local port.

## Verification

- Redis real-test binary compiled.
- TLS ACL/authentication matrix passed locally across Redis 7.4/8.8, RESP2/3,
  and optional client identity.
- Full hostile replacement matrix remains hosted-verification pending after
  push; one long local rerun encountered its existing later-iteration startup
  timeout after 323 seconds and is not claimed green.
- `cargo fmt --all -- --check`

## Provenance

No external product reference influenced this test transport. It preserves
the existing TableRock TLS contract while using Testcontainers 0.27.3's
reported remote endpoint.
