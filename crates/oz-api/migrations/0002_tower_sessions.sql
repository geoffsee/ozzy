DROP INDEX IF EXISTS idx_sessions_token_hash;
DROP INDEX IF EXISTS idx_sessions_profile_id;
DROP TABLE IF EXISTS sessions;

CREATE TABLE tower_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    data TEXT NOT NULL,
    expiry_date TEXT NOT NULL
);

CREATE INDEX idx_tower_sessions_expiry ON tower_sessions(expiry_date);
