use serde::{Deserialize, Serialize};

use crate::{
    approvals::PendingAction,
    identifiers::{EventId, QuestionId, SessionId, ToolInvocationId},
    sessions::SessionState,
    time::UnixTimestampMillis,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
}

pub const SCHEMA_VERSION_V1: SchemaVersion = SchemaVersion { major: 1, minor: 0 };

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum EventSource {
    Adapter(String),
    Controller,
    Daemon,
    PolicyEngine,
    RepositoryObserver,
    Supervisor,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
    Terminal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum AgentEvent {
    Output {
        stream: OutputStream,
        text: String,
    },
    PlanUpdated {
        summary: String,
        steps: Vec<String>,
    },
    ToolStarted {
        invocation_id: ToolInvocationId,
        name: String,
    },
    ToolFinished {
        invocation_id: ToolInvocationId,
        succeeded: bool,
        summary: String,
    },
    ApprovalRequested(PendingAction),
    FileChanged {
        path: String,
        change_kind: String,
    },
    TestsUpdated {
        passed: u64,
        failed: u64,
        skipped: u64,
        duration_ms: u64,
    },
    QuestionAsked {
        question_id: QuestionId,
        prompt: String,
    },
    ContextWarning {
        used_percent: u8,
    },
    StateChanged(SessionState),
    Failed {
        code: Option<String>,
        message: String,
    },
    Completed {
        summary: String,
    },
}

/// Durable normalized event with per-session ordering.
///
/// `sequence` is monotonic within one session. Consumers deduplicate by
/// `event_id` and must tolerate replay.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventEnvelope {
    pub schema_version: SchemaVersion,
    pub event_id: EventId,
    pub session_id: SessionId,
    pub sequence: u64,
    pub occurred_at: UnixTimestampMillis,
    pub source: EventSource,
    pub event: AgentEvent,
}

impl EventEnvelope {
    pub fn is_compatible_with(self_version: SchemaVersion, other: SchemaVersion) -> bool {
        self_version.major == other.major && other.minor <= self_version.minor
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{EventEnvelope, SCHEMA_VERSION_V1, SchemaVersion};

    #[test]
    fn schema_compatibility_accepts_older_minor_in_same_major() {
        let reader = SchemaVersion { major: 1, minor: 3 };
        assert!(EventEnvelope::is_compatible_with(reader, SCHEMA_VERSION_V1));
    }

    #[test]
    fn schema_compatibility_rejects_newer_minor_and_other_major() {
        assert!(!EventEnvelope::is_compatible_with(
            SCHEMA_VERSION_V1,
            SchemaVersion { major: 1, minor: 1 }
        ));
        assert!(!EventEnvelope::is_compatible_with(
            SCHEMA_VERSION_V1,
            SchemaVersion { major: 2, minor: 0 }
        ));
    }

    proptest! {
        #[test]
        fn compatibility_is_same_major_and_non_newer_minor(
            reader_major in any::<u16>(),
            reader_minor in any::<u16>(),
            writer_major in any::<u16>(),
            writer_minor in any::<u16>(),
        ) {
            let reader = SchemaVersion {
                major: reader_major,
                minor: reader_minor,
            };
            let writer = SchemaVersion {
                major: writer_major,
                minor: writer_minor,
            };

            prop_assert_eq!(
                EventEnvelope::is_compatible_with(reader, writer),
                reader_major == writer_major && writer_minor <= reader_minor
            );
        }
    }
}
