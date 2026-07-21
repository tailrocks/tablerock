# Persistence migration index

Apply migrations transactionally in numeric order. Never edit an applied
migration; add the next zero-padded SQL file and matching explanation.

| Sequence | Migration |
|---:|---|
| 0001 | [Bootstrap ledger](migration-docs/0001-bootstrap.md) |
| 0002 | [Support facts](migration-docs/0002-support-facts.md) |
| 0003 | [Saved profiles](migration-docs/0003-saved-profiles.md) |
| 0004 | [Profile list index](migration-docs/0004-profile-list-index.md) |
| 0005 | [Profile engine list index](migration-docs/0005-profile-engine-list-index.md) |
| 0006 | [Profile group list index](migration-docs/0006-profile-group-list-index.md) |
| 0007 | [Environment tag](migration-docs/0007-environment-tag.md) |
| 0008 | [Query history](migration-docs/0008-query-history.md) |
| 0009 | [Saved queries and session intent](migration-docs/0009-saved-queries-and-session-intent.md) |
| 0010 | [Column layout](migration-docs/0010-column-layout.md) |
| 0011 | [SSH properties and startup actions](migration-docs/0011-ssh-properties-and-startup-actions.md) |
| 0012 | [SSH agent preference](migration-docs/0012-ssh-use-agent-preference.md) |
| 0013 | [Saved filter library](migration-docs/0013-saved-filter-library.md) |
| 0014 | [Profile groups](migration-docs/0014-profile-groups.md) |
| 0015 | [Profile group ordering](migration-docs/0015-profile-group-ordering.md) |
| 0016 | [History retention](migration-docs/0016-history-retention.md) |
| 0017 | [Native window session intent](migration-docs/0017-native-window-session-intent.md) |
| 0018 | [Retire support facts](migration-docs/0018-retire-support-facts.md) |
