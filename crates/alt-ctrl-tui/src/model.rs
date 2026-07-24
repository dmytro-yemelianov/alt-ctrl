use sidecar_core::{
    AgentKind, AgentView, AppState, NavigationEffect, NavigationScreen, PendingAction,
    PermissionProfile, SessionId, SessionState, UiAction, reduce_navigation,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanItemState {
    Done,
    Active,
    Queued,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanItem {
    pub label: String,
    pub state: PlanItemState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionView {
    pub id: SessionId,
    pub name: String,
    pub agent: AgentKind,
    pub objective: String,
    pub repository: String,
    pub worktree: String,
    pub branch: String,
    pub state: SessionState,
    pub connection: sidecar_core::AdapterConnectionState,
    pub permission_profile: PermissionProfile,
    pub current_tool: String,
    pub elapsed: String,
    pub context_percent: u8,
    pub changed_files: usize,
    pub git_summary: String,
    pub test_summary: String,
    pub output: Vec<String>,
    pub plan: Vec<PlanItem>,
    pub pending_action: Option<PendingAction>,
}

impl SessionView {
    pub fn attention_count(&self) -> usize {
        usize::from(self.pending_action.is_some())
            + usize::from(matches!(
                self.state,
                SessionState::Failed | SessionState::Blocked
            ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Semantic(UiAction),
    RejectPending,
    Quit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TuiEffect {
    RequestInterrupt(SessionId),
    RequestGlobalStop,
    Audit(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiState {
    pub navigation: AppState,
    pub sessions: Vec<SessionView>,
    pub focus_index: usize,
    pub approval_session: Option<SessionId>,
    pub global_stop_committed: bool,
    pub should_quit: bool,
    pub notice: String,
    pub audit_count: usize,
}

impl TuiState {
    pub fn new(mut sessions: Vec<SessionView>) -> Self {
        sessions.sort_by_key(|session| state_priority(session.state));
        Self {
            navigation: AppState::default(),
            sessions,
            focus_index: 0,
            approval_session: None,
            global_stop_committed: false,
            should_quit: false,
            notice: "Fixture event stream ready".to_owned(),
            audit_count: 148,
        }
    }

    pub fn selected_session(&self) -> Option<&SessionView> {
        self.sessions.get(self.focus_index)
    }

    pub fn active_session(&self) -> Option<&SessionView> {
        let id = active_session_id(&self.navigation)?;
        self.sessions.iter().find(|session| &session.id == id)
    }

    pub fn approval(&self) -> Option<(&SessionView, &PendingAction)> {
        let id = self.approval_session.as_ref()?;
        let session = self.sessions.iter().find(|session| &session.id == id)?;
        Some((session, session.pending_action.as_ref()?))
    }

    pub fn apply(&mut self, command: &Command) -> Vec<TuiEffect> {
        match command {
            Command::Quit => {
                self.should_quit = true;
                Vec::new()
            }
            Command::RejectPending => self.reject_pending(),
            Command::Semantic(action) => self.apply_semantic(action),
        }
    }

    fn apply_semantic(&mut self, action: &UiAction) -> Vec<TuiEffect> {
        match action {
            UiAction::Confirm => return self.confirm(),
            UiAction::Inspect => return self.inspect(),
            UiAction::MoveFocus(direction) => {
                self.move_focus(*direction);
                return Vec::new();
            }
            UiAction::Back if self.approval_session.is_some() => {
                self.approval_session = None;
                "Approval left pending".clone_into(&mut self.notice);
                return Vec::new();
            }
            UiAction::OpenEmergencyOverview => {
                self.global_stop_committed = false;
            }
            UiAction::ShowTestFailures => {
                self.open_active_view(AgentView::Tests);
                return Vec::new();
            }
            UiAction::ShowChangedFiles | UiAction::ReviewFullPatch => {
                self.open_active_view(AgentView::Diff);
                return Vec::new();
            }
            UiAction::ApproveAndContinue => return self.approve_pending(),
            UiAction::Scroll(_)
            | UiAction::FastScroll(_)
            | UiAction::ToggleFollowOutput
            | UiAction::PushToTalk
            | UiAction::Back
            | UiAction::OpenCommandPalette
            | UiAction::PreviousAgent
            | UiAction::NextAgent
            | UiAction::PreviousView
            | UiAction::NextView
            | UiAction::OpenDashboard
            | UiAction::BookmarkState
            | UiAction::InterruptCurrentAgent
            | UiAction::StopAllAgents
            | UiAction::OpenRawTerminal => {}
        }

        let leaving_emergency =
            matches!(action, UiAction::Back) && self.is_screen(&NavigationScreen::GlobalEmergency);
        let transition = reduce_navigation(self.navigation.clone(), action);
        self.navigation = transition.state;
        if leaving_emergency {
            self.global_stop_committed = false;
        }

        transition
            .effects
            .into_iter()
            .flat_map(|effect| self.handle_navigation_effect(effect))
            .collect()
    }

    fn confirm(&mut self) -> Vec<TuiEffect> {
        if self.approval_session.is_some() {
            return self.approve_pending();
        }

        match &self.navigation.screen {
            NavigationScreen::MissionControl => {
                let Some((session_id, session_name)) = self
                    .selected_session()
                    .map(|session| (session.id.clone(), session.name.clone()))
                else {
                    return Vec::new();
                };
                self.navigation = AppState::open_agent(session_id);
                self.notice = format!("Opened {session_name}");
                Vec::new()
            }
            NavigationScreen::GlobalEmergency if !self.global_stop_committed => {
                self.apply_semantic(&UiAction::StopAllAgents)
            }
            NavigationScreen::CommandPalette => {
                "Command palette actions arrive in a later slice".clone_into(&mut self.notice);
                Vec::new()
            }
            NavigationScreen::Agent { .. }
            | NavigationScreen::TaskComposer { .. }
            | NavigationScreen::RawTerminal { .. }
            | NavigationScreen::GlobalEmergency => Vec::new(),
        }
    }

    fn inspect(&mut self) -> Vec<TuiEffect> {
        let session_id = match &self.navigation.screen {
            NavigationScreen::MissionControl => {
                self.selected_session().map(|session| session.id.clone())
            }
            NavigationScreen::Agent { session_id, .. }
            | NavigationScreen::RawTerminal { session_id } => Some(session_id.clone()),
            NavigationScreen::TaskComposer { session_id } => session_id.clone(),
            NavigationScreen::CommandPalette | NavigationScreen::GlobalEmergency => None,
        };
        let Some(session_id) = session_id else {
            return Vec::new();
        };

        let has_approval = self
            .sessions
            .iter()
            .find(|session| session.id == session_id)
            .is_some_and(|session| session.pending_action.is_some());
        if has_approval {
            self.approval_session = Some(session_id);
            "Inspecting exact pending action".clone_into(&mut self.notice);
        } else {
            self.open_session_view(&session_id, AgentView::Diff);
        }
        Vec::new()
    }

    fn move_focus(&mut self, direction: sidecar_core::FocusDirection) {
        if !self.is_screen(&NavigationScreen::MissionControl) || self.sessions.is_empty() {
            return;
        }

        let columns = 2;
        self.focus_index = match direction {
            sidecar_core::FocusDirection::Left if self.focus_index % columns > 0 => {
                self.focus_index - 1
            }
            sidecar_core::FocusDirection::Right
                if self.focus_index % columns < columns - 1
                    && self.focus_index + 1 < self.sessions.len() =>
            {
                self.focus_index + 1
            }
            sidecar_core::FocusDirection::Up if self.focus_index >= columns => {
                self.focus_index - columns
            }
            sidecar_core::FocusDirection::Down
                if self.focus_index + columns < self.sessions.len() =>
            {
                self.focus_index + columns
            }
            sidecar_core::FocusDirection::Left
            | sidecar_core::FocusDirection::Right
            | sidecar_core::FocusDirection::Up
            | sidecar_core::FocusDirection::Down => self.focus_index,
        };
        if let Some(session) = self.selected_session() {
            self.notice = format!("Selected {}", session.name);
        }
    }

    fn handle_navigation_effect(&mut self, effect: NavigationEffect) -> Vec<TuiEffect> {
        match effect {
            NavigationEffect::SelectPreviousAgent => {
                self.cycle_agent(-1);
                Vec::new()
            }
            NavigationEffect::SelectNextAgent => {
                self.cycle_agent(1);
                Vec::new()
            }
            NavigationEffect::RequestInterrupt(session_id) => self.interrupt(session_id),
            NavigationEffect::RequestGlobalStop => self.global_stop(),
            NavigationEffect::Bookmark(session_id) => {
                self.audit_count = self.audit_count.saturating_add(1);
                "State bookmarked (fixture only)".clone_into(&mut self.notice);
                vec![TuiEffect::Audit(format!(
                    "bookmarked {}",
                    session_id
                        .as_ref()
                        .map_or("mission-control", SessionId::as_str)
                ))]
            }
            NavigationEffect::StartPushToTalk(_) => {
                "Voice input is not available in this slice".clone_into(&mut self.notice);
                Vec::new()
            }
        }
    }

    fn cycle_agent(&mut self, amount: isize) {
        if self.sessions.is_empty() {
            return;
        }

        let length = self.sessions.len();
        self.focus_index = if amount.is_negative() {
            (self.focus_index + length - 1) % length
        } else {
            (self.focus_index + 1) % length
        };

        let Some(session) = self.selected_session() else {
            return;
        };
        let session_id = session.id.clone();
        let session_name = session.name.clone();
        if let NavigationScreen::Agent {
            session_id: active_id,
            ..
        }
        | NavigationScreen::RawTerminal {
            session_id: active_id,
        } = &mut self.navigation.screen
        {
            *active_id = session_id;
        }
        self.notice = format!("Selected {session_name}");
    }

    fn interrupt(&mut self, session_id: SessionId) -> Vec<TuiEffect> {
        let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        else {
            return Vec::new();
        };
        if matches!(
            session.state,
            SessionState::Completed | SessionState::Interrupted | SessionState::Terminated
        ) {
            return Vec::new();
        }

        session.state = SessionState::Interrupted;
        "Interrupted by operator".clone_into(&mut session.current_tool);
        self.audit_count = self.audit_count.saturating_add(1);
        self.notice = format!("{} interrupted (fixture only)", session.name);
        vec![TuiEffect::RequestInterrupt(session_id)]
    }

    fn global_stop(&mut self) -> Vec<TuiEffect> {
        for session in &mut self.sessions {
            if !matches!(
                session.state,
                SessionState::Completed | SessionState::Interrupted | SessionState::Terminated
            ) {
                session.state = SessionState::Interrupted;
                "Stopped by global emergency action".clone_into(&mut session.current_tool);
            }
        }
        self.global_stop_committed = true;
        self.audit_count = self.audit_count.saturating_add(1);
        "Global stop committed (fixture only)".clone_into(&mut self.notice);
        vec![TuiEffect::RequestGlobalStop]
    }

    fn approve_pending(&mut self) -> Vec<TuiEffect> {
        let Some(session_id) = self
            .approval_session
            .clone()
            .or_else(|| self.active_session().map(|session| session.id.clone()))
        else {
            return Vec::new();
        };
        let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        else {
            return Vec::new();
        };
        let Some(action) = session.pending_action.take() else {
            return Vec::new();
        };

        session.state = SessionState::Running;
        session
            .current_tool
            .clone_from(&action.request.action.resolved_target);
        self.approval_session = None;
        self.audit_count = self.audit_count.saturating_add(1);
        self.notice = format!("{} approved once (fixture only)", session.name);
        vec![TuiEffect::Audit(format!(
            "approved {} once",
            action.request.action_id
        ))]
    }

    fn reject_pending(&mut self) -> Vec<TuiEffect> {
        let Some(session_id) = self.approval_session.clone() else {
            return Vec::new();
        };
        let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        else {
            return Vec::new();
        };
        let Some(action) = session.pending_action.take() else {
            return Vec::new();
        };

        session.state = SessionState::Blocked;
        "Action rejected by operator".clone_into(&mut session.current_tool);
        self.approval_session = None;
        self.audit_count = self.audit_count.saturating_add(1);
        self.notice = format!("{} action rejected (fixture only)", session.name);
        vec![TuiEffect::Audit(format!(
            "rejected {}",
            action.request.action_id
        ))]
    }

    fn open_active_view(&mut self, view: AgentView) {
        let Some(session_id) = self
            .active_session()
            .or_else(|| self.selected_session())
            .map(|session| session.id.clone())
        else {
            return;
        };
        self.open_session_view(&session_id, view);
    }

    fn open_session_view(&mut self, session_id: &SessionId, view: AgentView) {
        self.navigation = AppState {
            screen: NavigationScreen::Agent {
                session_id: session_id.clone(),
                view,
            },
            back_stack: vec![NavigationScreen::MissionControl],
        };
        if let Some(index) = self
            .sessions
            .iter()
            .position(|session| &session.id == session_id)
        {
            self.focus_index = index;
        }
    }

    fn is_screen(&self, expected: &NavigationScreen) -> bool {
        std::mem::discriminant(&self.navigation.screen) == std::mem::discriminant(expected)
    }
}

fn active_session_id(navigation: &AppState) -> Option<&SessionId> {
    match &navigation.screen {
        NavigationScreen::Agent { session_id, .. }
        | NavigationScreen::RawTerminal { session_id } => Some(session_id),
        NavigationScreen::TaskComposer { session_id } => session_id.as_ref(),
        NavigationScreen::MissionControl
        | NavigationScreen::CommandPalette
        | NavigationScreen::GlobalEmergency => {
            navigation
                .back_stack
                .iter()
                .rev()
                .find_map(|screen| match screen {
                    NavigationScreen::Agent { session_id, .. }
                    | NavigationScreen::RawTerminal { session_id } => Some(session_id),
                    NavigationScreen::TaskComposer { session_id } => session_id.as_ref(),
                    NavigationScreen::MissionControl
                    | NavigationScreen::CommandPalette
                    | NavigationScreen::GlobalEmergency => None,
                })
        }
    }
}

const fn state_priority(state: SessionState) -> u8 {
    match state {
        SessionState::WaitingForApproval
        | SessionState::WaitingForAnswer
        | SessionState::Blocked => 0,
        SessionState::Failing | SessionState::Failed => 1,
        SessionState::Starting
        | SessionState::Running
        | SessionState::Interrupting
        | SessionState::Resuming => 2,
        SessionState::Completed => 3,
        SessionState::Created
        | SessionState::Interrupted
        | SessionState::Terminating
        | SessionState::Terminated => 4,
    }
}

#[cfg(test)]
mod tests {
    use sidecar_core::{FocusDirection, NavigationScreen, SessionState, UiAction};

    use crate::fixtures::fixture_state;

    use super::{Command, TuiEffect};

    #[test]
    fn focus_moves_as_a_two_column_grid_without_wrapping() {
        let mut state = fixture_state();

        state.apply(&Command::Semantic(UiAction::MoveFocus(
            FocusDirection::Right,
        )));
        assert_eq!(state.focus_index, 1);
        state.apply(&Command::Semantic(UiAction::MoveFocus(
            FocusDirection::Right,
        )));
        assert_eq!(state.focus_index, 1);
        state.apply(&Command::Semantic(UiAction::MoveFocus(
            FocusDirection::Down,
        )));
        assert_eq!(state.focus_index, 3);
        state.apply(&Command::Semantic(UiAction::MoveFocus(
            FocusDirection::Left,
        )));
        assert_eq!(state.focus_index, 2);
    }

    #[test]
    fn focused_session_opens_and_views_cycle_through_core_reducer() {
        let mut state = fixture_state();
        state.apply(&Command::Semantic(UiAction::Confirm));
        assert!(matches!(
            state.navigation.screen,
            NavigationScreen::Agent { .. }
        ));

        state.apply(&Command::Semantic(UiAction::NextView));
        assert!(matches!(
            state.navigation.screen,
            NavigationScreen::Agent {
                view: sidecar_core::AgentView::Plan,
                ..
            }
        ));
    }

    #[test]
    fn inspection_and_confirmation_approve_exact_fixture_action() {
        let mut state = fixture_state();
        state.apply(&Command::Semantic(UiAction::Inspect));
        assert!(state.approval().is_some());

        let effects = state.apply(&Command::Semantic(UiAction::Confirm));
        assert_eq!(
            effects,
            vec![TuiEffect::Audit("approved action-17 once".to_owned())]
        );
        assert!(state.approval().is_none());
        assert!(state.sessions[0].pending_action.is_none());
        assert_eq!(state.sessions[0].state, SessionState::Running);
    }

    #[test]
    fn emergency_screen_requires_a_separate_confirmation() {
        let mut state = fixture_state();
        let armed = state.apply(&Command::Semantic(UiAction::OpenEmergencyOverview));
        assert!(armed.is_empty());
        assert!(!state.global_stop_committed);
        assert!(
            state
                .sessions
                .iter()
                .any(|session| session.state == SessionState::Running)
        );

        let effects = state.apply(&Command::Semantic(UiAction::Confirm));
        assert_eq!(effects, vec![TuiEffect::RequestGlobalStop]);
        assert!(state.global_stop_committed);
        assert!(state.sessions.iter().all(|session| matches!(
            session.state,
            SessionState::Interrupted | SessionState::Completed
        )));
    }

    #[test]
    fn escape_leaves_approval_pending() {
        let mut state = fixture_state();
        state.apply(&Command::Semantic(UiAction::Inspect));
        state.apply(&Command::Semantic(UiAction::Back));

        assert!(state.approval_session.is_none());
        assert!(state.sessions[0].pending_action.is_some());
    }
}
