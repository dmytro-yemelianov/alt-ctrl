use serde::{Deserialize, Serialize};

use crate::{SessionId, UiAction};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentView {
    Output,
    Plan,
    Diff,
    Tests,
    Approvals,
}

impl AgentView {
    const ORDER: [Self; 5] = [
        Self::Output,
        Self::Plan,
        Self::Diff,
        Self::Tests,
        Self::Approvals,
    ];

    fn next(self) -> Self {
        let index = Self::ORDER
            .iter()
            .position(|candidate| *candidate == self)
            .expect("all view variants must appear in ORDER");
        Self::ORDER[(index + 1) % Self::ORDER.len()]
    }

    fn previous(self) -> Self {
        let index = Self::ORDER
            .iter()
            .position(|candidate| *candidate == self)
            .expect("all view variants must appear in ORDER");
        Self::ORDER[(index + Self::ORDER.len() - 1) % Self::ORDER.len()]
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NavigationScreen {
    MissionControl,
    Agent {
        session_id: SessionId,
        view: AgentView,
    },
    TaskComposer {
        session_id: Option<SessionId>,
    },
    CommandPalette,
    RawTerminal {
        session_id: SessionId,
    },
    GlobalEmergency,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppState {
    pub screen: NavigationScreen,
    #[serde(default)]
    pub back_stack: Vec<NavigationScreen>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            screen: NavigationScreen::MissionControl,
            back_stack: Vec::new(),
        }
    }
}

impl AppState {
    pub fn open_agent(session_id: SessionId) -> Self {
        Self {
            screen: NavigationScreen::Agent {
                session_id,
                view: AgentView::Output,
            },
            back_stack: vec![NavigationScreen::MissionControl],
        }
    }

    fn active_session(&self) -> Option<SessionId> {
        match &self.screen {
            NavigationScreen::Agent { session_id, .. }
            | NavigationScreen::RawTerminal { session_id } => Some(session_id.clone()),
            NavigationScreen::TaskComposer { session_id } => session_id.clone(),
            NavigationScreen::MissionControl
            | NavigationScreen::CommandPalette
            | NavigationScreen::GlobalEmergency => {
                self.back_stack
                    .iter()
                    .rev()
                    .find_map(|screen| match screen {
                        NavigationScreen::Agent { session_id, .. }
                        | NavigationScreen::RawTerminal { session_id } => Some(session_id.clone()),
                        NavigationScreen::TaskComposer { session_id } => session_id.clone(),
                        NavigationScreen::MissionControl
                        | NavigationScreen::CommandPalette
                        | NavigationScreen::GlobalEmergency => None,
                    })
            }
        }
    }

    fn push(&mut self, next: NavigationScreen) {
        if self.screen != next {
            self.back_stack.push(self.screen.clone());
            self.screen = next;
        }
    }

    fn back(&mut self) {
        if let Some(parent) = self.back_stack.pop() {
            self.screen = parent;
        } else if !matches!(self.screen, NavigationScreen::MissionControl) {
            self.screen = NavigationScreen::MissionControl;
        }
    }

