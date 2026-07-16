# 0003 — Saved profiles

## Before

The database contained only the migration ledger and metadata-only support
facts. No connection profile could be persisted.

## After

`saved_profiles` stores versioned identity, revision, engine, bounded connection
policy, organization, and preferences. `saved_profile_tags` preserves validated
tag order. `saved_profile_properties` preserves each property's source kind and
source-specific fields without flattening references into resolved values.

Only the persistence actor's saved-profile token command may insert these rows.
Temporary profiles have no such token. One transaction inserts the aggregate,
its ordered tags, and its ordered properties. Migration `0003` and its ledger
row are separately committed together by the migration runner.

The schema uses 16-byte opaque IDs and 8-byte big-endian unsigned revisions so
the full core identity/revision domains round-trip without signed narrowing.
Organization indexes support bounded future list/filter queries.
