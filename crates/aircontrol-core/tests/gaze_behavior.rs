mod support;

use aircontrol_core::{
    Command, Engine, GazeCalibrationTracker, GazeCalibrationUpdate, GazeMapper, GazeSample,
    ScreenRect, Settings,
};
use support::pointer;

fn sample_for_target(stage: usize) -> GazeSample {
    let x = [0.32, 0.50, 0.68][stage % 3];
    let y = [0.34, 0.50, 0.66][stage / 3];
    GazeSample::new(x, y, 0.95, true)
}

fn completed_profile() -> aircontrol_core::GazeProfile {
    let mut tracker = GazeCalibrationTracker::new(0.55, 0.65, 0.02);
    for stage in 0..9 {
        let sample = sample_for_target(stage);
        let base = stage as f64;
        assert!(matches!(
            tracker.update(base, Some(sample)),
            GazeCalibrationUpdate::Progress { stage: actual, .. } if actual == stage
        ));
        assert!(matches!(
            tracker.update(base + 0.30, Some(sample)),
            GazeCalibrationUpdate::Progress { stage: actual, .. } if actual == stage
        ));
        let update = tracker.update(base + 0.66, Some(sample));
        if stage < 8 {
            assert_eq!(update, GazeCalibrationUpdate::TargetCaptured { stage });
        } else if let GazeCalibrationUpdate::Completed(profile) = update {
            return profile;
        } else {
            panic!("expected completed profile, got {update:?}");
        }
    }
    unreachable!()
}

#[test]
fn nine_stable_targets_produce_a_profile_and_map_to_the_screen() {
    let profile = completed_profile();
    let screen = ScreenRect {
        origin_x: -1920.0,
        origin_y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    let mut mapper = GazeMapper::new(profile, screen, 0.55, 0.35).unwrap();

    let top_left = mapper
        .update(0.0, Some(sample_for_target(0)))
        .expect("valid gaze point");
    assert!((top_left.0 - (-1920.0 + 0.12 * 1920.0)).abs() < 2.0);
    assert!((top_left.1 - 0.12 * 1080.0).abs() < 2.0);

    mapper.reset();
    let bottom_right = mapper
        .update(1.0, Some(sample_for_target(8)))
        .expect("valid gaze point");
    assert!((bottom_right.0 - (-1920.0 + 0.88 * 1920.0)).abs() < 2.0);
    assert!((bottom_right.1 - 0.88 * 1080.0).abs() < 2.0);
}

#[test]
fn blink_low_confidence_and_jitter_never_advance_or_move() {
    let mut tracker = GazeCalibrationTracker::new(0.55, 0.65, 0.02);
    let valid = sample_for_target(0);
    let blink = GazeSample::new(valid.x, valid.y, 0.95, false);
    let low = GazeSample::new(valid.x, valid.y, 0.20, true);

    _ = tracker.update(0.00, Some(valid));
    assert!(matches!(
        tracker.update(0.70, Some(blink)),
        GazeCalibrationUpdate::Progress {
            stage: 0,
            progress: 0.0
        }
    ));
    assert!(matches!(
        tracker.update(1.40, Some(low)),
        GazeCalibrationUpdate::Progress {
            stage: 0,
            progress: 0.0
        }
    ));
    _ = tracker.update(2.00, Some(valid));
    let jitter = GazeSample::new(valid.x + 0.08, valid.y, 0.95, true);
    assert!(matches!(
        tracker.update(2.70, Some(jitter)),
        GazeCalibrationUpdate::Progress {
            stage: 0,
            progress: 0.0
        }
    ));

    let profile = completed_profile();
    let mut mapper = GazeMapper::new(
        profile,
        ScreenRect {
            origin_x: 0.0,
            origin_y: 0.0,
            width: 1000.0,
            height: 800.0,
        },
        0.55,
        0.35,
    )
    .unwrap();
    assert_eq!(mapper.update(0.0, Some(blink)), None);
    assert_eq!(mapper.update(0.1, Some(low)), None);
    assert_eq!(mapper.update(0.2, None), None);
}

#[test]
fn external_gaze_anchor_hands_off_to_relative_pointer_without_a_jump() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.rebase_pointer(640.0, 360.0).unwrap();

    let first = engine.process(&pointer(0.00, 0.30, 0.80));
    let first_move = first.iter().find_map(|command| match command {
        Command::Move { x, y } => Some((*x, *y)),
        _ => None,
    });
    assert_eq!(first_move, Some((640.0, 360.0)));

    let second = engine.process(&pointer(0.10, 0.34, 0.80));
    let second_move = second.iter().find_map(|command| match command {
        Command::Move { x, y } => Some((*x, *y)),
        _ => None,
    });
    let (x, y) = second_move.expect("relative motion after handoff");
    assert!(
        x < 640.0 && x > 600.0,
        "camera x remains mirrored at precision gain"
    );
    assert!((y - 360.0).abs() < 1.0);

    engine.clear_pointer_rebase();
    let absolute = engine.process(&pointer(0.20, 0.34, 0.80));
    assert!(
        absolute
            .iter()
            .any(|command| matches!(command, Command::Move { .. }))
    );
}
