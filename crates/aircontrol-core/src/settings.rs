use crate::geometry::{Calibration, ScreenRect};

pub const GESTURE_POINTER: u32 = 1 << 0;
pub const GESTURE_PRIMARY_CLICK: u32 = 1 << 1;
pub const GESTURE_DRAG: u32 = 1 << 2;
pub const GESTURE_SECONDARY_CLICK: u32 = 1 << 3;
pub const GESTURE_SCROLL: u32 = 1 << 4;
pub const ALL_OPTIONAL_GESTURES: u32 = GESTURE_POINTER
    | GESTURE_PRIMARY_CLICK
    | GESTURE_DRAG
    | GESTURE_SECONDARY_CLICK
    | GESTURE_SCROLL;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub version: u32,
    pub smoothing: f64,
    pub minimum_confidence: f64,
    pub pinch_enter: f64,
    pub pinch_exit: f64,
    pub drag_hold_seconds: f64,
    pub fist_hold_seconds: f64,
    pub hand_loss_seconds: f64,
    pub scroll_gain: f64,
    pub scroll_dead_zone: f64,
    pub calibration: Calibration,
    pub screen: ScreenRect,
    pub enabled_gestures: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsError {
    UnsupportedVersion,
    NonFinite,
    OutOfRange,
    InvalidPinchHysteresis,
    InvalidCalibration,
    InvalidScreen,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            smoothing: 0.32,
            minimum_confidence: 0.35,
            pinch_enter: 0.28,
            pinch_exit: 0.45,
            drag_hold_seconds: 0.35,
            fist_hold_seconds: 0.70,
            hand_loss_seconds: 0.20,
            scroll_gain: 1_200.0,
            scroll_dead_zone: 0.008,
            calibration: Calibration {
                min_x: 0.10,
                max_x: 0.90,
                min_y: 0.10,
                max_y: 0.90,
            },
            screen: ScreenRect {
                origin_x: 0.0,
                origin_y: 0.0,
                width: 1440.0,
                height: 900.0,
            },
            enabled_gestures: ALL_OPTIONAL_GESTURES,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.version != 1 {
            return Err(SettingsError::UnsupportedVersion);
        }
        let values = [
            self.smoothing,
            self.minimum_confidence,
            self.pinch_enter,
            self.pinch_exit,
            self.drag_hold_seconds,
            self.fist_hold_seconds,
            self.hand_loss_seconds,
            self.scroll_gain,
            self.scroll_dead_zone,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(SettingsError::NonFinite);
        }
        if !(0.05..=1.0).contains(&self.smoothing)
            || !(0.0..=1.0).contains(&self.minimum_confidence)
            || !(0.05..=1.5).contains(&self.pinch_enter)
            || !(0.05..=2.0).contains(&self.pinch_exit)
            || !(0.10..=2.0).contains(&self.drag_hold_seconds)
            || !(0.20..=3.0).contains(&self.fist_hold_seconds)
            || !(0.05..=2.0).contains(&self.hand_loss_seconds)
            || !(10.0..=10_000.0).contains(&self.scroll_gain)
            || !(0.0..=0.20).contains(&self.scroll_dead_zone)
        {
            return Err(SettingsError::OutOfRange);
        }
        if self.pinch_exit <= self.pinch_enter {
            return Err(SettingsError::InvalidPinchHysteresis);
        }
        if !self.calibration.is_valid() {
            return Err(SettingsError::InvalidCalibration);
        }
        if !self.screen.is_valid() {
            return Err(SettingsError::InvalidScreen);
        }
        Ok(())
    }
}
