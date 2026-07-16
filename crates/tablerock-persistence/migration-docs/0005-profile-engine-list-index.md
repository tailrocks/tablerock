# 0005 — Profile engine list index

## Before

Migration `0004` supported the unfiltered and favorite-scoped canonical list
order. Selecting one engine still required inspecting profiles from other
engines before applying the bounded page limit.

## After

`saved_profiles_engine_bounded_list` prefixes the same stable keyset order with
the closed engine discriminator. Engine-only and engine-plus-favorite requests
seek within one engine range, while unfiltered/favorite-only requests retain
the `0004` index.

The adapter builds SQL only from a closed set of trusted filter clauses and
binds every value. Cursor scope is validated in core before the query. No
dynamic user text enters SQL and no compatibility query path remains.
