# Persistence migration index

Apply migrations transactionally in numeric order. Never edit an applied
migration; add the next zero-padded SQL file and matching explanation.

| Sequence | Migration |
|---:|---|
| 0001 | [Bootstrap ledger](migration-docs/0001-bootstrap.md) |
| 0002 | [Support facts](migration-docs/0002-support-facts.md) |
| 0003 | [Saved profiles](migration-docs/0003-saved-profiles.md) |
