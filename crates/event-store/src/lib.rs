//! Durable normalized-event storage and reducer checkpoints.
//!
//! An event and its optional reducer checkpoint are committed in one `SQLite`
//! transaction. The UI or daemon must acknowledge the event only after
//! [`EventStore::append`] succeeds.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sidecar_core::{
    AdapterConnectionState, EventEnvelope, SCHEMA_VERSION_V1, SessionId, SessionState,
    UnixTimestampMillis,
};
use thiserror::Error;

const DATABASE_VERSION: i64 = 1;
const INITIAL_MIGRATION: &str = include_str!("../migrations/0001_initial.sql");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReducerCheckpoint {
    pub session_id: SessionId,
    pub sequence: u64,
    pub session_state: SessionState,
    pub connection_state: AdapterConnectionState,
    pub updated_at: UnixTimestampMillis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppendOutcome {
    Appended,
    AlreadyPresent,
}

/// Persistence contract used by the daemon's event-ingestion path.
pub trait EventStore {
    /// Atomically appends an event and its optional derived-state checkpoint.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] for incompatible schemas, sequence gaps or
    /// collisions, changed duplicate payloads, invalid checkpoints,
    /// serialization failures, and `SQLite` failures.
    fn append(
        &mut self,
        event: &EventEnvelope,
        checkpoint: Option<&ReducerCheckpoint>,
    ) -> Result<AppendOutcome, StoreError>;

    /// Replays events after the supplied per-session sequence, in order.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the sequence cannot fit `SQLite`'s signed
    /// integer representation, a stored record is corrupt, or `SQLite` fails.
    fn replay(
        &self,
        session_id: &SessionId,
        after_sequence: u64,
    ) -> Result<Vec<EventEnvelope>, StoreError>;

    /// Loads the most recently committed reducer checkpoint for a session.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the stored checkpoint is corrupt or `SQLite`
    /// fails.
    fn latest_checkpoint(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<ReducerCheckpoint>, StoreError>;
}

#[derive(Debug)]
pub struct SqliteEventStore {
    connection: Connection,
}

impl SqliteEventStore {
    /// Opens or creates a store and applies supported migrations.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the database cannot be opened, configured,
    /// migrated, or has a newer unsupported schema.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::initialize(Connection::open(path)?)
    }

    /// Creates a migrated in-memory store, primarily for tests and fixtures.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when `SQLite` setup or migration fails.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn initialize(mut connection: Connection) -> Result<Self, StoreError> {
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            PRAGMA busy_timeout = 5000;
            ",
        )?;

        let version =
            connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?;
        match version {
            0 => {
                let transaction =
                    connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
                transaction.execute_batch(INITIAL_MIGRATION)?;
                transaction.commit()?;
            }
            DATABASE_VERSION => {}
            other => return Err(StoreError::UnsupportedDatabaseVersion(other)),
        }

        Ok(Self { connection })
    }

    fn validate_checkpoint(
        event: &EventEnvelope,
        checkpoint: Option<&ReducerCheckpoint>,
    ) -> Result<(), StoreError> {
        let Some(checkpoint) = checkpoint else {
            return Ok(());
        };
        if checkpoint.session_id != event.session_id {
            return Err(StoreError::InvalidCheckpoint(
                "checkpoint session does not match event session",
            ));
        }
        if checkpoint.sequence != event.sequence {
            return Err(StoreError::InvalidCheckpoint(
                "checkpoint sequence does not match event sequence",
            ));
        }
        Ok(())
    }

    fn existing_payload_by_event_id(
        transaction: &Transaction<'_>,
        event_id: &str,
    ) -> Result<Option<String>, StoreError> {
        Ok(transaction
            .query_row(
                "SELECT payload_json FROM events WHERE event_id = ?1",
                [event_id],
                |row| row.get(0),
            )
            .optional()?)
    }

    fn validate_sequence(
        transaction: &Transaction<'_>,
        event: &EventEnvelope,
        sequence: i64,
    ) -> Result<(), StoreError> {
        let event_at_sequence: Option<String> = transaction
            .query_row(
                "
                SELECT event_id
                FROM events
                WHERE session_id = ?1 AND sequence = ?2
                ",
                params![event.session_id.as_str(), sequence],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(existing_event_id) = event_at_sequence {
            return Err(StoreError::SequenceConflict {
                session_id: event.session_id.to_string(),
                sequence: event.sequence,
                existing_event_id,
            });
        }

        let latest: Option<i64> = transaction.query_row(
            "SELECT MAX(sequence) FROM events WHERE session_id = ?1",
            [event.session_id.as_str()],
            |row| row.get(0),
        )?;
        let expected = match latest {
            Some(value) => sql_sequence_to_u64(value)?.saturating_add(1),
            None => 1,
        };
        if event.sequence != expected {
            return Err(StoreError::SequenceGap {
                session_id: event.session_id.to_string(),
                expected,
                actual: event.sequence,
            });
        }
        Ok(())
    }

    fn insert_event(
        transaction: &Transaction<'_>,
        event: &EventEnvelope,
        sequence: i64,
        payload_json: &str,
    ) -> Result<(), StoreError> {
        transaction.execute(
            "
            INSERT INTO events (
                event_id,
                session_id,
                sequence,
                occurred_at_ms,
                schema_major,
                schema_minor,
                payload_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ",
            params![
                event.event_id.as_str(),
                event.session_id.as_str(),
                sequence,
                event.occurred_at.0,
                i64::from(event.schema_version.major),
                i64::from(event.schema_version.minor),
                payload_json,
            ],
        )?;
        Ok(())
    }

    fn upsert_checkpoint(
        transaction: &Transaction<'_>,
        checkpoint: &ReducerCheckpoint,
        sequence: i64,
        checkpoint_json: &str,
    ) -> Result<(), StoreError> {
        transaction.execute(
            "
            INSERT INTO reducer_checkpoints (
                session_id,
                sequence,
                updated_at_ms,
                payload_json
            )
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(session_id) DO UPDATE SET
                sequence = excluded.sequence,
                updated_at_ms = excluded.updated_at_ms,
                payload_json = excluded.payload_json
            WHERE excluded.sequence > reducer_checkpoints.sequence
            ",
            params![
                checkpoint.session_id.as_str(),
                sequence,
                checkpoint.updated_at.0,
                checkpoint_json,
            ],
        )?;
        Ok(())
    }
}

impl EventStore for SqliteEventStore {
    fn append(
        &mut self,
        event: &EventEnvelope,
        checkpoint: Option<&ReducerCheckpoint>,
    ) -> Result<AppendOutcome, StoreError> {
        if !EventEnvelope::is_compatible_with(SCHEMA_VERSION_V1, event.schema_version) {
            return Err(StoreError::UnsupportedEventSchema {
                major: event.schema_version.major,
                minor: event.schema_version.minor,
            });
        }
        Self::validate_checkpoint(event, checkpoint)?;

        let sequence = sequence_to_sql(event.sequence)?;
        let payload_json = serde_json::to_string(event)?;
        let checkpoint_json = checkpoint.map(serde_json::to_string).transpose()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) =
            Self::existing_payload_by_event_id(&transaction, event.event_id.as_str())?
        {
            if existing == payload_json {
                return Ok(AppendOutcome::AlreadyPresent);
            }
            return Err(StoreError::EventIdConflict(event.event_id.to_string()));
        }

        Self::validate_sequence(&transaction, event, sequence)?;
        Self::insert_event(&transaction, event, sequence, &payload_json)?;
        if let (Some(checkpoint), Some(checkpoint_json)) = (checkpoint, checkpoint_json) {
            Self::upsert_checkpoint(&transaction, checkpoint, sequence, &checkpoint_json)?;
        }

        transaction.commit()?;
        Ok(AppendOutcome::Appended)
    }

    fn replay(
        &self,
        session_id: &SessionId,
        after_sequence: u64,
    ) -> Result<Vec<EventEnvelope>, StoreError> {
        let after_sequence = sequence_to_sql(after_sequence)?;
        let mut statement = self.connection.prepare(
            "
            SELECT payload_json
            FROM events
            WHERE session_id = ?1 AND sequence > ?2
            ORDER BY sequence ASC
            ",
        )?;
        let rows = statement.query_map(params![session_id.as_str(), after_sequence], |row| {
            row.get::<_, String>(0)
        })?;

        rows.map(|row| {
            let payload = row?;
            serde_json::from_str(&payload).map_err(StoreError::from)
        })
        .collect()
    }

    fn latest_checkpoint(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<ReducerCheckpoint>, StoreError> {
        let payload: Option<String> = self
            .connection
            .query_row(
                "
                SELECT payload_json
                FROM reducer_checkpoints
                WHERE session_id = ?1
                ",
                [session_id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        payload
            .map(|payload| serde_json::from_str(&payload).map_err(StoreError::from))
            .transpose()
    }
}

fn sequence_to_sql(sequence: u64) -> Result<i64, StoreError> {
    i64::try_from(sequence).map_err(|_| StoreError::SequenceOutOfRange(sequence))
}

fn sql_sequence_to_u64(sequence: i64) -> Result<u64, StoreError> {
    u64::try_from(sequence)
        .map_err(|_| StoreError::CorruptRecord("stored sequence is negative".to_owned()))
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database schema version {0} is newer than this binary supports")]
    UnsupportedDatabaseVersion(i64),
    #[error("event schema {major}.{minor} is not supported")]
    UnsupportedEventSchema { major: u16, minor: u16 },
    #[error("event sequence {0} exceeds SQLite's supported integer range")]
    SequenceOutOfRange(u64),
    #[error("event sequence gap for session {session_id}: expected {expected}, received {actual}")]
    SequenceGap {
        session_id: String,
        expected: u64,
        actual: u64,
    },
    #[error(
        "sequence {sequence} for session {session_id} is already held by event {existing_event_id}"
    )]
    SequenceConflict {
        session_id: String,
        sequence: u64,
        existing_event_id: String,
    },
    #[error("event ID {0} was replayed with a changed payload")]
    EventIdConflict(String),
    #[error("invalid reducer checkpoint: {0}")]
    InvalidCheckpoint(&'static str),
    #[error("corrupt stored record: {0}")]
    CorruptRecord(String),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

#[cfg(test)]
mod tests {
    use sidecar_core::{
        AgentEvent, EventEnvelope, EventId, EventSource, OutputStream, SCHEMA_VERSION_V1,
        SessionId, SessionState, UnixTimestampMillis,
    };
    use tempfile::tempdir;

    use super::{AppendOutcome, EventStore, ReducerCheckpoint, SqliteEventStore, StoreError};

    fn session_id() -> SessionId {
        SessionId::new("session-01").expect("valid session ID")
    }

    fn event(event_id: &str, sequence: u64, text: &str) -> EventEnvelope {
        EventEnvelope {
            schema_version: SCHEMA_VERSION_V1,
            event_id: EventId::new(event_id).expect("valid event ID"),
            session_id: session_id(),
            sequence,
            occurred_at: UnixTimestampMillis(1_000 + i64::try_from(sequence).unwrap_or(0)),
            source: EventSource::Adapter("fixture".to_owned()),
            event: AgentEvent::Output {
                stream: OutputStream::Terminal,
                text: text.to_owned(),
            },
        }
    }

    fn checkpoint(sequence: u64) -> ReducerCheckpoint {
        ReducerCheckpoint {
            session_id: session_id(),
            sequence,
            session_state: SessionState::Running,
            connection_state: AdapterConnectionState::Connected,
            updated_at: UnixTimestampMillis(2_000),
        }
    }

    use sidecar_core::AdapterConnectionState;

    #[test]
    fn append_and_replay_preserve_order_and_payloads() {
        let mut store = SqliteEventStore::open_in_memory().expect("store");
        let first = event("event-01", 1, "first");
        let second = event("event-02", 2, "second");

        assert_eq!(
            store.append(&first, None).expect("append"),
            AppendOutcome::Appended
        );
        assert_eq!(
            store.append(&second, Some(&checkpoint(2))).expect("append"),
            AppendOutcome::Appended
        );
        assert_eq!(
            store.replay(&session_id(), 0).expect("replay"),
            vec![first, second]
        );
        assert_eq!(
            store.latest_checkpoint(&session_id()).expect("checkpoint"),
            Some(checkpoint(2))
        );
    }

    #[test]
    fn identical_replay_is_idempotent() {
        let mut store = SqliteEventStore::open_in_memory().expect("store");
        let event = event("event-01", 1, "same");

        assert_eq!(
            store.append(&event, None).expect("append"),
            AppendOutcome::Appended
        );
        assert_eq!(
            store.append(&event, None).expect("replay"),
            AppendOutcome::AlreadyPresent
        );
        assert_eq!(store.replay(&session_id(), 0).expect("events").len(), 1);
    }

    #[test]
    fn changed_duplicate_payload_fails_closed() {
        let mut store = SqliteEventStore::open_in_memory().expect("store");
        store
            .append(&event("event-01", 1, "original"), None)
            .expect("append");

        assert!(matches!(
            store.append(&event("event-01", 1, "changed"), None),
            Err(StoreError::EventIdConflict(_))
        ));
    }

    #[test]
    fn sequence_collision_and_gap_are_rejected() {
        let mut store = SqliteEventStore::open_in_memory().expect("store");
        store
            .append(&event("event-01", 1, "first"), None)
            .expect("append");

        assert!(matches!(
            store.append(&event("event-02", 1, "collision"), None),
            Err(StoreError::SequenceConflict { .. })
        ));
        assert!(matches!(
            store.append(&event("event-03", 3, "gap"), None),
            Err(StoreError::SequenceGap {
                expected: 2,
                actual: 3,
                ..
            })
        ));
    }

    #[test]
    fn invalid_checkpoint_does_not_partially_append_event() {
        let mut store = SqliteEventStore::open_in_memory().expect("store");

        assert!(matches!(
            store.append(&event("event-01", 1, "first"), Some(&checkpoint(2))),
            Err(StoreError::InvalidCheckpoint(_))
        ));
        assert!(store.replay(&session_id(), 0).expect("replay").is_empty());
    }

    #[test]
    fn file_store_reopens_with_durable_history() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("events.sqlite");
        {
            let mut store = SqliteEventStore::open(&path).expect("store");
            store
                .append(&event("event-01", 1, "durable"), Some(&checkpoint(1)))
                .expect("append");
        }

        let reopened = SqliteEventStore::open(&path).expect("reopen");
        assert_eq!(
            reopened.replay(&session_id(), 0).expect("replay"),
            vec![event("event-01", 1, "durable")]
        );
        assert_eq!(
            reopened
                .latest_checkpoint(&session_id())
                .expect("checkpoint"),
            Some(checkpoint(1))
        );
    }
}
