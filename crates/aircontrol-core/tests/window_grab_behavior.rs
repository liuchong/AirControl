mod support;

use aircontrol_core::{
    AssistMode, Command, Engine, Handedness, HandsFrame, JointKind, Point, Settings,
};
use support::{fist, hands, open_palm, shifted_palm, three_fingers, unknown_hand};

#[test]
fn isolated_right_fist_never_starts_window_grab_or_pauses() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    let mut commands = Vec::new();
    for timestamp in [0.00, 0.10, 0.30, 0.80] {
        commands.extend(engine.process(&fist(timestamp)));
    }

    assert!(commands.iter().all(|command| !matches!(
        command,
        Command::WindowGrabBegin { .. }
            | Command::WindowMove { .. }
            | Command::WindowGrabEnd
            | Command::PauseChanged { .. }
    )));
}

#[test]
fn stable_open_then_closing_then_fist_starts_once_at_the_frozen_cursor() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    let first = engine.process(&open_palm(0.00));
    let anchor = first
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Move { x, y } => Some((*x, *y)),
            _ => None,
        })
        .expect("open hand should establish a cursor position");
    let armed = engine.process(&open_palm(0.13));
    let armed_anchor = armed
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Move { x, y } => Some((*x, *y)),
            _ => None,
        })
        .unwrap_or(anchor);

    let closing = engine.process(&closing_hand(0.20));
    let first_fist = engine.process(&fist(0.25));
    let second_fist = engine.process(&fist(0.29));
    let all = [closing, first_fist, second_fist].concat();

    assert!(
        all.iter()
            .all(|command| !matches!(command, Command::Move { .. }))
    );
    let starts: Vec<_> = all
        .iter()
        .filter_map(|command| match command {
            Command::WindowGrabBegin { x, y } => Some((*x, *y)),
            _ => None,
        })
        .collect();
    assert_eq!(starts, vec![armed_anchor]);
    assert!(all.iter().all(|command| !matches!(
        command,
        Command::Click { .. } | Command::MouseDown { .. } | Command::MouseUp { .. }
    )));
}

#[test]
fn grabbed_window_follows_palm_and_reopening_ends_once() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    complete_grab(&mut engine);

    let moved = engine.process(&shifted_palm(fist(0.36), 0.53, 0.22));
    assert!(
        moved
            .iter()
            .any(|command| matches!(command, Command::WindowMove { .. }))
    );
    assert!(moved.iter().all(|command| !matches!(
        command,
        Command::Move { .. } | Command::Click { .. } | Command::MouseDown { .. }
    )));

    let first_open = engine.process(&open_palm(0.42));
    let second_open = engine.process(&open_palm(0.47));
    let still_open = engine.process(&open_palm(0.55));
    assert_eq!(
        [first_open, second_open, still_open]
            .concat()
            .iter()
            .filter(|command| matches!(command, Command::WindowGrabEnd))
            .count(),
        1
    );
}

#[test]
fn grip_contraction_does_not_move_the_window_after_grab_confirmation() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&open_palm(0.00));
    engine.process(&open_palm(0.13));
    engine.process(&closing_hand(0.20));
    let contracted_fist = shifted_palm(fist(0.25), 0.53, 0.23);
    engine.process(&contracted_fist);
    let began = engine.process(&shifted_palm(fist(0.29), 0.53, 0.23));
    let anchor = began
        .iter()
        .find_map(|command| match command {
            Command::WindowGrabBegin { x, y } => Some((*x, *y)),
            _ => None,
        })
        .expect("stable fist should begin a grab");

    let steady = engine.process(&shifted_palm(fist(0.35), 0.53, 0.23));
    assert!(steady.iter().all(|command| !matches!(
        command,
        Command::WindowMove { x, y } if (*x, *y) != anchor
    )));
}

#[test]
fn hand_loss_and_stop_release_an_active_window_grab_once() {
    let mut lost_engine = Engine::new(Settings::default()).unwrap();
    complete_grab(&mut lost_engine);
    assert!(
        lost_engine
            .process(&aircontrol_core::HandFrame::empty(0.40))
            .is_empty()
    );
    let lost = lost_engine.process(&aircontrol_core::HandFrame::empty(0.57));
    assert_eq!(lost, vec![Command::WindowGrabEnd]);
    assert!(
        lost_engine
            .process(&aircontrol_core::HandFrame::empty(0.62))
            .is_empty()
    );

    let mut stopped_engine = Engine::new(Settings::default()).unwrap();
    complete_grab(&mut stopped_engine);
    assert_eq!(stopped_engine.stop(), vec![Command::WindowGrabEnd]);
    assert!(stopped_engine.stop().is_empty());
}