    fn dashboard(&mut self) {
        self.screen = NavigationScreen::MissionControl;
        self.back_stack.clear();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NavigationEffect {
    SelectPreviousAgent,
    SelectNextAgent,
    RequestInterrupt(SessionId),
    RequestGlobalStop,
    Bookmark(Option<SessionId>),
    StartPushToTalk(Option<SessionId>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationTransition {
    pub state: AppState,
    pub effects: Vec<NavigationEffect>,
}

/// Pure application-navigation reducer.
///
/// Input services have already resolved chords and holds before this reducer is
/// called. In particular, `StopAllAgents` represents a completed emergency
/// hold, not the initial button press.
pub fn reduce_navigation(mut state: AppState, action: &UiAction) -> NavigationTransition {
    let mut effects = Vec::new();

    match action {
        UiAction::Back => state.back(),
        UiAction::OpenDashboard => state.dashboard(),
        UiAction::OpenCommandPalette => state.push(NavigationScreen::CommandPalette),
        UiAction::OpenEmergencyOverview => state.push(NavigationScreen::GlobalEmergency),
        UiAction::OpenRawTerminal => {
            if let Some(session_id) = state.active_session() {
                state.push(NavigationScreen::RawTerminal { session_id });
            }
        }
        UiAction::PreviousView => {
            if let NavigationScreen::Agent { view, .. } = &mut state.screen {
                *view = view.previous();
            }
        }
        UiAction::NextView => {
            if let NavigationScreen::Agent { view, .. } = &mut state.screen {
                *view = view.next();
            }
        }
        UiAction::PreviousAgent => effects.push(NavigationEffect::SelectPreviousAgent),
        UiAction::NextAgent => effects.push(NavigationEffect::SelectNextAgent),
        UiAction::InterruptCurrentAgent => {
            if let Some(session_id) = state.active_session() {
                effects.push(NavigationEffect::RequestInterrupt(session_id));
            }
        }
        UiAction::StopAllAgents => {
            state.push(NavigationScreen::GlobalEmergency);
            effects.push(NavigationEffect::RequestGlobalStop);
        }
        UiAction::BookmarkState => {
            effects.push(NavigationEffect::Bookmark(state.active_session()));
        }
        UiAction::PushToTalk => {
            effects.push(NavigationEffect::StartPushToTalk(state.active_session()));
        }
        UiAction::MoveFocus(_)
        | UiAction::Scroll(_)
        | UiAction::FastScroll(_)
        | UiAction::Confirm
        | UiAction::Inspect
        | UiAction::ToggleFollowOutput
        | UiAction::ApproveAndContinue
        | UiAction::ReviewFullPatch
        | UiAction::ShowTestFailures
        | UiAction::ShowChangedFiles => {}
    }

    NavigationTransition { state, effects }
}

#[cfg(test)]
mod tests {
    use crate::{SessionId, UiAction};

    use super::{AgentView, AppState, NavigationEffect, NavigationScreen, reduce_navigation};

    fn session_id() -> SessionId {
        SessionId::new("session-01").expect("valid session ID")
    }

    #[test]
    fn back_always_moves_toward_safe_parent() {
        let agent = AppState::open_agent(session_id());
        let palette = reduce_navigation(agent, &UiAction::OpenCommandPalette).state;
        let back_to_agent = reduce_navigation(palette, &UiAction::Back).state;
        assert!(matches!(
            back_to_agent.screen,
            NavigationScreen::Agent { .. }
        ));

        let dashboard = reduce_navigation(back_to_agent, &UiAction::Back).state;
        assert_eq!(dashboard, AppState::default());
        assert!(
            reduce_navigation(dashboard, &UiAction::Back)
                .effects
                .is_empty()
        );
    }

    #[test]
    fn agent_views_cycle_deterministically() {
        let state = AppState::open_agent(session_id());
        let next = reduce_navigation(state, &UiAction::NextView).state;
        assert!(matches!(
            next.screen,
            NavigationScreen::Agent {
                view: AgentView::Plan,
                ..
            }
        ));
        let previous = reduce_navigation(next, &UiAction::PreviousView).state;
        assert!(matches!(
            previous.screen,
            NavigationScreen::Agent {
                view: AgentView::Output,
                ..
            }
        ));
    }

    #[test]
    fn completed_emergency_hold_requests_stop_and_opens_overview() {
        let state = AppState::open_agent(session_id());
        let transition = reduce_navigation(state, &UiAction::StopAllAgents);

        assert_eq!(transition.screen(), &NavigationScreen::GlobalEmergency);
        assert_eq!(
            transition.effects,
            vec![NavigationEffect::RequestGlobalStop]
        );
    }

    #[test]
    fn interrupt_effect_is_bound_to_active_session() {
        let id = session_id();
        let transition = reduce_navigation(
            AppState::open_agent(id.clone()),
            &UiAction::InterruptCurrentAgent,
        );
        assert_eq!(
            transition.effects,
            vec![NavigationEffect::RequestInterrupt(id)]
        );
    }

    impl super::NavigationTransition {
        fn screen(&self) -> &NavigationScreen {
            &self.state.screen
        }
    }
}
