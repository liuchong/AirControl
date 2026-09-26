use aircontrol_core::{
    Calibration, CalibrationStage, CalibrationTracker, CalibrationUpdate, Point, ScreenRect,
    Settings, SettingsError,
};

#[test]
fn default_settings_are_safe_and_valid() {
    let settings = Settings::default();
    assert_eq!(settings.version, 1);
    assert_eq!(settings.validate(), Ok(()));
    assert!(settings.pinch_exit > settings.pinch_enter);
}

#[test]
fn invalid_hysteresis_is_rejected() {
    let defaults = Settings::default();
    let settings = Settings {
        pinch_exit: defaults.pinch_enter,
        ..defaults
    };
    assert_eq!(
        settings.validate(),
        Err(SettingsError::InvalidPinchHysteresis)
    );
}

#[test]
fn non_finite_setting_is_rejected() {
    let settings = Settings {
        smoothing: f64::NAN,
        ..Settings::default()
    };
    assert_eq!(settings.validate(), Err(SettingsError::NonFinite));
}

#[test]
fn calibration_rejects_reversed_and_tiny_ranges() {
    assert!(
        Calibration::from_corners(Point::new(0.20, 0.80, 1.0), Point::new(0.80, 0.20, 1.0),)
            .is_some()
    );
    assert!(
        Calibration::from_corners(Point::new(0.80, 0.80, 1.0), Point::new(0.20, 0.20, 1.0),)
            .is_none()
    );
    assert!(
        Calibration::from_corners(Point::new(0.20, 0.40, 1.0), Point::new(0.40, 0.25, 1.0),)
            .is_none()
    );
}

#[test]
fn mapping_mirrors_camera_x_and_converts_y_to_screen_coordinates() {
    let calibration = Calibration {
        min_x: 0.1,
        max_x: 0.9,
        min_y: 0.1,
        max_y: 0.9,
    };
    let screen = ScreenRect {
        origin_x: 100.0,
        origin_y: 200.0,
        width: 1000.0,
        height: 500.0,
    };
    let (x, y) = calibration.map_mirrored(Point::new(0.1, 0.9, 1.0), screen);
    assert!((x - 1100.0).abs() < 0.001);
    assert!((y - 700.0).abs() < 0.001);

    let (x, y) = calibration.map_mirrored(Point::new(1.0, 0.0, 1.0), screen);
    assert!((x - 100.0).abs() < 0.001);
    assert!((y - 200.0).abs() < 0.001);
}

#[test]
fn calibration_tracker_requires_stable_holds_and_returns_valid_region() {
    let mut tracker = CalibrationTracker::new(0.8, 0.025);
    assert!(matches!(
        tracker.update(0.0, Some(Point::new(0.2, 0.8, 1.0))),
        CalibrationUpdate::Progress {
            stage: CalibrationStage::TopLeft,
            ..
        }
    ));
    assert!(matches!(
        tracker.update(0.4, Some(Point::new(0.205, 0.795, 1.0))),
        CalibrationUpdate::Progress { .. }
    ));
    assert_eq!(
        tracker.update(0.81, Some(Point::new(0.2, 0.8, 1.0))),
        CalibrationUpdate::CornerCaptured(CalibrationStage::TopLeft)
    );
    tracker.update(1.0, Some(Point::new(0.8, 0.2, 1.0)));
    let result = tracker.update(1.81, Some(Point::new(0.795, 0.205, 1.0)));
    let CalibrationUpdate::Completed(calibration) = result else {
        panic!("expected completion")
    };
    assert!(calibration.is_valid());
}

#[test]
fn unstable_calibration_sample_restarts_hold_timer() {
    let mut tracker = CalibrationTracker::new(0.8, 0.025);
    tracker.update(0.0, Some(Point::new(0.2, 0.8, 1.0)));
    tracker.update(0.6, Some(Point::new(0.4, 0.6, 1.0)));
    assert!(matches!(
        tracker.update(0.9, Some(Point::new(0.4, 0.6, 1.0))),
        CalibrationUpdate::Progress { progress, .. } if progress < 0.5
    ));
}
