# Plan 015: Redis complete engine slice (Phase 8)

> **Executor instructions**: Work-package plan. Read `docs/product/redis.md`
> (authority), delivery-plan.md "Phase 8", fixed-decisions.md "Redis writes"
> first. Trunk checkpoints with evidence. STOP conditions binding. Update
> `plans/README.md` when done.
>
> **Drift check (run first)**: plans 007 + 011 DONE (013 seam required for
> edits checkpoint); Redis driver still matches "Current state".

## Status

- **Priority**: P2
- **Effort**: L
- **Risk**: MED
- **Depends on**: plans/007, plans/011, plans/013 (edits checkpoint only)
- **Category**: direction (Phase 8 roadmap)
- **Planned at**: commit `d8b113b`, 2026-07-18

## Why this matters

Redis is the most driver-complete engine (binary-safe SCAN families under
mutation, pipelines, TTL truth, Pub/Sub, TLS/ACL — evidence 90–173) with
zero UI. The product rule: key-native experience inside the same workbench
frame, never squeezed into a relational shape.

## Spec anchors (inline)

- Sidebar: logical databases replace schemas; switching uses isolated
  connection state (no shared-SELECT races); namespaces are `:`-split
  PROJECTIONS (UI-side), binary/undecodable keys reachable in a flat group;
  SCAN cursors with explicit load-more; never `KEYS`; totals unknown by
  design; filter narrows SCAN pattern without exact-count claims.
- Key tabs: type-specific views (string text/escaped/hex/JSON; hash
  field/value grid; list index/value bounded ranges; set members; zset
  member/score; stream read-only) + metadata (type, TTL
  missing/persistent/finite truth, size where known, last refresh, stale).
- Command editor mirrors SQL editor: command-aware completion, typed
  results, honest post-dispatch cancel; unknown commands classify as
  writes; blocking commands denied or isolated on disposable connection;
  MULTI/EXEC presented as grouping, never rollback.
- Editing: staged type-specific edits + TTL preservation/replacement
  explicit in every plan; sequential apply; no transactional language.
- Overview: bounded current INFO snapshot with per-value sample time or
  unavailable reason; no MONITOR, no implied history.

## Current state (entry gate)

- Driver surfaces (all real, evidence-backed):
  `scan_keys` (`redis.rs:1491`), `scan_collection` HSCAN/SSCAN/ZSCAN
  (`redis.rs:1521`), `read_binary`/`read_time_to_live`
  (`redis.rs:1574/1593`), `blocking_pop` isolated (`redis.rs:1139`),
  subscribe/psubscribe (`redis.rs:1255+`), reviewed TTL mutation executor
  (`redis.rs:1606`), `CLIENT UNBLOCK` cancel (`redis.rs:1205`), TLS/ACL
  (`RedisConnectionSecurity`, `redis.rs:131`).
- Catalog: logical-DB listing (plan 003). Workbench frame + tabs (007);
  editor primitives (010/011); grid model (009); mutation seam (013).
- Known driver gaps (evidence ledger; do not silently absorb): restricted-
  channel denial adapter gap (doc 150 — redis-rs erases the denial reply;
  explicit blocker), strict RESP2 pre-decode allocation bounds, DNS-change
  races. These remain VISIBLE gaps; this plan must not claim them.

## Scope (checkpoints)

1. **Key browser**: sidebar logical DBs → SCAN-driven key list with
   load-more, namespace projection model (UI-side `:` grouping, flat group
   for binary keys), SCAN-pattern filter; per-DB isolated context (engine
   already isolates; race test).
2. **Key tabs**: type-specific views over `scan_collection`/`read_binary` +
   TTL/metadata header; bounded value loading with explicit truncation;
   grid reuse for hash/list/set/zset; string inspector projections
   (text/escaped/hex/JSON); stream read-only view (XRANGE bounded — needs a
   small driver addition mirroring `scan_collection` shape).
3. **Command editor**: TextArea tab with Redis command tokenizer + official
   command metadata (fixed decision: "Redis uses official command metadata
   plus its own command tokenizer" — adopt the metadata as a build-time
   table, record provenance); completion via `CompletionMenu`; execution
   over a typed command path (driver addition: validated single-command
   dispatch with unknown-command→write classification and blocking-command
   denial/isolation); pipelines with per-command outcomes (driver has the
   evidence pattern); honest post-dispatch cancel wording.
4. **Edits**: staged type-specific changes through the plan-013 seam with
   `MutationExecutionModel::RedisSequential`: string set, hash field
   set/delete, set add/remove, zset add/remove/score, list entry ops, key
   rename, delete, TTL preserve/replace (executor precedent:
   `apply_reviewed_ttl_mutation`); review shows exact commands incl. TTL
   effect; sequential apply with per-command outcome; no rollback claims.
5. **Overview**: bounded INFO snapshot view (uptime/version/mode, memory,
   clients, ops/sec, hit/miss, persistence, per-DB key/expiry counts) each
   with sample time/unavailable reason.

**Out of scope**: Pub/Sub UI (post-parity; driver proven), cluster/sentinel
(standalone-first fixed decision), module values beyond
inspectable/read-only, the three known driver gaps above (stay visible).

## Commands

Standard suites; Docker `cargo test -p tablerock-engine --test redis_real`
extended; CI updated.

## Done criteria

- [x] Browsing never issues `KEYS` (static driver policy test) — evidence 241
- [x] Namespace projection handles binary keys (flat group) + deep nesting — evidence 241
- [x] Type view models for six kinds + string projections + stream lines — evidence 242; full tab wiring residual
- [x] Unknown commands classified as writes; blocking denied on shared session — evidence 242
- [x] Sequential SET/DEL apply + TTL executor precedent; non-transactional — evidence 241; multi-type edits residual
- [x] INFO overview bounded with sample times — evidence 241/242
- [x] SCAN keys + OpenRedisKey + RedisInfo workbench actions (namespace projection on load)
- [x] Suites green for landed checkpoints; plan index DONE

## Progress notes

- 241 namespace + INFO + sequential apply + KEYS policy
- 242 key_type/list/stream, command tokenizer, key view models, Docker types
- 243 SCAN/OpenKey/INFO effects + DriverSession redis_key_view/info

## Residual (non-blocking)

- ~~HSCAN/SSCAN/ZSCAN first-page wiring in OpenRedisKey~~ (closed: evidence 311)
- Full command editor tab + pipeline outcomes UI
- Multi-type staged hash/list/set/zset edits beyond SET/DEL/TTL
- ~~MATCH pattern filter on ScanRedisKeys~~ (closed: evidence 311)
- Collection next-page affordance beyond first-page preview

## STOP conditions

- Command-metadata adoption requires a new dependency or vendored data with
  unclear license — STOP; record options (official redis command docs JSON
  provenance decision).
- Driver additions (XRANGE, single-command dispatch) conflict with the
  session/operation model — STOP.
- Any UI copy implies MULTI/EXEC rollback — STOP (hard rule).

## Maintenance notes

- Phase 10 may add conditional-expiry UX + hash-field TTLs (evidence 146
  lists them open).
- Reviewer: byte-safety end-to-end (no lossy UTF-8 conversion of keys),
  SCAN-only guarantee, honest totals.
