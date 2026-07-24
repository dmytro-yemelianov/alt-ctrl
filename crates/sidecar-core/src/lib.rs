//! Platform-neutral domain contracts for Alt Ctrl.
//!
//! This crate deliberately contains no operating-system or async runtime
//! dependencies. Reducers, policy code, protocol adapters, and UI clients share
//! these types.

pub mod actions;
pub mod approvals;
pub mod capabilities;
pub mod events;
pub mod identifiers;
pub mod navigation;
pub mod sessions;
pub mod time;

pub use actions::{FocusDirection, ScrollDirection, UiAction};
pub use approvals::{
    ActionRequest, ApprovalScope, ApprovalState, Capability, NormalizedAction, ParseCertainty,
    PendingAction, PermissionProfile, RiskClass,
};
pub use capabilities::{
    AdapterCapabilities, AdapterCapability, AdapterCapabilitySnapshot, AdapterIntegrationLevel,
    AdapterReadiness, CapabilitySupport,
};
pub use events::{
    AgentEvent, EventEnvelope, EventSource, OutputStream, SCHEMA_VERSION_V1, SchemaVersion,
};
pub use identifiers::{ActionId, EventId, QuestionId, SessionId, ToolInvocationId};
pub use navigation::{
    AgentView, AppState, NavigationEffect, NavigationScreen, NavigationTransition,
    reduce_navigation,
};
pub use sessions::{
    AdapterConnectionState, AgentKind, AgentSession, SessionState, TransitionOutcome,
};
pub use time::{MonotonicMillis, UnixTimestampMillis};
