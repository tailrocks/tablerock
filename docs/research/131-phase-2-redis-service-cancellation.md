# Phase 2 Redis service cancellation evidence

Date: 2026-07-17

Redis blocking-command cancellation now crosses the complete
core/service/runtime/adapter path. Each owned Redis session has a dedicated
operation connection, its stable `CLIENT ID`, and a separate control
connection. A blocking `BLPOP` runs in an owned task whose one-shot result can
be awaited repeatedly without resending the command when runtime control wins a
`select!` race.

Cancellation issues this control command with integer arguments, never command
text assembled from user data:

```text
CLIENT UNBLOCK <operation-client-id> ERROR
```

Only integer reply `1` records request dispatch. Reply `0` is `ServerRejected`;
transport/protocol failure is `TransportFailed`. After reply `1`, the operation
connection must also observe the server-generated error before the terminal
outcome becomes `ServerConfirmedCancelled`. Dropping a redis-rs request future
or stopping the client is never mislabeled as server cancellation.

The blocking key is bounded binary data. Debug output records only its byte
length. One session admits one active blocking operation, and stream completion
or drop clears ownership. SQL, Redis arguments, returned values, and server
error text remain absent from stable errors and logs.

One real Testcontainers contract runs Redis 7.4.9 and 8.8.0 under RESP2 and
RESP3. It obtains the operation connection ID before ownership transfer, waits
until `CLIENT LIST ID` reports the connection with blocking flag `b`, requests
cancellation, then requires `RequestSent` and `ServerConfirmedCancelled` within
five-second deadlines for all four combinations. The same matrix also unblocks
a second `BLPOP` through `RPUSH` and verifies its bounded binary result page,
proving normal completion is not confused with cancellation.

This follows official Redis semantics: `CLIENT ID` identifies the exact
connection and is not reused; `CLIENT UNBLOCK ... ERROR` returns `1` only when
it unblocks the target client. redis-rs 1.4.0 documents that dropping a
`MultiplexedConnection` request future does not cancel the server request and
can retain a blocking connection. Context7 was requested first but unavailable
because its monthly quota was exhausted.

Remaining Redis Phase 2 proof includes TLS/authentication, SCAN-family breadth,
pipelines/partial failures, Pub/Sub isolation, timeouts, and reconnect races.
No external-product source or protected expression influenced this checkpoint.

Public sources:

- <https://redis.io/docs/latest/commands/client-id/>
- <https://redis.io/docs/latest/commands/client-unblock/>
- <https://redis.io/docs/latest/commands/blpop/>
- <https://docs.rs/redis/1.4.0/redis/aio/struct.MultiplexedConnection.html>
