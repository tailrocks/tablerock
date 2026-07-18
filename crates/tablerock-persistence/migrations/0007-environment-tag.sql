-- Optional environment tag for saved profiles.
-- kind: 1 Production, 2 Staging, 3 Development, 4 Testing, 5 Custom (+ label).
-- NULL kind means no environment (pre-0007 rows and new untagged profiles).

ALTER TABLE saved_profiles ADD COLUMN environment_kind INTEGER
    CHECK(
        environment_kind IS NULL
        OR environment_kind BETWEEN 1 AND 5
    );

ALTER TABLE saved_profiles ADD COLUMN environment_label TEXT
    CHECK(
        environment_label IS NULL
        OR length(CAST(environment_label AS BLOB)) BETWEEN 1 AND 64
    );

-- Custom labels require kind 5; fixed kinds require NULL label.
-- SQLite cannot add multi-column CHECK via ALTER easily; enforce in Rust decode/encode.

CREATE INDEX saved_profiles_environment
    ON saved_profiles(environment_kind, profile_id);
