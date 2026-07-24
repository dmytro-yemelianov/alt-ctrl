use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use sidecar_core::{FocusDirection, UiAction};

use crate::model::Command;

pub fn map_key_event(event: KeyEvent) -> Option<Command> {
    if matches!(event.kind, KeyEventKind::Release) {
        return None;
    }
    if event.code == KeyCode::Char('c') && event.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Command::Quit);
    }

    let action = match event.code {
        KeyCode::Up => UiAction::MoveFocus(FocusDirection::Up),
        KeyCode::Down => UiAction::MoveFocus(FocusDirection::Down),
        KeyCode::Left => UiAction::MoveFocus(FocusDirection::Left),
        KeyCode::Right => UiAction::MoveFocus(FocusDirection::Right),
        KeyCode::Enter => UiAction::Confirm,
        KeyCode::Esc => UiAction::Back,
        KeyCode::Tab => UiAction::NextView,
        KeyCode::BackTab => UiAction::PreviousView,
        KeyCode::Char(character) => {
            return map_character(character.to_ascii_lowercase());
        }
        KeyCode::Backspace
        | KeyCode::Home
        | KeyCode::End
        | KeyCode::PageUp
        | KeyCode::PageDown
        | KeyCode::Insert
        | KeyCode::Delete
        | KeyCode::F(_)
        | KeyCode::Null
        | KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::KeypadBegin
        | KeyCode::Media(_)
        | KeyCode::Modifier(_) => return None,
    };
    Some(Command::Semantic(action))
}

fn map_character(character: char) -> Option<Command> {
    let action = match character {
        'x' => return Some(Command::Quit),
        'r' => return Some(Command::RejectPending),
        's' => UiAction::Inspect,
        't' => UiAction::OpenCommandPalette,
        'q' => UiAction::PreviousAgent,
        'e' => UiAction::NextAgent,
        'z' => UiAction::PreviousView,
        'c' => UiAction::NextView,
        'm' => UiAction::OpenDashboard,
        'i' => UiAction::InterruptCurrentAgent,
        'g' => UiAction::OpenEmergencyOverview,
        'b' => UiAction::BookmarkState,
        'u' => UiAction::OpenRawTerminal,
        _ => return None,
    };
    Some(Command::Semantic(action))
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use sidecar_core::{FocusDirection, UiAction};

    use super::map_key_event;
    use crate::model::Command;

    #[test]
    fn arrows_map_to_semantic_focus_actions() {
        assert_eq!(
            map_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            Some(Command::Semantic(UiAction::MoveFocus(
                FocusDirection::Right
            )))
        );
    }

    #[test]
    fn controller_style_keys_map_without_reaching_the_renderer_model() {
        assert_eq!(
            map_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
            Some(Command::Semantic(UiAction::Inspect))
        );
        assert_eq!(
            map_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)),
            Some(Command::Semantic(UiAction::OpenEmergencyOverview))
        );
    }

    #[test]
    fn control_c_and_x_quit() {
        assert_eq!(
            map_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Command::Quit)
        );
        assert_eq!(
            map_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            Some(Command::Quit)
        );
    }
}