#[test]
fn unknown_hand_and_active_left_assist_cannot_arm_window_grab() {
    let mut unknown_engine = Engine::new(Settings::default()).unwrap();
    let sequence = [
        unknown_hand(0.00, open_palm(0.00)),
        unknown_hand(0.13, open_palm(0.13)),
        unknown_hand(0.20, closing_hand(0.20)),
        unknown_hand(0.25, fist(0.25)),
        unknown_hand(0.29, fist(0.29)),
    ];
    let unknown_commands: Vec<_> = sequence
        .iter()
        .flat_map(|frame| unknown_engine.process_hands(frame))
        .collect();
    assert!(unknown_commands.iter().all(|command| !matches!(
        command,
        Command::WindowGrabBegin { .. } | Command::WindowMove { .. }
    )));

    let mut assisted_engine = Engine::new(Settings::default()).unwrap();
    assisted_engine.process_hands(&hands(
        0.00,
        Some(three_fingers(0.00)),
        Some(open_palm(0.00)),
    ));
    assisted_engine.process_hands(&hands(
        0.13,
        Some(three_fingers(0.13)),
        Some(open_palm(0.13)),
    ));
    assert_eq!(assisted_engine.assist_mode(), AssistMode::Drag);
    let assisted_commands = [
        (0.20, closing_hand(0.20)),
        (0.25, fist(0.25)),
        (0.29, fist(0.29)),
    ]
    .into_iter()
    .flat_map(|(timestamp, right)| {
        assisted_engine.process_hands(&hands(
            timestamp,
            Some(three_fingers(timestamp)),
            Some(right),
        ))
    })
    .collect::<Vec<_>>();
    assert!(assisted_commands.iter().all(|command| !matches!(
        command,
        Command::WindowGrabBegin { .. } | Command::WindowMove { .. }
    )));
}

#[test]
fn left_fist_pause_ends_window_grab_before_pause_change() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    complete_grab(&mut engine);
    engine.process_hands(
        &HandsFrame::new(0.40)
            .with_hand(Handedness::Left, support::left_fist(0.40))
            .with_hand(Handedness::Right, fist(0.40)),
    );
    let paused = engine.process_hands(
        &HandsFrame::new(1.11)
            .with_hand(Handedness::Left, support::left_fist(1.11))
            .with_hand(Handedness::Right, fist(1.11)),
    );
    assert_eq!(
        paused,
        vec![
            Command::WindowGrabEnd,
            Command::PauseChanged { paused: true },
        ]
    );
}

#[test]
fn reopening_during_closing_cancels_the_pending_grab() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&open_palm(0.00));
    engine.process(&open_palm(0.13));
    engine.process(&closing_hand(0.20));
    engine.process(&open_palm(0.25));

    let commands = [engine.process(&fist(0.30)), engine.process(&fist(0.35))].concat();
    assert!(
        commands
            .iter()
            .all(|command| !matches!(command, Command::WindowGrabBegin { .. }))
    );
}

#[test]
fn closing_that_takes_too_long_must_be_rearmed_from_an_open_palm() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&open_palm(0.00));
    engine.process(&open_palm(0.13));
    engine.process(&closing_hand(0.20));
    engine.process(&closing_hand(0.86));

    let commands = [engine.process(&fist(0.90)), engine.process(&fist(0.95))].concat();
    assert!(
        commands
            .iter()
            .all(|command| !matches!(command, Command::WindowGrabBegin { .. }))
    );
}

fn complete_grab(engine: &mut Engine) {
    engine.process(&open_palm(0.00));
    engine.process(&open_palm(0.13));
    engine.process(&closing_hand(0.20));
    engine.process(&fist(0.25));
    let commands = engine.process(&fist(0.29));
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::WindowGrabBegin { .. }))
            .count(),
        1
    );
}

fn closing_hand(timestamp: f64) -> aircontrol_core::HandFrame {
    open_palm(timestamp)
        .with(JointKind::IndexTip, Point::new(0.44, 0.55, 1.0))
        .with(JointKind::MiddleTip, Point::new(0.54, 0.53, 1.0))
}
