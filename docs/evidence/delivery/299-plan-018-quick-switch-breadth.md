# Plan 018 residual — quick switch breadth (tabs + profiles + queries)

Date: 2026-07-18

## What landed

Ranked quick switch across:

| Screen | Candidates | Action on match |
|--------|------------|-----------------|
| Workbench | Tabs (title / 1-based index) | select tab |
| Workbench | Open saved-query panel entries | `LoadNamedQuery` |
| Connections / Picker | Profile name / target / engine | set `selected_id` |

Ranking: exact → prefix → contains (lower score wins). Tabs preferred over
saved queries when scores tie (+10 to query score).

Units:

- `quick_switch_selects_tab_by_title_substring`
- `quick_switch_ranks_profiles_on_connections`
- `quick_switch_loads_saved_query_by_name`

## Commands

```bash
cargo test -p tablerock-tui quick_switch
```
