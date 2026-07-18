CREATE TABLE saved_profile_groups (
    name TEXT PRIMARY KEY NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (length(CAST(name AS BLOB)) BETWEEN 1 AND 128)
) STRICT;

INSERT OR IGNORE INTO saved_profile_groups(name)
SELECT DISTINCT group_name
FROM saved_profiles
WHERE group_name IS NOT NULL;
