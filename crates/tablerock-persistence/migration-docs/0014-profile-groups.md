# 0014 profile groups

Adds durable empty profile groups as a strict table, then backfills every
distinct non-null group already referenced by saved profiles. `INSERT OR IGNORE`
makes duplicate legacy membership converge without losing profiles.
