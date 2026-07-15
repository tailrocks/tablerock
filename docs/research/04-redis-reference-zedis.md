# Redis Reference Analysis: Zedis

Zedis is useful because it treats Redis as a typed keyspace rather than forcing
it into a relational table. It is a concepts-only reference under the policy in
[01-clean-room-reference.md](01-clean-room-reference.md).

## Publicly useful concepts

The [public feature tour](https://github.com/vicanso/zedis/blob/main/docs/FEATURES.md)
supports these problem statements:

- cursor-driven discovery for large keyspaces;
- namespace-like grouping from key separators;
- type-specific value inspection;
- visible TTL and server context;
- a raw command workbench;
- safety escalation for production/read-only connections;
- current Redis status/observability.

Performance claims, GPU architecture, exact layouts, colors, icons, wording,
and key bindings are not TableRock requirements.

## Independent terminal translation

Wide terminals show a key browser beside a selected type view; narrow terminals
use Keys, Value, Commands, and Overview screens. Shared Tailrocks components
supply panels/focus/actions, while all Redis state remains TableRock-local.

Decoded keys may be grouped by a configurable separator, but projected folders
are labeled as grouping views. Binary/undecodable keys remain accessible.
Changing the separator only rebuilds presentation state.

Discovery reports cursor progress such as “scanned N,” not a false percentage.
Missing, expired, or type-changed keys become recoverable stale states.

## First Redis slice

- standalone Redis with TLS and ACL username/password;
- logical database selection;
- SCAN browser and namespace projection;
- string, hash, list, set, sorted set, and stream read views;
- TTL and bounded type/range metadata;
- current bounded INFO overview;
- command editor and safe raw result projection;
- string/hash/set/sorted-set/TTL writes, explicit list operations, streams
  read-only;
- destructive/unknown command confirmation.

## Deliberately deferred

- cluster, Sentinel, and module-specific breadth;
- SSH/cloud tunnels;
- historical metrics and charts;
- memory-analysis scans, Slow Log/latency correlation, MONITOR;
- client killing/config administration and cross-server tools;
- AI commands or natural-language queries.

Each deferred item needs an official Redis contract, resource/safety budget,
direct use case, and measured implementation plan. The goal is not to reproduce
Zedis.
