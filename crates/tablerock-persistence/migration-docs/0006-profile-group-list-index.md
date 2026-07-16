# 0006 — Profile group list index

## Before

Unfiltered/favorite and engine-scoped pages had exact ordering indexes.
Selecting one group still inspected rows outside that group before applying the
bounded page limit.

## After

`saved_profiles_group_bounded_list` prefixes the canonical stable keyset order
with the validated exact group label. Group requests seek within one group
range. Exact tag filtering uses the existing
`saved_profile_tags_lookup(tag, profile_id)` index and retains the canonical
parent ordering index.

Group and tag values are bound parameters. Labels are core-validated and cursor
scope owns the same redacted filter values, so a continuation cannot be reused
after changing either filter.
