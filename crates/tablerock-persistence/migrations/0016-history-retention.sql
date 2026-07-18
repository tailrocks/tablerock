CREATE TABLE IF NOT EXISTS history_preferences (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton = 1),
    retention INTEGER NOT NULL CHECK(retention BETWEEN 1 AND 3)
);

INSERT OR IGNORE INTO history_preferences(singleton, retention) VALUES (1, 1);
