-- Prefer SSH agent auth when a bastion is configured (profile preference).
ALTER TABLE saved_profiles ADD COLUMN ssh_use_agent INTEGER NOT NULL DEFAULT 0
    CHECK(ssh_use_agent IN (0, 1));
