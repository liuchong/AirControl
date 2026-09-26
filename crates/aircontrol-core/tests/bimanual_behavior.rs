mod support;

use aircontrol_core::{AssistMode, Button, Command, Engine, Handedness, HandsFrame, Settings};
use support::{
    hand, hands, index_only, left_fist, open_palm, pointer, shifted_palm, three_fingers, thumbs_up,
    unknown_hand, v_sign,
};

#[test]
fn unknown_handedness_never_arms_left_assists() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process_hands(&unknown_hand(0.00, open_palm(0.00)));
    let commands = engine.process_hands(&unknown_hand(0.13, open_palm(0.13)));
    assert_eq!(engine.assist_mode(), AssistMode::None);
    assert!(
        commands
            .iter()
            .all(|command| !matches!(command, Command::AssistChanged { .. }))
    );
}

#[test]
fn open_left_palm_stabilizes_pointer_on_right_palm_instead_of_fingertips() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process_hands(&hands(
        0.00,
        Some(open_palm(0.00)),
        Some(pointer(0.00, 0.44, 0.82)),
    ));
    engine.process_hands(&hands(
        0.13,
        Some(open_palm(0.13)),
        Some(pointer(0.13, 0.44, 0.82)),
    ));
    assert_eq!(engine.assist_mode(), AssistMode::Precision);

    let commands = engine.process_hands(&hands(
        0.20,
        Some(open_palm(0.20)),
        Some(hand(0.20, true, false)),
    ));
    assert!(commands.iter().all(
        |command| !matches!(command, Command::Move { x, y } if x.abs() > 0.001 || y.abs() > 0.001)
    ));
}

#[test]
fn left_index_locks_cursor_but_preserves_right_clicks() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&pointer(0.00, 0.44, 0.82));
    engine.process_hands(&hands(
        0.10,
        Some(index_only(0.10)),
        Some(hand(0.10, false, false)),
    ));
    engine.process_hands(&hands(
        0.23,
        Some(index_only(0.23)),
        Some(hand(0.23, false, false)),
    ));
    assert_eq!(engine.assist_mode(), AssistMode::CursorLock);

    let moved = engine.process_hands(&hands(
        0.28,
        Some(index_only(0.28)),
        Some(pointer(0.28, 0.80, 0.82)),
    ));
    assert!(
        moved
            .iter()
            .all(|command| !matches!(command, Command::Move { .. }))
    );

    engine.process_hands(&hands(
        0.32,
        Some(index_only(0.32)),
        Some(hand(0.32, true, false)),
    ));
    engine.process_hands(&hands(
        0.37,
        Some(index_only(0.37)),
        Some(hand(0.37, true, false)),
    ));
    engine.process_hands(&hands(
        0.45,
        Some(index_only(0.45)),
        Some(hand(0.45, false, false)),
    ));
    let click = engine.process_hands(&hands(
        0.49,
        Some(index_only(0.49)),
        Some(hand(0.49, false, false)),
    ));
    assert!(click.iter().any(|command| matches!(
        command,
        Command::Click {
            button: Button::Left,
            count: 1,
            ..
        }
    )));
}

#[test]
fn left_thumb_up_turns_one_primary_pinch_into_one_explicit_double_click() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&pointer(0.00, 0.44, 0.82));
    engine.process_hands(&hands(
        0.10,
        Some(thumbs_up(0.10)),
        Some(hand(0.10, false, false)),
    ));
    engine.process_hands(&hands(
        0.23,
        Some(thumbs_up(0.23)),
        Some(hand(0.23, false, false)),
    ));
    assert_eq!(engine.assist_mode(), AssistMode::DoubleClick);

    engine.process_hands(&hands(
        0.28,
        Some(thumbs_up(0.28)),
        Some(hand(0.28, true, false)),
    ));
    engine.process_hands(&hands(
        0.33,
        Some(thumbs_up(0.33)),
        Some(hand(0.33, true, false)),
    ));
    engine.process_hands(&hands(
        0.41,
        Some(thumbs_up(0.41)),
        Some(hand(0.41, false, false)),
    ));
    let commands = engine.process_hands(&hands(
        0.45,
        Some(thumbs_up(0.45)),
        Some(hand(0.45, false, false)),
    ));
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(
                command,
                Command::Click {
                    button: Button::Left,
                    count: 2,
                    ..
                }
            ))
            .count(),
        1
    );
    assert!(
        commands
            .iter()
            .all(|command| !matches!(command, Command::MouseDown { .. } | Command::MouseUp { .. }))
    );
}

#[test]
fn left_v_uses_right_palm_for_scroll_without_clicking() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process_hands(&hands(
        0.00,
        Some(v_sign(0.00)),
        Some(hand(0.00, true, false)),
    ));
    engine.process_hands(&hands(
        0.13,
        Some(v_sign(0.13)),
        Some(hand(0.13, true, false)),
    ));
    assert_eq!(engine.assist_mode(), AssistMode::Scroll);

    let right = shifted_palm(hand(0.20, true, false), 0.50, 0.30);
    let commands = engine.process_hands(&hands(0.20, Some(v_sign(0.20)), Some(right)));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::Scroll { .. }))
    );
    assert!(
        commands
            .iter()
            .all(|command| !matches!(command, Command::Click { .. } | Command::MouseDown { .. }))
    );
}

#[test]
fn left_three_fingers_drag_and_either_hand_loss_releases_once() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process_hands(&hands(
        0.00,
        Some(three_fingers(0.00)),
        Some(hand(0.00, false, false)),
    ));
    let start = engine.process_hands(&hands(
        0.13,
        Some(three_fingers(0.13)),
        Some(hand(0.13, false, false)),
    ));
    assert_eq!(engine.assist_mode(), AssistMode::Drag);
    assert_eq!(
        start
            .iter()
            .filter(|command| matches!(
                command,
                Command::MouseDown {
                    button: Button::Left,
                    ..
                }
            ))
            .count(),
        1
    );

    let release = engine.process_hands(
        &HandsFrame::new(0.35).with_hand(Handedness::Right, hand(0.35, false, false)),
    );
    assert_eq!(
        release
            .iter()
            .filter(|command| matches!(
                command,
                Command::MouseUp {
                    button: Button::Left,
                    ..
                }
            ))
            .count(),
        1
    );
    let again = engine.process_hands(
        &HandsFrame::new(0.40).with_hand(Handedness::Right, hand(0.40, false, false)),
    );
    assert!(
        again
            .iter()
            .all(|command| !matches!(command, Command::MouseUp { .. }))
    );
}

#[test]
fn only_left_fist_can_pause_and_stop_clears_pause_for_next_session() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process_hands(&hands(0.00, None, Some(left_fist(0.00))));
    let right_fist = engine.process_hands(&hands(0.71, None, Some(left_fist(0.71))));
    assert!(
        right_fist
            .iter()
            .all(|command| !matches!(command, Command::PauseChanged { .. }))
    );

    engine.process_hands(&hands(
        1.00,
        Some(left_fist(1.00)),
        Some(pointer(1.00, 0.44, 0.82)),
    ));
    let paused = engine.process_hands(&hands(
        1.71,
        Some(left_fist(1.71)),
        Some(pointer(1.71, 0.44, 0.82)),
    ));
    assert!(paused.contains(&Command::PauseChanged { paused: true }));
    engine.stop();

    engine.process(&pointer(2.00, 0.44, 0.82));
    let next_session = engine.process(&pointer(2.12, 0.70, 0.82));
    assert!(
        next_session
            .iter()
            .any(|command| matches!(command, Command::Move { .. }))
    );
}
