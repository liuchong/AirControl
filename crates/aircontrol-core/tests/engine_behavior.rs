mod support;

use aircontrol_core::{
    Button, Command, Engine, GestureState, HandFrame, JointKind, Point, Settings,
};
use support::{fist, hand, horizontal_pointer, pointer};

fn left_click(command: &&Command) -> bool {
    matches!(
        command,
        Command::Click {
            button: Button::Left,
            count: 1,
            ..
        }
    )
}

fn right_click(command: &&Command) -> bool {
    matches!(
        command,
        Command::Click {
            button: Button::Right,
            count: 1,
            ..
        }
    )
}

fn left_down(command: &&Command) -> bool {
    matches!(
        command,
        Command::MouseDown {
            button: Button::Left,
            ..
        }
    )
}

fn left_up(command: &&Command) -> bool {
    matches!(
        command,
        Command::MouseUp {
            button: Button::Left,
            ..
        }
    )
}

#[test]
fn pointer_stays_until_a_deliberate_move_then_tracks_from_the_current_cursor() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.rebase_pointer(720.0, 450.0).unwrap();
    engine.clear_pointer_rebase();

    let settled = engine.process(&pointer(0.00, 0.40, 0.70));
    let jitter = engine.process(&pointer(0.05, 0.412, 0.708));
    assert!(
        settled
            .iter()
            .chain(jitter.iter())
            .all(|command| !matches!(command, Command::Move { .. })),
        "appearing and small jitter must not move the cursor"
    );

    let moved = engine.process(&pointer(0.16, 0.62, 0.70));
    let (x, y) = moved
        .iter()
        .find_map(|command| match command {
            Command::Move { x, y } => Some((*x, *y)),
            _ => None,
        })
        .expect("a deliberate index move starts tracking");
    assert!(x < 720.0, "camera x must be mirrored");
    let raised = engine.process(&pointer(0.28, 0.62, 0.82));
    let raised_y = raised
        .iter()
        .find_map(|command| match command {
            Command::Move { y, .. } => Some(*y),
            _ => None,
        })
        .expect("raising the index moves the cursor");
    assert!(
        raised_y > y,
        "a higher camera y is a higher finger, so the quartz cursor must rise"
    );
    assert!(
        x > 200.0 && (y - 450.0).abs() < 80.0,
        "tracking must leave the current cursor instead of jumping to a screen edge, got {x},{y}"
    );
}

#[test]
fn leaning_closer_does_not_speed_the_cursor_up() {
    fn travel(second_mcp_y: f64) -> f64 {
        let mut engine = Engine::new(Settings::default()).unwrap();
        engine.rebase_pointer(720.0, 450.0).unwrap();
        engine.clear_pointer_rebase();
        engine.process(&pointer(0.00, 0.44, 0.62));
        engine.process(&pointer(0.16, 0.44, 0.74));
        let closer = pointer(0.32, 0.44, 0.90).with(
            JointKind::MiddleMcp,
            Point::new(0.53, second_mcp_y, 1.0),
        );
        let moved = engine.process(&closer);
        moved
            .iter()
            .find_map(|command| match command {
                Command::Move { y, .. } => Some((*y - 450.0).abs()),
                _ => None,
            })
            .unwrap_or(0.0)
    }

    let seated = travel(0.44);
    let closer = travel(0.72);
    assert!(
        closer < seated * 0.9,
        "a larger palm is a closer hand and must slow the cursor: closer={closer}, seated={seated}"
    );
}

#[test]
fn pointer_continues_when_an_unrelated_joint_is_low_confidence() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.rebase_pointer(720.0, 450.0).unwrap();
    engine.clear_pointer_rebase();
    engine.process(&pointer(0.0, 0.40, 0.70));
    let commands = engine.process(
        &pointer(0.12, 0.62, 0.70).with(JointKind::RingTip, Point::new(0.60, 0.48, 0.0)),
    );

    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::Move { .. })),
        "an unrelated low-confidence joint must not drop a valid pointer frame"
    );
    assert_eq!(engine.state(), GestureState::Pointer);
}

