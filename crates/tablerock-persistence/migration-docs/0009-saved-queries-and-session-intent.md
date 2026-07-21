# 0009 saved queries and session intent

Adds engine-scoped named queries and one bounded intent-only JSON document per
profile. Session intent stores context, tab identity, titles, and optional SQL;
it has no result-page, cell-value, operation, credential, or pending-write
field. Names are unique within an engine.
