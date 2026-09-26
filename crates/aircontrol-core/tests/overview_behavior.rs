mod support;

use aircontrol_core::{Command, Engine, HandFrame, JointKind, Point, Settings};
use support::{fist, hands, open_palm};

fn palm_at(timestamp: f64, x: f64, y: f64) -> HandFrame {
    open_palm(timestamp).with(JointKind::Wrist, Point::new(x, y, 1.0))
}

fn fist_at(timestamp: f64, x: f64, y: f64) -> HandFrame {
    fist(timestamp).with(JointKind::Wrist, Point::new(x, y, 1.0))
}

#[test]
fn both_open_palms_rising_together_show_the_overview_once() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    let mut commands = Vec::new();
    for (timestamp, y) in [(0.00, 0.20), (0.12, 0.20), (0.40, 0.36)] {
        commands.extend(engine.process_hands(&hands(
            timestamp,
            Some(palm_at(timestamp, 0.20, y)),
            Some(palm_at(timestamp, 0.55, y)),
        )));
    }

    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::ShowAppOverview))
            .count(),
        1
    );

    let held = engine.process_hands(&hands(
        0.55,
        Some(palm_at(0.55, 0.20, 0.36)),
        Some(palm_at(0.55, 0.55, 0.36)),
    ));
    assert!(held.iter().all(|command| !matches!(command, Command::ShowAppOverview)));
}

#[test]
fn overlapped_hands_do_not_turn_a_right_fist_into_a_left_pause() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process_hands(&hands(
        0.00,
        Some(palm_at(0.00, 0.40, 0.30)),
        Some(fist_at(0.00, 0.48, 0.30)),
    ));

    let mut commands = Vec::new();
    for timestamp in [0.20, 0.40, 0.60, 0.85] {
        commands.extend(engine.process_hands(&hands(
            timestamp,
            Some(fist_at(timestamp, 0.48, 0.30)),
            Some(palm_at(timestamp, 0.40, 0.30)),
        )));
    }

    assert!(
        commands
            .iter()
            .all(|command| !matches!(command, Command::PauseChanged { .. }))
    );
}
