//! Pure controller-button recognition.
//!
//! Hardware backends normalize their events into [`ButtonEvent`]. This crate
//! owns mapping, chord windows, hold timing, suppression, and cooldown without
//! reading a system clock or touching a controller device.

mod analog;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use sidecar_core::{FocusDirection, MonotonicMillis, UiAction};
use thiserror::Error;

pub use analog::{
    AnalogConfigError, Axis2d, DigitalState, HysteresisButton, HysteresisThresholds, RadialDeadzone,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Cross,
    Circle,
    Square,
    Triangle,
    L1,
    R1,
    L2,
    R2,
    Options,
    Share,
    Ps,
    L3,
    R3,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonPhase {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ButtonEvent {
    pub button: Button,
    pub phase: ButtonPhase,
    pub at: MonotonicMillis,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChordActivation {
    Immediate,
    Hold { duration_ms: u64 },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChordDefinition {
    pub buttons: BTreeSet<Button>,
    pub action: UiAction,
    pub activation: ChordActivation,
    pub cooldown_ms: u64,
}

impl ChordDefinition {
    pub fn immediate(buttons: impl IntoIterator<Item = Button>, action: UiAction) -> Self {
        Self {
            buttons: buttons.into_iter().collect(),
            action,
            activation: ChordActivation::Immediate,
            cooldown_ms: 0,
        }
    }

    pub fn hold(
        buttons: impl IntoIterator<Item = Button>,
        action: UiAction,
        duration_ms: u64,
        cooldown_ms: u64,
    ) -> Self {
        Self {
            buttons: buttons.into_iter().collect(),
            action,
            activation: ChordActivation::Hold { duration_ms },
            cooldown_ms,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InputConfig {
    pub chord_window_ms: u64,
    pub simple_mappings: BTreeMap<Button, UiAction>,
    pub chords: Vec<ChordDefinition>,
}

impl InputConfig {
    pub fn playstation_default() -> Self {
        let simple_mappings = BTreeMap::from([
            (Button::DpadUp, UiAction::MoveFocus(FocusDirection::Up)),
            (Button::DpadDown, UiAction::MoveFocus(FocusDirection::Down)),
            (Button::DpadLeft, UiAction::MoveFocus(FocusDirection::Left)),
            (
                Button::DpadRight,
                UiAction::MoveFocus(FocusDirection::Right),
            ),
            (Button::Cross, UiAction::Confirm),
            (Button::Circle, UiAction::Back),
            (Button::Square, UiAction::Inspect),
            (Button::Triangle, UiAction::OpenCommandPalette),
            (Button::L1, UiAction::PreviousAgent),
            (Button::R1, UiAction::NextAgent),
            (Button::L2, UiAction::PreviousView),
            (Button::R2, UiAction::NextView),
            (Button::Options, UiAction::OpenDashboard),
            (Button::Share, UiAction::BookmarkState),
            (Button::Ps, UiAction::OpenEmergencyOverview),
            (Button::L3, UiAction::ToggleFollowOutput),
            (Button::R3, UiAction::PushToTalk),
        ]);

        let chords = vec![
            ChordDefinition::immediate([Button::L1, Button::R1], UiAction::OpenDashboard),
            ChordDefinition::immediate([Button::L2, Button::R2], UiAction::InterruptCurrentAgent),
            ChordDefinition::immediate(
                [Button::Options, Button::Triangle],
                UiAction::OpenRawTerminal,
            ),
            ChordDefinition::hold(
                [Button::Options, Button::Circle],
                UiAction::StopAllAgents,
                1_000,
                300,
            ),
            ChordDefinition::immediate([Button::L1, Button::Cross], UiAction::ApproveAndContinue),
            ChordDefinition::immediate([Button::R1, Button::Square], UiAction::ReviewFullPatch),
            ChordDefinition::immediate([Button::L2, Button::Square], UiAction::ShowTestFailures),
            ChordDefinition::immediate([Button::R2, Button::Square], UiAction::ShowChangedFiles),
        ];

        Self {
            chord_window_ms: 150,
            simple_mappings,
            chords,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecognitionEvent {
    Action(UiAction),
    HoldStarted {
        action: UiAction,
        duration_ms: u64,
    },
    HoldProgress {
        action: UiAction,
        elapsed_ms: u64,
        duration_ms: u64,
    },
    HoldCancelled {
        action: UiAction,
    },
}

#[derive(Clone, Copy, Debug)]
struct PressState {
    pressed_at: MonotonicMillis,
    suppressed: bool,
    simple_emitted: bool,
}

#[derive(Clone, Debug)]
struct ActiveHold {
    chord_index: usize,
    action: UiAction,
    started_at: MonotonicMillis,
    duration_ms: u64,
    cooldown_ms: u64,
}

#[derive(Debug)]
pub struct InputRecognizer {
    config: InputConfig,
    pressed: BTreeMap<Button, PressState>,
    latched_chords: BTreeSet<usize>,
    active_hold: Option<ActiveHold>,
    cooldowns: HashMap<UiAction, MonotonicMillis>,
    last_timestamp: Option<MonotonicMillis>,
}

impl InputRecognizer {
    /// Builds a recognizer after validating chord definitions.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a chord has fewer than two distinct
    /// buttons or a hold has a zero duration.
    pub fn new(config: InputConfig) -> Result<Self, ConfigError> {
        for (index, chord) in config.chords.iter().enumerate() {
            if chord.buttons.len() < 2 {
                return Err(ConfigError::ChordNeedsTwoButtons { index });
            }
            if matches!(chord.activation, ChordActivation::Hold { duration_ms: 0 }) {
                return Err(ConfigError::HoldDurationIsZero { index });
            }
        }

        Ok(Self {
            config,
            pressed: BTreeMap::new(),
            latched_chords: BTreeSet::new(),
            active_hold: None,
            cooldowns: HashMap::new(),
            last_timestamp: None,
        })
    }

    /// Processes one normalized hardware event.
    ///
    /// # Errors
    ///
    /// Returns [`RecognitionError::NonMonotonicTimestamp`] when `event.at` is
    /// earlier than a previously observed timestamp.
    pub fn handle(
        &mut self,
        event: ButtonEvent,
    ) -> Result<Vec<RecognitionEvent>, RecognitionError> {
        let mut output = self.advance_internal(event.at)?;

        match event.phase {
            ButtonPhase::Pressed => {
                if self.pressed.contains_key(&event.button) {
                    return Ok(output);
                }
                self.pressed.insert(
                    event.button,
                    PressState {
                        pressed_at: event.at,
                        suppressed: false,
                        simple_emitted: false,
                    },
                );
                output.extend(self.try_recognize_chord(event.at));
            }
            ButtonPhase::Released => {
                if let Some(hold) = &self.active_hold {
                    if self.config.chords[hold.chord_index]
                        .buttons
                        .contains(&event.button)
                    {
                        output.push(RecognitionEvent::HoldCancelled {
                            action: hold.action.clone(),
                        });
                        self.active_hold = None;
                    }
                }

                if let Some(press) = self.pressed.remove(&event.button) {
                    if !press.suppressed && !press.simple_emitted {
                        if let Some(action) = self.config.simple_mappings.get(&event.button) {
                            output.push(RecognitionEvent::Action(action.clone()));
                        }
                    }
                }

                for (index, chord) in self.config.chords.iter().enumerate() {
                    if chord.buttons.contains(&event.button) {
                        self.latched_chords.remove(&index);
                    }
                }
            }
        }

        Ok(output)
    }

    /// Advances timers without requiring a hardware event.
    ///
    /// # Errors
    ///
    /// Returns [`RecognitionError::NonMonotonicTimestamp`] when `now` is
    /// earlier than a previously observed timestamp.
    pub fn advance(
        &mut self,
        now: MonotonicMillis,
    ) -> Result<Vec<RecognitionEvent>, RecognitionError> {
        self.advance_internal(now)
    }

    fn advance_internal(
        &mut self,
        now: MonotonicMillis,
    ) -> Result<Vec<RecognitionEvent>, RecognitionError> {
        self.ensure_monotonic(now)?;
        let mut output = Vec::new();

        let expired_buttons: Vec<Button> = self
            .pressed
            .iter()
            .filter_map(|(button, state)| {
                (!state.suppressed
                    && !state.simple_emitted
                    && now.saturating_duration_since(state.pressed_at)
                        > self.config.chord_window_ms)
                    .then_some(*button)
            })
            .collect();

        for button in expired_buttons {
            if let Some(state) = self.pressed.get_mut(&button) {
                state.simple_emitted = true;
            }
            if let Some(action) = self.config.simple_mappings.get(&button) {
                output.push(RecognitionEvent::Action(action.clone()));
            }
        }

        if let Some(hold) = self.active_hold.clone() {
            let elapsed_ms = now
                .saturating_duration_since(hold.started_at)
                .min(hold.duration_ms);
            if elapsed_ms > 0 {
                output.push(RecognitionEvent::HoldProgress {
                    action: hold.action.clone(),
                    elapsed_ms,
                    duration_ms: hold.duration_ms,
                });
            }
            if elapsed_ms == hold.duration_ms {
                output.push(RecognitionEvent::Action(hold.action.clone()));
                if hold.cooldown_ms > 0 {
                    self.cooldowns
                        .insert(hold.action, now.saturating_add(hold.cooldown_ms));
                }
                self.active_hold = None;
            }
        }

        Ok(output)
    }

    fn ensure_monotonic(&mut self, now: MonotonicMillis) -> Result<(), RecognitionError> {
        if let Some(previous) = self.last_timestamp {
            if now < previous {
                return Err(RecognitionError::NonMonotonicTimestamp {
                    previous,
                    current: now,
                });
            }
        }
        self.last_timestamp = Some(now);
        Ok(())
    }

    fn try_recognize_chord(&mut self, now: MonotonicMillis) -> Vec<RecognitionEvent> {
        let candidate = self
            .config
            .chords
            .iter()
            .enumerate()
            .filter(|(index, chord)| {
                !self.latched_chords.contains(index)
                    && chord
                        .buttons
                        .iter()
                        .all(|button| self.pressed.contains_key(button))
                    && self.chord_is_within_window(chord)
            })
            .max_by_key(|(index, chord)| (chord.buttons.len(), usize::MAX - *index))
            .map(|(index, _)| index);

        let Some(index) = candidate else {
            return Vec::new();
        };
        let chord = self.config.chords[index].clone();

        for button in &chord.buttons {
            if let Some(state) = self.pressed.get_mut(button) {
                state.suppressed = true;
            }
        }
        self.latched_chords.insert(index);

        if self
            .cooldowns
            .get(&chord.action)
            .is_some_and(|cooldown_until| now < *cooldown_until)
        {
            return Vec::new();
        }

        match chord.activation {
            ChordActivation::Immediate => {
                if chord.cooldown_ms > 0 {
                    self.cooldowns
                        .insert(chord.action.clone(), now.saturating_add(chord.cooldown_ms));
                }
                vec![RecognitionEvent::Action(chord.action)]
            }
            ChordActivation::Hold { duration_ms } => {
                self.active_hold = Some(ActiveHold {
                    chord_index: index,
                    action: chord.action.clone(),
                    started_at: now,
                    duration_ms,
                    cooldown_ms: chord.cooldown_ms,
                });
                vec![RecognitionEvent::HoldStarted {
                    action: chord.action,
                    duration_ms,
                }]
            }
        }
    }

    fn chord_is_within_window(&self, chord: &ChordDefinition) -> bool {
        let mut times = chord
            .buttons
            .iter()
            .filter_map(|button| self.pressed.get(button).map(|state| state.pressed_at));
        let Some(first) = times.next() else {
            return false;
        };
        let (minimum, maximum) = times.fold((first, first), |(minimum, maximum), time| {
            (minimum.min(time), maximum.max(time))
        });
        maximum.saturating_duration_since(minimum) <= self.config.chord_window_ms
    }
}

impl Default for InputRecognizer {
    fn default() -> Self {
        Self::new(InputConfig::playstation_default())
            .expect("the built-in PlayStation mapping must be valid")
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ConfigError {
    #[error("chord {index} must contain at least two distinct buttons")]
    ChordNeedsTwoButtons { index: usize },
    #[error("hold chord {index} must have a non-zero duration")]
    HoldDurationIsZero { index: usize },
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RecognitionError {
    #[error("input timestamp moved backwards from {previous:?} to {current:?}")]
    NonMonotonicTimestamp {
        previous: MonotonicMillis,
        current: MonotonicMillis,
    },
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use sidecar_core::{MonotonicMillis, UiAction};

    use super::{
        Button, ButtonEvent, ButtonPhase, InputRecognizer, RecognitionError, RecognitionEvent,
    };

    fn event(button: Button, phase: ButtonPhase, at_ms: u64) -> ButtonEvent {
        ButtonEvent {
            button,
            phase,
            at: MonotonicMillis(at_ms),
        }
    }

    #[test]
    fn chord_suppresses_both_individual_actions() {
        let mut recognizer = InputRecognizer::default();

        assert!(
            recognizer
                .handle(event(Button::L2, ButtonPhase::Pressed, 0))
                .expect("valid input")
                .is_empty()
        );
        assert_eq!(
            recognizer
                .handle(event(Button::R2, ButtonPhase::Pressed, 100))
                .expect("valid input"),
            vec![RecognitionEvent::Action(UiAction::InterruptCurrentAgent)]
        );
        assert!(
            recognizer
                .handle(event(Button::L2, ButtonPhase::Released, 110))
                .expect("valid input")
                .is_empty()
        );
        assert!(
            recognizer
                .handle(event(Button::R2, ButtonPhase::Released, 120))
                .expect("valid input")
                .is_empty()
        );
    }

    #[test]
    fn individual_action_is_deferred_until_chord_window_expires() {
        let mut recognizer = InputRecognizer::default();

        assert!(
            recognizer
                .handle(event(Button::Cross, ButtonPhase::Pressed, 0))
                .expect("valid input")
                .is_empty()
        );
        assert!(
            recognizer
                .advance(MonotonicMillis(150))
                .expect("valid time")
                .is_empty()
        );
        assert_eq!(
            recognizer
                .advance(MonotonicMillis(151))
                .expect("valid time"),
            vec![RecognitionEvent::Action(UiAction::Confirm)]
        );
        assert!(
            recognizer
                .handle(event(Button::Cross, ButtonPhase::Released, 160))
                .expect("valid input")
                .is_empty()
        );
    }

    #[test]
    fn buttons_outside_chord_window_keep_individual_meanings() {
        let mut recognizer = InputRecognizer::default();

        recognizer
            .handle(event(Button::L2, ButtonPhase::Pressed, 0))
            .expect("valid input");
        assert_eq!(
            recognizer
                .handle(event(Button::R2, ButtonPhase::Pressed, 151))
                .expect("valid input"),
            vec![RecognitionEvent::Action(UiAction::PreviousView)]
        );
        assert_eq!(
            recognizer
                .advance(MonotonicMillis(302))
                .expect("valid time"),
            vec![RecognitionEvent::Action(UiAction::NextView)]
        );
    }

    #[test]
    fn emergency_hold_reports_progress_and_completes() {
        let mut recognizer = InputRecognizer::default();

        recognizer
            .handle(event(Button::Options, ButtonPhase::Pressed, 0))
            .expect("valid input");
        assert_eq!(
            recognizer
                .handle(event(Button::Circle, ButtonPhase::Pressed, 100))
                .expect("valid input"),
            vec![RecognitionEvent::HoldStarted {
                action: UiAction::StopAllAgents,
                duration_ms: 1_000
            }]
        );
        assert_eq!(
            recognizer
                .advance(MonotonicMillis(1_099))
                .expect("valid time"),
            vec![RecognitionEvent::HoldProgress {
                action: UiAction::StopAllAgents,
                elapsed_ms: 999,
                duration_ms: 1_000
            }]
        );
        assert_eq!(
            recognizer
                .advance(MonotonicMillis(1_100))
                .expect("valid time"),
            vec![
                RecognitionEvent::HoldProgress {
                    action: UiAction::StopAllAgents,
                    elapsed_ms: 1_000,
                    duration_ms: 1_000
                },
                RecognitionEvent::Action(UiAction::StopAllAgents)
            ]
        );
    }

    #[test]
    fn early_release_cancels_hold_without_individual_action() {
        let mut recognizer = InputRecognizer::default();

        recognizer
            .handle(event(Button::Options, ButtonPhase::Pressed, 0))
            .expect("valid input");
        recognizer
            .handle(event(Button::Circle, ButtonPhase::Pressed, 100))
            .expect("valid input");
        assert_eq!(
            recognizer
                .handle(event(Button::Circle, ButtonPhase::Released, 500))
                .expect("valid input"),
            vec![
                RecognitionEvent::HoldProgress {
                    action: UiAction::StopAllAgents,
                    elapsed_ms: 400,
                    duration_ms: 1_000
                },
                RecognitionEvent::HoldCancelled {
                    action: UiAction::StopAllAgents
                }
            ]
        );
    }

    #[test]
    fn emergency_action_cannot_retrigger_during_cooldown() {
        let mut recognizer = InputRecognizer::default();

        recognizer
            .handle(event(Button::Options, ButtonPhase::Pressed, 0))
            .expect("valid input");
        recognizer
            .handle(event(Button::Circle, ButtonPhase::Pressed, 100))
            .expect("valid input");
        recognizer
            .advance(MonotonicMillis(1_100))
            .expect("valid time");
        recognizer
            .handle(event(Button::Circle, ButtonPhase::Released, 1_110))
            .expect("valid input");
        recognizer
            .handle(event(Button::Options, ButtonPhase::Released, 1_120))
            .expect("valid input");

        assert!(
            recognizer
                .handle(event(Button::Options, ButtonPhase::Pressed, 1_200))
                .expect("valid input")
                .is_empty()
        );
        assert!(
            recognizer
                .handle(event(Button::Circle, ButtonPhase::Pressed, 1_250))
                .expect("valid input")
                .is_empty()
        );
        assert!(
            recognizer
                .advance(MonotonicMillis(1_400))
                .expect("valid time")
                .is_empty()
        );
    }

    #[test]
    fn timestamps_must_be_monotonic() {
        let mut recognizer = InputRecognizer::default();
        recognizer.advance(MonotonicMillis(10)).expect("valid time");

        assert_eq!(
            recognizer.advance(MonotonicMillis(9)),
            Err(RecognitionError::NonMonotonicTimestamp {
                previous: MonotonicMillis(10),
                current: MonotonicMillis(9)
            })
        );
    }

    proptest! {
        #[test]
        fn interrupt_chord_is_order_independent_inside_window(
            start in 0_u64..1_000_000,
            delta in 0_u64..=150,
            l2_first in any::<bool>(),
        ) {
            let mut recognizer = InputRecognizer::default();
            let (first, second) = if l2_first {
                (Button::L2, Button::R2)
            } else {
                (Button::R2, Button::L2)
            };

            let first_output = recognizer
                .handle(event(first, ButtonPhase::Pressed, start))
                .expect("generated timestamp is monotonic");
            let second_output = recognizer
                .handle(event(second, ButtonPhase::Pressed, start + delta))
                .expect("generated timestamp is monotonic");
            let first_release = recognizer
                .handle(event(first, ButtonPhase::Released, start + delta + 1))
                .expect("generated timestamp is monotonic");
            let second_release = recognizer
                .handle(event(second, ButtonPhase::Released, start + delta + 2))
                .expect("generated timestamp is monotonic");

            prop_assert!(first_output.is_empty());
            prop_assert_eq!(
                second_output,
                vec![RecognitionEvent::Action(UiAction::InterruptCurrentAgent)]
            );
            prop_assert!(first_release.is_empty());
            prop_assert!(second_release.is_empty());
        }

        #[test]
        fn out_of_window_shoulders_never_interrupt(
            start in 0_u64..1_000_000,
            delta in 151_u64..1_000,
            l2_first in any::<bool>(),
        ) {
            let mut recognizer = InputRecognizer::default();
            let (first, second) = if l2_first {
                (Button::L2, Button::R2)
            } else {
                (Button::R2, Button::L2)
            };

            let mut output = recognizer
                .handle(event(first, ButtonPhase::Pressed, start))
                .expect("generated timestamp is monotonic");
            output.extend(
                recognizer
                    .handle(event(second, ButtonPhase::Pressed, start + delta))
                    .expect("generated timestamp is monotonic"),
            );
            output.extend(
                recognizer
                    .advance(MonotonicMillis(start + delta + 151))
                    .expect("generated timestamp is monotonic"),
            );

            prop_assert!(!output.contains(
                &RecognitionEvent::Action(UiAction::InterruptCurrentAgent)
            ));
            let expected_first = if l2_first {
                UiAction::PreviousView
            } else {
                UiAction::NextView
            };
            let expected_second = if l2_first {
                UiAction::NextView
            } else {
                UiAction::PreviousView
            };
            prop_assert!(output.contains(&RecognitionEvent::Action(expected_first)));
            prop_assert!(output.contains(&RecognitionEvent::Action(expected_second)));
        }
    }
}
