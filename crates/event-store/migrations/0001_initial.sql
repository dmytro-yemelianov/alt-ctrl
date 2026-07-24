CREATE TABLE events (
    event_id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK (sequence >= 1),
    occurred_at_ms INTEGER NOT NULL,
    schema_major INTEGER NOT NULL,
    schema_minor INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    UNIQUE (session_id, sequence)
);

CREATE INDEX events_by_session_time
    ON events (session_id, occurred_at_ms);

CREATE TABLE reducer_checkpoints (
    session_id TEXT PRIMARY KEY NOT NULL,
    sequence INTEGER NOT NULL CHECK (sequence >= 1),
    updated_at_ms INTEGER NOT NULL,
    payload_json TEXT NOT NULL
);

PRAGMA user_version = 1;
