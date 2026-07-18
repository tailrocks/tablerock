ALTER TABLE saved_profile_groups ADD COLUMN sort_mode INTEGER NOT NULL DEFAULT 1
    CHECK(sort_mode IN (1, 2));
