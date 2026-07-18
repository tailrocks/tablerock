# Migration 0007 — environment tag

## Decision

Add optional `environment_kind` / `environment_label` columns to
`saved_profiles` without bumping `ProfileAggregate::SCHEMA_VERSION`.

Organization already lives as columns (group/tags/favorite/order), not as a
versioned wire blob. Pre-0007 rows load as `environment: None`. The aggregate
schema remains 1; `from_wire(1)` is unchanged for callers that construct
`ProfileOrganization` with the new optional field defaulting to `None`.

## Encoding

| Kind | Meaning | Label |
|---:|---|---|
| NULL | No environment | NULL |
| 1 | Production | NULL |
| 2 | Staging | NULL |
| 3 | Development | NULL |
| 4 | Testing | NULL |
| 5 | Custom | 1..=64 UTF-8 bytes |

Only `Production` is a persistent warning in the product model.