#[test]
fn smoothing_response_is_stable_across_frame_rates() {
    fn final_x(frame_interval: f64) -> f64 {
        let mut engine = Engine::new(Settings::default()).unwrap();
        engine.rebase_pointer(720.0, 450.0).unwrap();
        engine.clear_pointer_rebase();
        engine.process(&pointer(0.0, 0.80, 0.80));
        let mut timestamp = frame_interval;
        let mut x = 0.0;
        while timestamp <= 0.20 + 1e-9 {
            let commands = engine.process(&pointer(timestamp, 0.20, 0.80));
            x = commands
                .iter()
                .find_map(|command| match command {
                    Command::Move { x, .. } => Some(*x),
                    _ => None,
                })
                .expect("pointer frame must move");
            timestamp += frame_interval;
        }
        x
    }

    let at_15_fps = final_x(1.0 / 15.0);
    let at_30_fps = final_x(1.0 / 30.0);
    assert!(
        (at_15_fps - at_30_fps).abs() <= Settings::default().screen.width * 0.02,
        "time-equivalent trajectories must not depend on frame count: 15fps={at_15_fps}, 30fps={at_30_fps}"
    );
}

#[test]
fn primary_pinch_pending_freezes_cursor_and_clicks_at_anchor() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.rebase_pointer(720.0, 450.0).unwrap();
    engine.clear_pointer_rebase();
    engine.process(&pointer(0.00, 0.44, 0.82));
    let anchor_commands = engine.process(&pointer(0.12, 0.20, 0.82));
    let anchor = anchor_commands
        .iter()
        .find_map(|command| match command {
            Command::Move { x, y } => Some((*x, *y)),
            _ => None,
        })
        .unwrap();

    let first = engine.process(&pinched_at(0.20, 0.54, 0.72));
    let second = engine.process(&pinched_at(0.25, 0.56, 0.70));
    assert!(
        first
            .iter()
            .chain(second.iter())
            .all(|command| !matches!(command, Command::Move { .. })),
        "pinch arming and pending must never move the visible cursor"
    );

    assert!(engine.process(&hand(0.36, false, false)).is_empty());
    let click = engine.process(&hand(0.40, false, false));
    let click_points: Vec<_> = click
        .iter()
        .filter_map(|command| match command {
            Command::Click {
                button: Button::Left,
                x,
                y,
                count: 1,
            } => Some((*x, *y)),
            _ => None,
        })
        .collect();
    assert_eq!(click_points.len(), 1);
    assert!(
        click_points
            .iter()
            .all(|point| (point.0 - anchor.0).abs() < 1e-6 && (point.1 - anchor.1).abs() < 1e-6),
        "click must use the cursor position captured before the fingers close"
    );
}

#[test]
fn brief_missing_thumb_does_not_cancel_primary_pinch() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&pointer(0.00, 0.44, 0.82));
    assert!(engine.process(&pinched_at(0.05, 0.50, 0.75)).is_empty());
    assert!(
        engine
            .process(
                &pinched_at(0.09, 0.51, 0.74).with(JointKind::ThumbTip, Point::new(0.0, 0.0, 0.0))
            )
            .is_empty()
    );
    assert!(engine.process(&pinched_at(0.12, 0.52, 0.73)).is_empty());

    let first_open = engine.process(&hand(0.20, false, false));
    assert!(
        first_open.iter().all(|command| !matches!(
            command,
            Command::Click { .. } | Command::MouseDown { .. } | Command::MouseUp { .. }
        )),
        "one open sample must not terminate a noisy pinch"
    );
    let second_open = engine.process(&hand(0.24, false, false));
    assert_eq!(second_open.iter().filter(left_click).count(), 1);
}

#[test]
fn recent_palm_scale_keeps_pinch_stable_during_wrist_dropout() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&pointer(0.00, 0.44, 0.82));
    assert!(engine.process(&pinched_at(0.05, 0.50, 0.75)).is_empty());
    for timestamp in [0.10, 0.15] {
        let frame =
            pinched_at(timestamp, 0.51, 0.74).with(JointKind::Wrist, Point::new(0.0, 0.0, 0.0));
        assert!(engine.process(&frame).is_empty());
    }

    assert!(engine.process(&hand(0.20, false, false)).is_empty());
    let click = engine.process(&hand(0.24, false, false));
    assert_eq!(click.iter().filter(left_click).count(), 1);
}

