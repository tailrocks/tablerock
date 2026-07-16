# 0004 — Profile list index

## Before

Saved profiles had an organization index ordered by favorite, saved order,
name, and ID. That shape could support name ordering, but it could not directly
serve the selected stable keyset cursor `(favorite, saved_order, profile_id)`.

## After

`saved_profiles_bounded_list` exactly matches the canonical profile-list order:
favorites first, then explicit saved order, then opaque stable ID. The
persistence adapter can seek after a cursor and fetch at most the requested
limit plus one lookahead row without offset scans or loading secret payload
columns.

The earlier index remains because future name/search projections have a
different access shape. This is not a compatibility path; each index serves a
distinct documented query.
