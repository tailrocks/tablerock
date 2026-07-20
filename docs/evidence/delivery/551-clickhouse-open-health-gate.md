# 551 — ClickHouse open health gate

Date: 2026-07-21

## Failure and repair

Real-server run `29775391243` showed that the ClickHouse HTTP client constructor
is lazy: bridge `open` returned a session before any server exchange, then the
first probe failed terminally. PostgreSQL and Redis already perform network
connection work during open.

The bridge now runs the existing bounded ClickHouse `SELECT 1` health check
before registering or returning a session. All three engines therefore share
one truthful open contract: unreachable/not-ready endpoints reject with the
safe `connect` code and never create a usable-looking session.

## Verification

The facade unreachable-endpoint test now covers PostgreSQL, ClickHouse, and
Redis. The pushed three-engine container run is authoritative for server-ready
sequencing.

No external product influenced this connection-lifecycle repair.
