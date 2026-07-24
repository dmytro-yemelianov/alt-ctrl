//! Fail-closed policy decisions and payload-bound approval grants.

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sidecar_core::{
    ActionId, ActionRequest, ApprovalScope, ParseCertainty, PermissionProfile, RiskClass,
    SessionId, UnixTimestampMillis,
};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ActionFingerprint([u8; 32]);

impl ActionFingerprint {
    /// Fingerprints the normalized operation and resolved target, intentionally
    /// excluding request identity so matching-action grants can authorize a new
    /// action ID with exactly the same normalized payload.
    ///
    /// # Errors
    ///
    /// Returns [`FingerprintError`] if the normalized action cannot be
    /// serialized into its canonical JSON representation.
    pub fn for_request(request: &ActionRequest) -> Result<Self, FingerprintError> {
        let bytes = serde_json::to_vec(&request.action).map_err(FingerprintError)?;
        Ok(Self(Sha256::digest(bytes).into()))
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ActionFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
#[error("could not canonicalize normalized action: {0}")]
pub struct FingerprintError(serde_json::Error);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConfirmationRequirement {
    AutoApproveAndLog,
    PressConfirm,
    Hold { duration_ms: u64 },
    InspectThenConfirm,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum PolicyDecision {
    AllowWithoutPrompt {
        fingerprint: ActionFingerprint,
    },
    RequireConfirmation {
        fingerprint: ActionFingerprint,
        requirement: ConfirmationRequirement,
        allowed_scopes: Vec<ApprovalScope>,
    },
    Deny {
        reason: DenyReason,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DenyReason {
    InsufficientProfile {
        active: PermissionProfile,
        required: PermissionProfile,
    },
    AmbiguousOrUnparsed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConfirmationEvidence {
    Automatic,
    Pressed,
    Held { duration_ms: u64 },
    InspectedThenConfirmed,
}

#[derive(Clone, Debug)]
pub struct PolicyEngine {
    high_risk_hold_ms: u64,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self {
            high_risk_hold_ms: 1_000,
        }
    }
}

impl PolicyEngine {
    pub const fn new(high_risk_hold_ms: u64) -> Self {
        Self { high_risk_hold_ms }
    }

    /// Evaluates an action against the active permission ceiling.
    ///
    /// # Errors
    ///
    /// Returns [`FingerprintError`] if an otherwise eligible normalized action
    /// cannot be serialized for payload binding.
    pub fn evaluate(
        &self,
        active_profile: PermissionProfile,
        request: &ActionRequest,
    ) -> Result<PolicyDecision, FingerprintError> {
        if matches!(
            request.action.certainty,
            ParseCertainty::Ambiguous | ParseCertainty::Unparsed
        ) {
            return Ok(PolicyDecision::Deny {
                reason: DenyReason::AmbiguousOrUnparsed,
            });
        }

        let required_profile = request.action.capability.minimum_profile();
        if active_profile < required_profile {
            return Ok(PolicyDecision::Deny {
                reason: DenyReason::InsufficientProfile {
                    active: active_profile,
                    required: required_profile,
                },
            });
        }

        let fingerprint = ActionFingerprint::for_request(request)?;
        let requirement = match (request.action.risk, request.action.certainty) {
            (RiskClass::Low, ParseCertainty::Structured) => {
                ConfirmationRequirement::AutoApproveAndLog
            }
            (RiskClass::Low | RiskClass::Medium, _) => ConfirmationRequirement::PressConfirm,
            (RiskClass::High, _) => ConfirmationRequirement::Hold {
                duration_ms: self.high_risk_hold_ms,
            },
            (RiskClass::Critical, _) => ConfirmationRequirement::InspectThenConfirm,
        };

        if requirement == ConfirmationRequirement::AutoApproveAndLog {
            return Ok(PolicyDecision::AllowWithoutPrompt { fingerprint });
        }

        let allowed_scopes = match request.action.risk {
            RiskClass::Low | RiskClass::Medium => {
                vec![ApprovalScope::Once, ApprovalScope::MatchingActionForSession]
            }
            RiskClass::High | RiskClass::Critical => vec![ApprovalScope::Once],
        };

        Ok(PolicyDecision::RequireConfirmation {
            fingerprint,
            requirement,
            allowed_scopes,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApprovalGrant {
    pub action_id: ActionId,
    pub session_id: SessionId,
    pub fingerprint: ActionFingerprint,
    pub scope: ApprovalScope,
    pub expires_at: UnixTimestampMillis,
    pub evidence: ConfirmationEvidence,
}

impl ApprovalGrant {
    /// Revalidates an approval immediately before action dispatch.
    ///
    /// # Errors
    ///
    /// Returns [`GrantValidationError`] if the grant expired, belongs to
    /// another request/session, has an unsupported scope, or the normalized
    /// payload changed.
    pub fn validate(
        &self,
        request: &ActionRequest,
        now: UnixTimestampMillis,
    ) -> Result<(), GrantValidationError> {
        if now > self.expires_at {
            return Err(GrantValidationError::Expired);
        }
        if self.session_id != request.session_id {
            return Err(GrantValidationError::WrongSession);
        }

        match self.scope {
            ApprovalScope::Once if self.action_id != request.action_id => {
                return Err(GrantValidationError::WrongAction);
            }
            ApprovalScope::SessionProfile => {
                return Err(GrantValidationError::ProfileScopeIsNotActionGrant);
            }
            ApprovalScope::Once | ApprovalScope::MatchingActionForSession => {}
        }

        let actual =
            ActionFingerprint::for_request(request).map_err(GrantValidationError::Fingerprint)?;
        if self.fingerprint != actual {
            return Err(GrantValidationError::PayloadChanged);
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum GrantValidationError {
    #[error("approval grant has expired")]
    Expired,
    #[error("approval grant belongs to a different session")]
    WrongSession,
    #[error("one-time approval grant belongs to a different action")]
    WrongAction,
    #[error("session-profile changes are not action grants")]
    ProfileScopeIsNotActionGrant,
    #[error("action payload changed after approval")]
    PayloadChanged,
    #[error(transparent)]
    Fingerprint(#[from] FingerprintError),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use sidecar_core::{
        ActionId, ActionRequest, ApprovalScope, Capability, NormalizedAction, ParseCertainty,
        PermissionProfile, RiskClass, SessionId, UnixTimestampMillis,
    };

    use super::{
        ActionFingerprint, ApprovalGrant, ConfirmationEvidence, ConfirmationRequirement,
        DenyReason, GrantValidationError, PolicyDecision, PolicyEngine,
    };

    fn request(
        action_id: &str,
        capability: Capability,
        risk: RiskClass,
        certainty: ParseCertainty,
    ) -> ActionRequest {
        ActionRequest {
            action_id: ActionId::new(action_id).expect("valid action ID"),
            session_id: SessionId::new("session-01").expect("valid session ID"),
            action: NormalizedAction {
                operation: "run".to_owned(),
                capability,
                resolved_target: "/repo/worktree".to_owned(),
                arguments: BTreeMap::from([("command".to_owned(), "cargo test".to_owned())]),
                risk,
                certainty,
            },
        }
    }

    #[test]
    fn profile_is_a_ceiling() {
        let request = request(
            "action-01",
            Capability::ModifyWorktree,
            RiskClass::Medium,
            ParseCertainty::Structured,
        );

        assert_eq!(
            PolicyEngine::default()
                .evaluate(PermissionProfile::Observe, &request)
                .expect("policy evaluation"),
            PolicyDecision::Deny {
                reason: DenyReason::InsufficientProfile {
                    active: PermissionProfile::Observe,
                    required: PermissionProfile::Edit,
                }
            }
        );
    }

    #[test]
    fn ambiguous_action_fails_closed() {
        let request = request(
            "action-01",
            Capability::ReadRepository,
            RiskClass::Low,
            ParseCertainty::Ambiguous,
        );

        assert_eq!(
            PolicyEngine::default()
                .evaluate(PermissionProfile::Elevated, &request)
                .expect("policy evaluation"),
            PolicyDecision::Deny {
                reason: DenyReason::AmbiguousOrUnparsed
            }
        );
    }

    #[test]
    fn structured_low_risk_read_is_logged_without_prompt() {
        let request = request(
            "action-01",
            Capability::ReadRepository,
            RiskClass::Low,
            ParseCertainty::Structured,
        );

        assert!(matches!(
            PolicyEngine::default()
                .evaluate(PermissionProfile::Observe, &request)
                .expect("policy evaluation"),
            PolicyDecision::AllowWithoutPrompt { .. }
        ));
    }

    #[test]
    fn inferred_low_risk_action_still_requires_confirmation() {
        let request = request(
            "action-01",
            Capability::ReadRepository,
            RiskClass::Low,
            ParseCertainty::Inferred,
        );

        assert!(matches!(
            PolicyEngine::default()
                .evaluate(PermissionProfile::Observe, &request)
                .expect("policy evaluation"),
            PolicyDecision::RequireConfirmation {
                requirement: ConfirmationRequirement::PressConfirm,
                ..
            }
        ));
    }

    #[test]
    fn high_risk_action_requires_configured_hold() {
        let request = request(
            "action-01",
            Capability::DeleteOrRevert,
            RiskClass::High,
            ParseCertainty::Structured,
        );

        assert!(matches!(
            PolicyEngine::new(1_250)
                .evaluate(PermissionProfile::Edit, &request)
                .expect("policy evaluation"),
            PolicyDecision::RequireConfirmation {
                requirement: ConfirmationRequirement::Hold { duration_ms: 1_250 },
                ..
            }
        ));
    }

    #[test]
    fn grant_is_invalidated_when_payload_changes() {
        let original = request(
            "action-01",
            Capability::ModifyWorktree,
            RiskClass::Medium,
            ParseCertainty::Structured,
        );
        let grant = ApprovalGrant {
            action_id: original.action_id.clone(),
            session_id: original.session_id.clone(),
            fingerprint: ActionFingerprint::for_request(&original).expect("fingerprint"),
            scope: ApprovalScope::Once,
            expires_at: UnixTimestampMillis(2_000),
            evidence: ConfirmationEvidence::Pressed,
        };

        let mut changed = original.clone();
        changed
            .action
            .arguments
            .insert("command".to_owned(), "cargo test --all".to_owned());

        assert!(matches!(
            grant.validate(&changed, UnixTimestampMillis(1_000)),
            Err(GrantValidationError::PayloadChanged)
        ));
    }

    #[test]
    fn matching_action_scope_accepts_new_action_id_in_same_session() {
        let original = request(
            "action-01",
            Capability::ModifyWorktree,
            RiskClass::Medium,
            ParseCertainty::Structured,
        );
        let grant = ApprovalGrant {
            action_id: original.action_id.clone(),
            session_id: original.session_id.clone(),
            fingerprint: ActionFingerprint::for_request(&original).expect("fingerprint"),
            scope: ApprovalScope::MatchingActionForSession,
            expires_at: UnixTimestampMillis(2_000),
            evidence: ConfirmationEvidence::Pressed,
        };
        let mut repeated = original.clone();
        repeated.action_id = ActionId::new("action-02").expect("valid action ID");

        assert!(
            grant
                .validate(&repeated, UnixTimestampMillis(1_000))
                .is_ok()
        );
    }

    #[test]
    fn expired_grant_is_rejected_before_dispatch() {
        let request = request(
            "action-01",
            Capability::ModifyWorktree,
            RiskClass::Medium,
            ParseCertainty::Structured,
        );
        let grant = ApprovalGrant {
            action_id: request.action_id.clone(),
            session_id: request.session_id.clone(),
            fingerprint: ActionFingerprint::for_request(&request).expect("fingerprint"),
            scope: ApprovalScope::Once,
            expires_at: UnixTimestampMillis(999),
            evidence: ConfirmationEvidence::Pressed,
        };

        assert!(matches!(
            grant.validate(&request, UnixTimestampMillis(1_000)),
            Err(GrantValidationError::Expired)
        ));
    }
}
