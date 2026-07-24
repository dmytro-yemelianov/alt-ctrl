use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    identifiers::{ActionId, SessionId},
    sessions::TransitionOutcome,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionProfile {
    Observe,
    Edit,
    Build,
    Elevated,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ReadRepository,
    ReadLogs,
    InspectGit,
    ModifyWorktree,
    RunAllowlistedTests,
    DeleteOrRevert,
    TerminateOwnedProcess,
    Build,
    RunContainer,
    InstallRepositoryDependency,
    CreateCommit,
    RewriteBranch,
    NetworkAccess,
    Deploy,
    ModifyExternalResource,
}

impl Capability {
    pub const fn minimum_profile(self) -> PermissionProfile {
        match self {
            Self::ReadRepository | Self::ReadLogs | Self::InspectGit => PermissionProfile::Observe,
            Self::ModifyWorktree
            | Self::RunAllowlistedTests
            | Self::DeleteOrRevert
            | Self::TerminateOwnedProcess => PermissionProfile::Edit,
            Self::Build
            | Self::RunContainer
            | Self::InstallRepositoryDependency
            | Self::CreateCommit
            | Self::RewriteBranch => PermissionProfile::Build,
            Self::NetworkAccess | Self::Deploy | Self::ModifyExternalResource => {
                PermissionProfile::Elevated
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseCertainty {
    Structured,
    Inferred,
    Ambiguous,
    Unparsed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalScope {
    Once,
    MatchingActionForSession,
    SessionProfile,
}

/// Stable, transport-independent representation used for policy decisions.
///
/// `resolved_target` must be produced after path/resource resolution. The
/// ordered map makes JSON serialization deterministic for fingerprinting.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NormalizedAction {
    pub operation: String,
    pub capability: Capability,
    pub resolved_target: String,
    #[serde(default)]
    pub arguments: BTreeMap<String, String>,
    pub risk: RiskClass,
    pub certainty: ParseCertainty,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionRequest {
    pub action_id: ActionId,
    pub session_id: SessionId,
    pub action: NormalizedAction,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PendingAction {
    pub request: ActionRequest,
    pub expected_side_effects: Vec<String>,
    pub requested_scope: ApprovalScope,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Inspecting,
    ApprovedOnce,
    ApprovedForSession,
    Executing,
    Succeeded,
    Failed,
    Rejected,
    RejectedWithInstruction,
    Expired,
}

impl ApprovalState {
    /// Validates the immutable approval lifecycle.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidApprovalTransition`] when the requested edge is absent
    /// from the approval state graph.
    pub fn transition_to(
        self,
        next: Self,
    ) -> Result<TransitionOutcome<Self>, InvalidApprovalTransition> {
        if self == next {
            return Ok(TransitionOutcome::Unchanged(self));
        }

        let valid = match self {
            Self::Pending | Self::Inspecting => matches!(
                next,
                Self::Pending
                    | Self::Inspecting
                    | Self::ApprovedOnce
                    | Self::ApprovedForSession
                    | Self::Rejected
                    | Self::RejectedWithInstruction
                    | Self::Expired
            ),
            Self::ApprovedOnce | Self::ApprovedForSession => {
                matches!(next, Self::Executing | Self::Expired)
            }
            Self::Executing => matches!(next, Self::Succeeded | Self::Failed),
            Self::Succeeded
            | Self::Failed
            | Self::Rejected
            | Self::RejectedWithInstruction
            | Self::Expired => false,
        };

        valid
            .then_some(TransitionOutcome::Changed(next))
            .ok_or(InvalidApprovalTransition {
                from: self,
                to: next,
            })
    }

    pub const fn can_execute(self) -> bool {
        matches!(self, Self::ApprovedOnce | Self::ApprovedForSession)
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("invalid approval transition from {from:?} to {to:?}")]
pub struct InvalidApprovalTransition {
    pub from: ApprovalState,
    pub to: ApprovalState,
}

#[cfg(test)]
mod tests {
    use crate::sessions::TransitionOutcome;

    use super::ApprovalState;

    #[test]
    fn approved_action_can_execute_and_finish() {
        assert_eq!(
            ApprovalState::Pending.transition_to(ApprovalState::ApprovedOnce),
            Ok(TransitionOutcome::Changed(ApprovalState::ApprovedOnce))
        );
        assert_eq!(
            ApprovalState::ApprovedOnce.transition_to(ApprovalState::Executing),
            Ok(TransitionOutcome::Changed(ApprovalState::Executing))
        );
        assert_eq!(
            ApprovalState::Executing.transition_to(ApprovalState::Succeeded),
            Ok(TransitionOutcome::Changed(ApprovalState::Succeeded))
        );
    }

    #[test]
    fn changed_or_expired_decision_cannot_execute() {
        assert!(
            ApprovalState::Pending
                .transition_to(ApprovalState::Executing)
                .is_err()
        );
        assert!(
            ApprovalState::Expired
                .transition_to(ApprovalState::Executing)
                .is_err()
        );
        assert!(!ApprovalState::Expired.can_execute());
    }

    #[test]
    fn inspection_returns_to_pending_without_mutating_request() {
        assert_eq!(
            ApprovalState::Inspecting.transition_to(ApprovalState::Pending),
            Ok(TransitionOutcome::Changed(ApprovalState::Pending))
        );
    }
}
