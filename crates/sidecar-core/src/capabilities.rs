//! Conservative capability negotiation for coding-agent adapters.
//!
//! An adapter starts with every capability unverified. It must explicitly
//! promote each capability after preflight, which prevents the UI from
//! presenting guessed support as a native operation.

use serde::{Deserialize, Serialize};

use crate::time::UnixTimestampMillis;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterIntegrationLevel {
    Pty,
    NativeLocal,
    SidecarProtocol,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterCapability {
    StructuredApprovals,
    PartialDiffDecisions,
    PlanEvents,
    UsageEvents,
    Interrupt,
    Resume,
    GracefulTerminate,
    FileEvents,
    TestEvents,
    SessionReplay,
    RemoteTransport,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "level", rename_all = "snake_case")]
pub enum CapabilitySupport {
    Native,
    Fallback { behavior: String },
    Unsupported,
    Unverified,
}

impl CapabilitySupport {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Native | Self::Fallback { .. })
    }

    pub fn is_native(&self) -> bool {
        matches!(self, Self::Native)
    }

    pub fn fallback_behavior(&self) -> Option<&str> {
        match self {
            Self::Fallback { behavior } => Some(behavior),
            Self::Native | Self::Unsupported | Self::Unverified => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdapterCapabilities {
    pub structured_approvals: CapabilitySupport,
    pub partial_diff_decisions: CapabilitySupport,
    pub plan_events: CapabilitySupport,
    pub usage_events: CapabilitySupport,
    pub interrupt: CapabilitySupport,
    pub resume: CapabilitySupport,
    pub graceful_terminate: CapabilitySupport,
    pub file_events: CapabilitySupport,
    pub test_events: CapabilitySupport,
    pub session_replay: CapabilitySupport,
    pub remote_transport: CapabilitySupport,
}

impl AdapterCapabilities {
    pub fn support(&self, capability: AdapterCapability) -> &CapabilitySupport {
        match capability {
            AdapterCapability::StructuredApprovals => &self.structured_approvals,
            AdapterCapability::PartialDiffDecisions => &self.partial_diff_decisions,
            AdapterCapability::PlanEvents => &self.plan_events,
            AdapterCapability::UsageEvents => &self.usage_events,
            AdapterCapability::Interrupt => &self.interrupt,
            AdapterCapability::Resume => &self.resume,
            AdapterCapability::GracefulTerminate => &self.graceful_terminate,
            AdapterCapability::FileEvents => &self.file_events,
            AdapterCapability::TestEvents => &self.test_events,
            AdapterCapability::SessionReplay => &self.session_replay,
            AdapterCapability::RemoteTransport => &self.remote_transport,
        }
    }
}

impl Default for AdapterCapabilities {
    fn default() -> Self {
        Self {
            structured_approvals: CapabilitySupport::Unverified,
            partial_diff_decisions: CapabilitySupport::Unverified,
            plan_events: CapabilitySupport::Unverified,
            usage_events: CapabilitySupport::Unverified,
            interrupt: CapabilitySupport::Unverified,
            resume: CapabilitySupport::Unverified,
            graceful_terminate: CapabilitySupport::Unverified,
            file_events: CapabilitySupport::Unverified,
            test_events: CapabilitySupport::Unverified,
            session_replay: CapabilitySupport::Unverified,
            remote_transport: CapabilitySupport::Unverified,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterReadiness {
    Probing,
    Ready,
    ClientMissing,
    AuthenticationRequired,
    IncompatibleVersion,
    Unavailable,
    Failed,
}

impl AdapterReadiness {
    pub fn can_start_session(self) -> bool {
        matches!(self, Self::Ready)
    }

    pub fn can_offer_fallback(self) -> bool {
        matches!(
            self,
            Self::ClientMissing | Self::IncompatibleVersion | Self::Unavailable | Self::Failed
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdapterCapabilitySnapshot {
    pub integration_level: AdapterIntegrationLevel,
    pub readiness: AdapterReadiness,
    pub installed_version: Option<String>,
    pub protocol_version: Option<String>,
    pub capabilities: AdapterCapabilities,
    pub checked_at: UnixTimestampMillis,
    pub diagnostic: Option<String>,
}

impl AdapterCapabilitySnapshot {
    pub fn probing(
        integration_level: AdapterIntegrationLevel,
        checked_at: UnixTimestampMillis,
    ) -> Self {
        Self {
            integration_level,
            readiness: AdapterReadiness::Probing,
            installed_version: None,
            protocol_version: None,
            capabilities: AdapterCapabilities::default(),
            checked_at,
            diagnostic: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AdapterCapabilities, AdapterCapability, AdapterCapabilitySnapshot, AdapterIntegrationLevel,
        AdapterReadiness, CapabilitySupport,
    };
    use crate::UnixTimestampMillis;

    const ALL_CAPABILITIES: [AdapterCapability; 11] = [
        AdapterCapability::StructuredApprovals,
        AdapterCapability::PartialDiffDecisions,
        AdapterCapability::PlanEvents,
        AdapterCapability::UsageEvents,
        AdapterCapability::Interrupt,
        AdapterCapability::Resume,
        AdapterCapability::GracefulTerminate,
        AdapterCapability::FileEvents,
        AdapterCapability::TestEvents,
        AdapterCapability::SessionReplay,
        AdapterCapability::RemoteTransport,
    ];

    #[test]
    fn capabilities_fail_closed_until_preflight_verifies_them() {
        let capabilities = AdapterCapabilities::default();

        for capability in ALL_CAPABILITIES {
            assert_eq!(
                capabilities.support(capability),
                &CapabilitySupport::Unverified
            );
            assert!(!capabilities.support(capability).is_available());
        }
    }

    #[test]
    fn fallback_is_available_but_never_reported_as_native() {
        let support = CapabilitySupport::Fallback {
            behavior: "generate a previewed follow-up instruction".to_owned(),
        };

        assert!(support.is_available());
        assert!(!support.is_native());
        assert_eq!(
            support.fallback_behavior(),
            Some("generate a previewed follow-up instruction")
        );
    }

    #[test]
    fn readiness_drives_start_and_fallback_choices() {
        assert!(AdapterReadiness::Ready.can_start_session());
        assert!(!AdapterReadiness::AuthenticationRequired.can_start_session());
        assert!(AdapterReadiness::IncompatibleVersion.can_offer_fallback());
        assert!(!AdapterReadiness::AuthenticationRequired.can_offer_fallback());
    }

    #[test]
    fn probing_snapshot_is_conservative_and_round_trips() {
        let snapshot = AdapterCapabilitySnapshot::probing(
            AdapterIntegrationLevel::NativeLocal,
            UnixTimestampMillis(42),
        );

        assert_eq!(snapshot.readiness, AdapterReadiness::Probing);
        let json = serde_json::to_string(&snapshot).expect("serialize snapshot");
        assert_eq!(
            serde_json::from_str::<AdapterCapabilitySnapshot>(&json).expect("deserialize snapshot"),
            snapshot
        );
    }
}
