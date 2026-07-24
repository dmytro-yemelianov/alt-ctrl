use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{approvals::PermissionProfile, identifiers::SessionId, time::UnixTimestampMillis};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum AgentKind {
    Codex,
    ClaudeCode,
    GenericPty,
    Other(String),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Created,
    Starting,
    Running,
    WaitingForApproval,
    WaitingForAnswer,
    Blocked,
    Interrupting,
    Interrupted,
    Resuming,
    Failing,
    Failed,
    Terminating,
    Terminated,
    Completed,
}

impl SessionState {
    /// Validates the lifecycle graph from the product specification.
    ///
    /// Repeating the current state is explicitly idempotent so replayed events
    /// cannot corrupt derived state.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidSessionTransition`] when the requested edge is absent
    /// from the lifecycle graph.
    pub fn transition_to(
        self,
        next: Self,
    ) -> Result<TransitionOutcome<Self>, InvalidSessionTransition> {
        if self == next {
            return Ok(TransitionOutcome::Unchanged(self));
        }

        let valid = match self {
            Self::Created => matches!(next, Self::Starting | Self::Terminating),
            Self::Starting => {
                matches!(next, Self::Running | Self::Failing | Self::Terminating)
            }
            Self::Running => matches!(
                next,
                Self::Completed
                    | Self::WaitingForApproval
                    | Self::WaitingForAnswer
                    | Self::Blocked
                    | Self::Interrupting
                    | Self::Failing
                    | Self::Terminating
            ),
            Self::WaitingForApproval | Self::WaitingForAnswer | Self::Blocked => matches!(
                next,
                Self::Running | Self::Interrupting | Self::Failing | Self::Terminating
            ),
            Self::Interrupting => matches!(next, Self::Interrupted | Self::Failing),
            Self::Interrupted => matches!(next, Self::Resuming | Self::Terminating),
            Self::Resuming => {
                matches!(next, Self::Running | Self::Failing | Self::Terminating)
            }
            Self::Failing => matches!(next, Self::Failed),
            Self::Terminating => matches!(next, Self::Terminated),
            Self::Failed | Self::Terminated | Self::Completed => false,
        };

        valid
            .then_some(TransitionOutcome::Changed(next))
            .ok_or(InvalidSessionTransition {
                from: self,
                to: next,
            })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterConnectionState {
    Unknown,
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
    Incompatible,
}

impl AdapterConnectionState {
    /// Validates an adapter-connectivity transition independently of lifecycle.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidConnectionTransition`] when the requested edge is
    /// absent from the connectivity graph.
    pub fn transition_to(
        self,
        next: Self,
    ) -> Result<TransitionOutcome<Self>, InvalidConnectionTransition> {
        if self == next {
            return Ok(TransitionOutcome::Unchanged(self));
        }

        let valid = match self {
            Self::Unknown => matches!(
                next,
                Self::Connecting | Self::Connected | Self::Disconnected | Self::Incompatible
            ),
            Self::Connecting | Self::Reconnecting => {
                matches!(
                    next,
                    Self::Connected | Self::Disconnected | Self::Incompatible
                )
            }
            Self::Connected => matches!(
                next,
                Self::Reconnecting | Self::Disconnected | Self::Incompatible
            ),
            Self::Disconnected => {
                matches!(
                    next,
                    Self::Connecting | Self::Reconnecting | Self::Incompatible
                )
            }
            Self::Incompatible => matches!(next, Self::Connecting | Self::Disconnected),
        };

        valid
            .then_some(TransitionOutcome::Changed(next))
            .ok_or(InvalidConnectionTransition {
                from: self,
                to: next,
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionOutcome<T> {
    Changed(T),
    Unchanged(T),
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("invalid session transition from {from:?} to {to:?}")]
pub struct InvalidSessionTransition {
    pub from: SessionState,
    pub to: SessionState,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("invalid adapter connection transition from {from:?} to {to:?}")]
pub struct InvalidConnectionTransition {
    pub from: AdapterConnectionState,
    pub to: AdapterConnectionState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AgentSession {
    pub id: SessionId,
    pub name: String,
    pub adapter: AgentKind,
    pub repository: PathBuf,
    pub worktree: PathBuf,
    pub branch: String,
    pub base_revision: String,
    pub objective: String,
    pub state: SessionState,
    pub connection: AdapterConnectionState,
    pub permission_profile: PermissionProfile,
    pub started_at: UnixTimestampMillis,
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{AdapterConnectionState, SessionState, TransitionOutcome};

    fn any_session_state() -> impl Strategy<Value = SessionState> {
        prop_oneof![
            Just(SessionState::Created),
            Just(SessionState::Starting),
            Just(SessionState::Running),
            Just(SessionState::WaitingForApproval),
            Just(SessionState::WaitingForAnswer),
            Just(SessionState::Blocked),
            Just(SessionState::Interrupting),
            Just(SessionState::Interrupted),
            Just(SessionState::Resuming),
            Just(SessionState::Failing),
            Just(SessionState::Failed),
            Just(SessionState::Terminating),
            Just(SessionState::Terminated),
            Just(SessionState::Completed),
        ]
    }

    fn model_allows_session_transition(from: SessionState, to: SessionState) -> bool {
        if from == to {
            return true;
        }
        match from {
            SessionState::Created => {
                matches!(to, SessionState::Starting | SessionState::Terminating)
            }
            SessionState::Starting => matches!(
                to,
                SessionState::Running | SessionState::Failing | SessionState::Terminating
            ),
            SessionState::Running => matches!(
                to,
                SessionState::Completed
                    | SessionState::WaitingForApproval
                    | SessionState::WaitingForAnswer
                    | SessionState::Blocked
                    | SessionState::Interrupting
                    | SessionState::Failing
                    | SessionState::Terminating
            ),
            SessionState::WaitingForApproval
            | SessionState::WaitingForAnswer
            | SessionState::Blocked => matches!(
                to,
                SessionState::Running
                    | SessionState::Interrupting
                    | SessionState::Failing
                    | SessionState::Terminating
            ),
            SessionState::Interrupting => {
                matches!(to, SessionState::Interrupted | SessionState::Failing)
            }
            SessionState::Interrupted => {
                matches!(to, SessionState::Resuming | SessionState::Terminating)
            }
            SessionState::Resuming => matches!(
                to,
                SessionState::Running | SessionState::Failing | SessionState::Terminating
            ),
            SessionState::Failing => to == SessionState::Failed,
            SessionState::Terminating => to == SessionState::Terminated,
            SessionState::Failed | SessionState::Terminated | SessionState::Completed => false,
        }
    }

    #[test]
    fn lifecycle_allows_documented_happy_path() {
        let states = [
            SessionState::Created,
            SessionState::Starting,
            SessionState::Running,
            SessionState::WaitingForApproval,
            SessionState::Running,
            SessionState::Interrupting,
            SessionState::Interrupted,
            SessionState::Resuming,
            SessionState::Running,
            SessionState::Completed,
        ];

        for pair in states.windows(2) {
            assert_eq!(
                pair[0].transition_to(pair[1]),
                Ok(TransitionOutcome::Changed(pair[1]))
            );
        }
    }

    #[test]
    fn lifecycle_rejects_transition_from_terminal_state() {
        let error = SessionState::Completed
            .transition_to(SessionState::Running)
            .expect_err("completed is terminal");
        assert_eq!(error.from, SessionState::Completed);
        assert_eq!(error.to, SessionState::Running);
    }

    #[test]
    fn duplicate_lifecycle_event_is_idempotent() {
        assert_eq!(
            SessionState::Running.transition_to(SessionState::Running),
            Ok(TransitionOutcome::Unchanged(SessionState::Running))
        );
    }

    #[test]
    fn connectivity_is_independent_and_reconnectable() {
        assert_eq!(
            AdapterConnectionState::Connected.transition_to(AdapterConnectionState::Disconnected),
            Ok(TransitionOutcome::Changed(
                AdapterConnectionState::Disconnected
            ))
        );
        assert_eq!(
            AdapterConnectionState::Disconnected
                .transition_to(AdapterConnectionState::Reconnecting),
            Ok(TransitionOutcome::Changed(
                AdapterConnectionState::Reconnecting
            ))
        );
    }

    proptest! {
        #[test]
        fn lifecycle_matches_the_specified_transition_graph(
            from in any_session_state(),
            to in any_session_state(),
        ) {
            let result = from.transition_to(to);
            prop_assert_eq!(
                result.is_ok(),
                model_allows_session_transition(from, to)
            );

            match result {
                Ok(TransitionOutcome::Changed(state)) => {
                    prop_assert_eq!(state, to);
                    prop_assert_ne!(from, to);
                }
                Ok(TransitionOutcome::Unchanged(state)) => {
                    prop_assert_eq!(state, from);
                    prop_assert_eq!(from, to);
                }
                Err(error) => {
                    prop_assert_eq!(error.from, from);
                    prop_assert_eq!(error.to, to);
                }
            }
        }
    }
}
