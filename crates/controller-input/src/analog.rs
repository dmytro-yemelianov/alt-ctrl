//! Pure analog-input normalization.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ButtonPhase;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Axis2d {
    pub x: f32,
    pub y: f32,
}

impl Axis2d {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct RadialDeadzone {
    threshold: f32,
}

impl RadialDeadzone {
    /// Creates a radial deadzone with rescaling outside its threshold.
    ///
    /// # Errors
    ///
    /// Returns [`AnalogConfigError`] unless `threshold` is finite and within
    /// the half-open interval `[0, 1)`.
    pub fn new(threshold: f32) -> Result<Self, AnalogConfigError> {
        if !threshold.is_finite() || !(0.0..1.0).contains(&threshold) {
            return Err(AnalogConfigError::InvalidDeadzone);
        }
        Ok(Self { threshold })
    }

    pub fn threshold(self) -> f32 {
        self.threshold
    }

    /// Applies the deadzone and rescales the remaining radial magnitude to
    /// `[0, 1]`. Non-finite samples fail closed to the neutral position.
    pub fn apply(self, sample: Axis2d) -> Axis2d {
        if !sample.x.is_finite() || !sample.y.is_finite() {
            return Axis2d::ZERO;
        }

        let x = sample.x.clamp(-1.0, 1.0);
        let y = sample.y.clamp(-1.0, 1.0);
        let magnitude = x.hypot(y);
        if magnitude <= self.threshold || magnitude == 0.0 {
            return Axis2d::ZERO;
        }

        let scaled_magnitude =
            ((magnitude.min(1.0) - self.threshold) / (1.0 - self.threshold)).clamp(0.0, 1.0);
        Axis2d {
            x: (x / magnitude) * scaled_magnitude,
            y: (y / magnitude) * scaled_magnitude,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct HysteresisThresholds {
    press: f32,
    release: f32,
}

impl HysteresisThresholds {
    /// Creates thresholds for turning an analog trigger into button phases.
    ///
    /// # Errors
    ///
    /// Returns [`AnalogConfigError`] unless both values are finite and
    /// `0 <= release < press <= 1`.
    pub fn new(press: f32, release: f32) -> Result<Self, AnalogConfigError> {
        if !press.is_finite()
            || !release.is_finite()
            || !(0.0..=1.0).contains(&press)
            || !(0.0..=1.0).contains(&release)
            || release >= press
        {
            return Err(AnalogConfigError::InvalidHysteresis);
        }
        Ok(Self { press, release })
    }

    pub fn press(self) -> f32 {
        self.press
    }

    pub fn release(self) -> f32 {
        self.release
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DigitalState {
    Released,
    Pressed,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct HysteresisButton {
    thresholds: HysteresisThresholds,
    state: DigitalState,
}

impl HysteresisButton {
    pub fn new(thresholds: HysteresisThresholds) -> Self {
        Self {
            thresholds,
            state: DigitalState::Released,
        }
    }

    pub fn state(self) -> DigitalState {
        self.state
    }

    /// Returns a phase only when the digital state changes. Samples between
    /// the thresholds retain the previous state, preventing boundary chatter.
    pub fn update(&mut self, sample: f32) -> Option<ButtonPhase> {
        if !sample.is_finite() {
            return None;
        }

        let sample = sample.clamp(0.0, 1.0);
        match self.state {
            DigitalState::Released if sample >= self.thresholds.press => {
                self.state = DigitalState::Pressed;
                Some(ButtonPhase::Pressed)
            }
            DigitalState::Pressed if sample <= self.thresholds.release => {
                self.state = DigitalState::Released;
                Some(ButtonPhase::Released)
            }
            DigitalState::Released | DigitalState::Pressed => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum AnalogConfigError {
    #[error("radial deadzone must be finite and within [0, 1)")]
    InvalidDeadzone,
    #[error("hysteresis thresholds must be finite and satisfy 0 <= release < press <= 1")]
    InvalidHysteresis,
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{
        AnalogConfigError, Axis2d, DigitalState, HysteresisButton, HysteresisThresholds,
        RadialDeadzone,
    };
    use crate::ButtonPhase;

    #[test]
    fn radial_deadzone_is_zero_inside_and_rescaled_outside() {
        let deadzone = RadialDeadzone::new(0.2).expect("deadzone");

        assert_eq!(deadzone.apply(Axis2d { x: 0.1, y: 0.1 }), Axis2d::ZERO);
        let output = deadzone.apply(Axis2d { x: 0.6, y: 0.0 });
        assert!((output.x - 0.5).abs() < f32::EPSILON);
        assert!(output.y.abs() < f32::EPSILON);
    }

    #[test]
    fn trigger_hysteresis_suppresses_boundary_chatter() {
        let thresholds = HysteresisThresholds::new(0.5, 0.3).expect("thresholds");
        let mut button = HysteresisButton::new(thresholds);

        assert_eq!(button.update(0.49), None);
        assert_eq!(button.update(0.5), Some(ButtonPhase::Pressed));
        assert_eq!(button.state(), DigitalState::Pressed);
        assert_eq!(button.update(0.4), None);
        assert_eq!(button.update(0.31), None);
        assert_eq!(button.update(0.3), Some(ButtonPhase::Released));
        assert_eq!(button.state(), DigitalState::Released);
    }

    #[test]
    fn invalid_analog_configuration_is_rejected() {
        assert_eq!(
            RadialDeadzone::new(1.0),
            Err(AnalogConfigError::InvalidDeadzone)
        );
        assert_eq!(
            HysteresisThresholds::new(0.3, 0.5),
            Err(AnalogConfigError::InvalidHysteresis)
        );
    }

    proptest! {
        #[test]
        fn normalized_axis_is_finite_and_bounded(
            threshold in 0.0_f32..0.99,
            x in -2.0_f32..2.0,
            y in -2.0_f32..2.0,
        ) {
            let output = RadialDeadzone::new(threshold)
                .expect("generated threshold")
                .apply(Axis2d { x, y });
            prop_assert!(output.x.is_finite());
            prop_assert!(output.y.is_finite());
            prop_assert!(output.x.hypot(output.y) <= 1.0 + f32::EPSILON);
        }
    }
}
