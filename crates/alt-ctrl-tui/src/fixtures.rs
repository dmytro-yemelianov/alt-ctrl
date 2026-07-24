use std::collections::BTreeMap;

use sidecar_core::{
    ActionId, ActionRequest, AdapterConnectionState, AgentKind, ApprovalScope, Capability,
    NormalizedAction, ParseCertainty, PendingAction, PermissionProfile, RiskClass, SessionId,
    SessionState,
};

use crate::model::{PlanItem, PlanItemState, SessionView, TuiState};

pub fn fixture_state() -> TuiState {
    TuiState::new(vec![api_sentinel(), ui_forge(), test_pilot(), docs_lens()])
}

fn api_sentinel() -> SessionView {
    let id = session_id("api-sentinel");
    SessionView {
        id: id.clone(),
        name: "API Sentinel".to_owned(),
        agent: AgentKind::Codex,
        objective: "Harden refresh-token rotation without changing the public API.".to_owned(),
        repository: "alt-ctrl".to_owned(),
        worktree: "worktrees/api-sentinel".to_owned(),
        branch: "agent/refresh-rotation".to_owned(),
        state: SessionState::WaitingForApproval,
        connection: AdapterConnectionState::Connected,
        permission_profile: PermissionProfile::Edit,
        current_tool: "cargo test -p auth-core".to_owned(),
        elapsed: "18m 42s".to_owned(),
        context_percent: 63,
        changed_files: 4,
        git_summary: "4 modified".to_owned(),
        test_summary: "42 passed / 1 pending".to_owned(),
        output: lines(&[
            "turn.started / analysing refresh boundary",
            "Found one stale-token race in rotate_refresh_token().",
            "OK  added a single-use token guard",
            "Running targeted tests before changing the handler...",
            "WAIT approval required / cargo test -p auth-core",
        ]),
        plan: plan(&[
            ("Reproduce the stale-token race", PlanItemState::Done),
            ("Add single-use rotation guard", PlanItemState::Done),
            ("Run focused auth tests", PlanItemState::Active),
            ("Review public API diff", PlanItemState::Queued),
        ]),
        pending_action: Some(PendingAction {
            request: ActionRequest {
                action_id: ActionId::new("action-17").expect("fixture action ID"),
                session_id: id,
                action: NormalizedAction {
                    operation: "process.run".to_owned(),
                    capability: Capability::RunAllowlistedTests,
                    resolved_target: "cargo test -p auth-core refresh_rotation -- --nocapture"
                        .to_owned(),
                    arguments: BTreeMap::from([(
                        "cwd".to_owned(),
                        "worktrees/api-sentinel".to_owned(),
                    )]),
                    risk: RiskClass::Medium,
                    certainty: ParseCertainty::Structured,
                },
            },
            expected_side_effects: vec![
                "Starts a local test process in the assigned worktree.".to_owned(),
            ],
            requested_scope: ApprovalScope::Once,
        }),
    }
}

fn ui_forge() -> SessionView {
    SessionView {
        id: session_id("ui-forge"),
        name: "UI Forge".to_owned(),
        agent: AgentKind::ClaudeCode,
        objective: "Build the controller-first Mission Control layout and focus system.".to_owned(),
        repository: "alt-ctrl".to_owned(),
        worktree: "worktrees/ui-forge".to_owned(),
        branch: "agent/mission-control".to_owned(),
        state: SessionState::Running,
        connection: AdapterConnectionState::Connected,
        permission_profile: PermissionProfile::Edit,
        current_tool: "Editing terminal dashboard".to_owned(),
        elapsed: "11m 09s".to_owned(),
        context_percent: 41,
        changed_files: 7,
        git_summary: "7 modified".to_owned(),
        test_summary: "checks clean".to_owned(),
        output: lines(&[
            "turn.started / mapping terminal layout",
            "Building the session card grid and attention rail.",
            "OK  deterministic focus order wired",
            "Tuning large-format terminal typography and contrast.",
        ]),
        plan: plan(&[
            ("Establish visual tokens", PlanItemState::Done),
            ("Build Mission Control grid", PlanItemState::Active),
            ("Add controller legend", PlanItemState::Queued),
            ("Validate keyboard parity", PlanItemState::Queued),
        ]),
        pending_action: None,
    }
}

fn test_pilot() -> SessionView {
    SessionView {
        id: session_id("test-pilot"),
        name: "Test Pilot".to_owned(),
        agent: AgentKind::Codex,
        objective: "Add crash-recovery fixtures for the durable event store.".to_owned(),
        repository: "alt-ctrl".to_owned(),
        worktree: "worktrees/test-pilot".to_owned(),
        branch: "agent/recovery-fixtures".to_owned(),
        state: SessionState::Failed,
        connection: AdapterConnectionState::Connected,
        permission_profile: PermissionProfile::Build,
        current_tool: "cargo test -p event-store".to_owned(),
        elapsed: "26m 31s".to_owned(),
        context_percent: 78,
        changed_files: 3,
        git_summary: "3 modified".to_owned(),
        test_summary: "18 passed / 2 failed".to_owned(),
        output: lines(&[
            "test.run / event-store recovery suite",
            "OK  18 assertions passed",
            "FAIL reopen_after_truncated_wal",
            "FAIL checkpoint_never_leads_event",
            "The fixture needs a deterministic crash boundary.",
        ]),
        plan: plan(&[
            ("Build restart fixture", PlanItemState::Done),
            ("Inject WAL interruption", PlanItemState::Active),
            ("Prove checkpoint ordering", PlanItemState::Queued),
        ]),
        pending_action: None,
    }
}

fn docs_lens() -> SessionView {
    SessionView {
        id: session_id("docs-lens"),
        name: "Docs Lens".to_owned(),
        agent: AgentKind::GenericPty,
        objective: "Review controller projects and record adoption decisions.".to_owned(),
        repository: "alt-ctrl".to_owned(),
        worktree: "worktrees/docs-lens".to_owned(),
        branch: "agent/reference-review".to_owned(),
        state: SessionState::Completed,
        connection: AdapterConnectionState::Disconnected,
        permission_profile: PermissionProfile::Observe,
        current_tool: "Session complete".to_owned(),
        elapsed: "08m 14s".to_owned(),
        context_percent: 22,
        changed_files: 1,
        git_summary: "clean".to_owned(),
        test_summary: "review complete".to_owned(),
        output: lines(&[
            "session.completed / reference review",
            "OK  9 projects classified by license and relevance",
            "Decisions written to docs/reference-research.md.",
        ]),
        plan: plan(&[
            ("Review controller projects", PlanItemState::Done),
            ("Review orchestrators", PlanItemState::Done),
            ("Record license boundaries", PlanItemState::Done),
        ]),
        pending_action: None,
    }
}

fn session_id(value: &str) -> SessionId {
    SessionId::new(value).expect("fixture session ID")
}

fn lines(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn plan(values: &[(&str, PlanItemState)]) -> Vec<PlanItem> {
    values
        .iter()
        .map(|(label, state)| PlanItem {
            label: (*label).to_owned(),
            state: *state,
        })
        .collect()
}