#[test]
fn horizontal_pointer_moves_without_being_mistaken_for_a_fist() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    let first = engine.process(&horizontal_pointer(0.0, 0.72));
    let second = engine.process(&horizontal_pointer(0.71, 0.82));

    assert!(
        first
            .iter()
            .chain(second.iter())
            .any(|command| matches!(command, Command::Move { .. })),
        "a rotated pointing hand must still move the cursor"
    );
    assert!(
        first
            .iter()
            .chain(second.iter())
            .all(|command| !matches!(command, Command::PauseChanged { .. })),
        "a rotated pointing hand must never toggle pause"
    );
    assert_eq!(engine.state(), GestureState::Pointer);
}

#[test]
fn short_primary_pinch_emits_exactly_one_click() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&hand(0.00, true, false));
    engine.process(&hand(0.05, true, false));
    assert!(engine.process(&hand(0.20, false, false)).is_empty());
    let commands = engine.process(&hand(0.24, false, false));
    assert_eq!(commands.iter().filter(left_click).count(), 1);
}

#[test]
fn short_secondary_pinch_emits_exactly_one_right_click() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&hand(0.00, false, true));
    engine.process(&hand(0.05, false, true));
    assert!(engine.process(&hand(0.20, false, false)).is_empty());
    let commands = engine.process(&hand(0.24, false, false));
    assert_eq!(commands.iter().filter(right_click).count(), 1);
}

#[test]
fn disabled_primary_click_never_emits_mouse_button_commands() {
    let settings = Settings {
        enabled_gestures: Settings::default().enabled_gestures & !(1 << 1),
        ..Settings::default()
    };
    let mut engine = Engine::new(settings).unwrap();
    engine.process(&hand(0.00, true, false));
    engine.process(&hand(0.05, true, false));
    let commands = engine.process(&hand(0.20, false, false));
    assert!(commands.iter().all(|command| !matches!(
        command,
        Command::Click { .. } | Command::MouseDown { .. } | Command::MouseUp { .. }
    )));
}

#[test]
fn held_primary_pinch_becomes_drag_and_releases_once() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&hand(0.00, true, false));
    engine.process(&hand(0.05, true, false));
    let start = engine.process(&hand(0.36, true, false));
    assert_eq!(start.iter().filter(left_down).count(), 1);
    assert_eq!(engine.state(), GestureState::Dragging);
    assert!(engine.process(&hand(0.50, false, false)).is_empty());
    let release = engine.process(&hand(0.54, false, false));
    assert_eq!(release.iter().filter(left_up).count(), 1);
    assert_eq!(engine.state(), GestureState::Scrolling);
}

#[test]
fn hand_loss_during_drag_releases_button() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&hand(0.00, true, false));
    engine.process(&hand(0.05, true, false));
    engine.process(&hand(0.36, true, false));
    assert!(engine.process(&HandFrame::empty(0.45)).is_empty());
    let release = engine.process(&HandFrame::empty(0.58));
    assert_eq!(release.iter().filter(left_up).count(), 1);
    assert_eq!(engine.state(), GestureState::NoHand);
}

#[test]
fn safe_stop_releases_drag_and_is_idempotent() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&hand(0.00, true, false));
    engine.process(&hand(0.05, true, false));
    engine.process(&hand(0.36, true, false));
    assert_eq!(engine.stop().iter().filter(left_up).count(), 1);
    assert!(engine.stop().is_empty());
}

#[test]
fn two_extended_fingers_scroll_without_clicking() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    engine.process(&hand(0.0, false, false));
    let commands = engine.process(&hand(0.1, false, false).with(
        aircontrol_core::JointKind::MiddleTip,
        aircontrol_core::Point::new(0.55, 0.68, 1.0),
    ));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::Scroll { .. }))
    );
    assert!(!commands.iter().any(|command| matches!(
        command,
        Command::Click { .. } | Command::MouseDown { .. } | Command::MouseUp { .. }
    )));
}

#[test]
fn right_fist_never_toggles_pause() {
    let mut engine = Engine::new(Settings::default()).unwrap();
    let first = engine.process(&fist(0.0));
    let held = engine.process(&fist(0.71));
    assert!(
        first
            .iter()
            .chain(held.iter())
            .all(|command| !matches!(command, Command::PauseChanged { .. }))
    );
}

fn pinched_at(timestamp: f64, x: f64, y: f64) -> HandFrame {
    hand(timestamp, true, false)
        .with(JointKind::IndexTip, Point::new(x, y, 1.0))
        .with(JointKind::ThumbTip, Point::new(x + 0.005, y - 0.005, 1.0))
}
