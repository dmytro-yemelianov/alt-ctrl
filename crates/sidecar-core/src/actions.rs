use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Semantic actions emitted by input services and consumed by reducers.
///
/// Hardware button names do not cross this boundary.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum UiAction {
    MoveFocus(FocusDirection),
    Scroll(ScrollDirection),
    FastScroll(ScrollDirection),
    Confirm,
    Back,
    Inspect,
    OpenCommandPalette,
    PreviousAgent,
    NextAgent,
    PreviousView,
    NextView,
    OpenDashboard,
    BookmarkState,
    OpenEmergencyOverview,
    ToggleFollowOutput,
    PushToTalk,
    InterruptCurrentAgent,
    StopAllAgents,
    OpenRawTerminal,
    ApproveAndContinue,
    ReviewFullPatch,
    ShowTestFailures,
    ShowChangedFiles,
}
