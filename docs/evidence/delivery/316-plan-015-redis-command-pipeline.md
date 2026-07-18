# Plan 015 residual — Redis command editor sequential pipeline

Date: 2026-07-18

## Checkpoint

| Item | Status |
|------|--------|
| `RedisPipelineCommand` / `RedisPipelineOutcome` | done |
| `RedisSession::execute_pipeline` sequential, no MULTI/EXEC | done |
| Blocking line fails alone; later lines still run | done |
| `DriverSession::redis_execute_pipeline` | done |
| Effect `ExecuteRedisPipeline` + CLI handler | done |
| EngineMsg `RedisPipelineDone` / `RedisPipelineFailed` | done |
| Run on Redis workbench → pipeline (not SQL) | done |
| Editor title "Redis"; `#` comments skipped; max 64 lines | done |
| Result sections + inspector per-command ok/err | done |
| Docker: SET/GET/BLPOP/INCR partial failure | done |
| Unit: emit effect; deny blocking pre-effect | done |

## Decision

Pipeline is sequential non-transactional dispatch over the shared Redis
session. Each line is one tokenized command; outcomes are independent.
Blocking names are denied on the shared session (InvalidMutation → line
failure) so later lines still execute — never MULTI/EXEC rollback language.
Detail text is length-bounded; arg payloads are not logged in summaries.

## Evidence

```text
cargo test -p tablerock-tui --lib redis_run_pipeline
cargo test -p tablerock-tui --lib redis_pipeline_done
cargo test -p tablerock-engine --test redis_real executes_sequential_pipeline_without_multi_exec
```

## Remaining work

- Official Redis command-metadata completion table (license provenance gate)
- Disposable-connection isolation for intentional blocking ops
- Pub/Sub UI (post-parity)
