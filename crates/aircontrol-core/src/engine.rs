use crate::{
    HandFrame, Handedness, HandsFrame, JointKind, Point, Settings,
    bimanual::{AssistMode, AssistTracker, AssistTransition},
    cursor_filter::CursorFilter,
    pinch::{PinchAction, PinchEvidence, PinchTracker},
    settings::{
        GESTURE_DRAG, GESTURE_POINTER, GESTURE_PRIMARY_CLICK, GESTURE_SCROLL,
        GESTURE_SECONDARY_CLICK,
    },
    window_grab::{WindowGrabAction, WindowGrabTracker},
    identity::HandIdentity,
    overview::{OverviewTracker, RisingHand},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Move {
        x: f64,
        y: f64,
    },
    Click {
        button: Button,
        x: f64,
        y: f64,
        count: u8,
    },
    MouseDown {
        button: Button,
        x: f64,
        y: f64,
    },
    MouseUp {
        button: Button,
        x: f64,
        y: f64,
    },
    Scroll {
        delta_y: f64,
    },
    PauseChanged {
        paused: bool,
    },
    AssistChanged {
        mode: AssistMode,
    },
    WindowGrabBegin {
        x: f64,
        y: f64,
    },
    WindowMove {
        x: f64,
        y: f64,
    },
    WindowGrabEnd,
    ShowAppOverview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureState {
    NoHand,
    Pointer,
    PrimaryPinchPending,
    Dragging,
    Scrolling,
    Paused,
}

#[derive(Clone)]
pub struct Engine {
    settings: Settings,
    state: GestureState,
    paused: bool,
    left_fist_started_at: Option<f64>,
    left_fist_latched: bool,
    primary_pinch: PinchTracker,
    secondary_pinch: PinchTracker,
    scroll_anchor_y: Option<f64>,
    last_right_seen_at: Option<f64>,
    last_palm_size: Option<(f64, f64)>,
    index_extended_at: Option<f64>,
    cursor_filter: CursorFilter,
    last_cursor: (f64, f64),
    assist_tracker: AssistTracker,
    assist_cursor_filter: CursorFilter,
    assist_anchor_palm: Option<Point>,
    assist_anchor_cursor: (f64, f64),
    assist_scroll_anchor_y: Option<f64>,
    assist_drag_down: bool,
    window_grab: WindowGrabTracker,
    window_cursor_filter: CursorFilter,
    window_anchor_palm: Option<Point>,
    window_anchor_cursor: (f64, f64),
    external_pointer: Option<ExternalPointerHandoff>,
    pointer_anchor: Option<Point>,
    pointer_armed: bool,
    pointer_origin_hand: Option<Point>,
    pointer_origin_cursor: (f64, f64),
    identity: HandIdentity,
    overview: OverviewTracker,
    grab_hold_until_neutral: bool,
}

#[derive(Clone, Copy)]
struct ExternalPointerHandoff {
    cursor_anchor: (f64, f64),
    hand_anchor: Option<Point>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerRebaseError {
    InvalidCoordinate,
}

impl Engine {
    pub fn new(settings: Settings) -> Result<Self, crate::SettingsError> {
        settings.validate()?;
        Ok(Self {
            settings,
            state: GestureState::NoHand,
            paused: false,
            left_fist_started_at: None,
            left_fist_latched: false,
            primary_pinch: PinchTracker::default(),
            secondary_pinch: PinchTracker::default(),
            scroll_anchor_y: None,
            last_right_seen_at: None,
            last_palm_size: None,
            index_extended_at: None,
            cursor_filter: CursorFilter::default(),
            last_cursor: (0.0, 0.0),
            assist_tracker: AssistTracker::default(),
            assist_cursor_filter: CursorFilter::default(),
            assist_anchor_palm: None,
            assist_anchor_cursor: (0.0, 0.0),
            assist_scroll_anchor_y: None,
            assist_drag_down: false,
            window_grab: WindowGrabTracker::default(),
            window_cursor_filter: CursorFilter::default(),
            window_anchor_palm: None,
            window_anchor_cursor: (0.0, 0.0),
            external_pointer: None,
            pointer_anchor: None,
            pointer_armed: false,
            pointer_origin_hand: None,
            pointer_origin_cursor: (0.0, 0.0),
            identity: HandIdentity::default(),
            overview: OverviewTracker::default(),
            grab_hold_until_neutral: false,
        })
    }

    pub fn state(&self) -> GestureState {
        self.state
    }

    pub fn assist_mode(&self) -> AssistMode {
        self.assist_tracker.active()
    }

    pub fn process(&mut self, frame: &HandFrame) -> Vec<Command> {
        let hands = HandsFrame::new(frame.timestamp).with_hand(Handedness::Right, frame.clone());
        self.process_hands(&hands)
    }

    pub fn process_hands(&mut self, frame: &HandsFrame) -> Vec<Command> {
        let stabilized = self.identity.update(frame);
        let frame = &stabilized.frame;
        let timestamp = frame.timestamp;
        let left = frame
            .left()
            .map(|hand| HandPoints::from_frame(hand, self.settings.minimum_confidence));
        let right = frame
            .controlling_hand()
            .map(|hand| HandPoints::from_frame(hand, self.settings.minimum_confidence));

        let left_is_fist = left.is_some_and(|points| points.is_fist() && !points.thumb_up());
        if left_is_fist {
            if let Some(commands) = self.handle_left_fist(timestamp) {
                return commands;
            }
        } else if left.is_some() {
            self.left_fist_started_at = None;
            self.left_fist_latched = false;
        }

        if self.paused {
            self.state = GestureState::Paused;
            return Vec::new();
        }

        let overview = self.overview.update(
            timestamp,
            rising_hand(frame.left(), self.settings.minimum_confidence),
            rising_hand(frame.right(), self.settings.minimum_confidence),
            stabilized.overlapped,
        );
        if overview.show {
            self.grab_hold_until_neutral = true;
            self.window_grab.reset();
            self.reset_window_anchors();
        }
        if overview.suppress {
            return Vec::new();
        }

        let observed_assist = if stabilized.overlapped {
            Some(self.assist_tracker.active())
        } else {
            left.and_then(|points| {
                points.has_signal().then_some(if left_is_fist {
                    AssistMode::None
                } else {
                    points.assist_mode()
                })
            })
        };

        let mut commands = Vec::new();
        if overview.show {
            commands.push(Command::ShowAppOverview);
        }
        if let Some(transition) = self.assist_tracker.update(timestamp, observed_assist) {
            commands.extend(self.apply_assist_transition(transition, right, timestamp));
        }

        let mut mode_commands = match self.assist_tracker.active() {
            AssistMode::None => {
                let grab_allowed = self.controlling_hand_may_grab(frame) && !self.grab_is_held(right);
                let (window_commands, suppress_standard) =
                    self.process_window_grab(right, timestamp, grab_allowed);
                if suppress_standard {
                    window_commands
                } else {
                    let mut commands = window_commands;
                    commands.extend(self.process_standard(right, timestamp, 1));
                    commands
                }
            }
            AssistMode::Precision => self.process_precision(right, timestamp),
            AssistMode::CursorLock => self.process_locked(right, timestamp),
            AssistMode::DoubleClick => self.process_standard(right, timestamp, 2),
            AssistMode::Scroll => self.process_assist_scroll(right, timestamp),
            AssistMode::Drag => self.process_assist_drag(right, timestamp),
        };
        commands.append(&mut mode_commands);
        commands
    }

    pub fn rebase_pointer(&mut self, x: f64, y: f64) -> Result<(), PointerRebaseError> {
        let screen = self.settings.screen;
        if !x.is_finite()
            || !y.is_finite()
            || !(screen.origin_x..=screen.origin_x + screen.width).contains(&x)
            || !(screen.origin_y..=screen.origin_y + screen.height).contains(&y)
        {
            return Err(PointerRebaseError::InvalidCoordinate);
        }
        self.last_cursor = (x, y);
        self.cursor_filter.reset();
        self.external_pointer = Some(ExternalPointerHandoff {
            cursor_anchor: (x, y),
            hand_anchor: None,
        });
        Ok(())
    }

    pub fn clear_pointer_rebase(&mut self) {
        self.external_pointer = None;
        self.cursor_filter.reset();
    }

    pub fn stop(&mut self) -> Vec<Command> {
        let commands = self.release_commands();
        self.reset_right_state();
        self.assist_tracker.clear();
        self.identity.reset();
        self.overview.reset();
        self.grab_hold_until_neutral = false;
        self.paused = false;
        self.left_fist_started_at = None;
        self.left_fist_latched = false;
        self.state = GestureState::NoHand;
        commands
    }

    fn controlling_hand_may_grab(&self, frame: &HandsFrame) -> bool {
        frame.right().is_some()
            || self.window_grab.is_engaged()
            || (frame.left().is_none() && frame.unknown().is_some())
    }

    fn grab_is_held(&mut self, right: Option<HandPoints>) -> bool {
        if !self.grab_hold_until_neutral {
            return false;
        }
        let still_open = right.is_some_and(|points| points.extended_finger_count() == Some(4));
        if !still_open {
            self.grab_hold_until_neutral = false;
        }
        still_open
    }

    fn gesture_enabled(&self, gesture: u32) -> bool {
        self.settings.enabled_gestures & gesture != 0
    }

    fn process_standard(
        &mut self,
        points: Option<HandPoints>,
        timestamp: f64,
        primary_click_count: u8,
    ) -> Vec<Command> {
        let Some(points) = points.filter(|points| points.has_signal()) else {
            return self.handle_missing_controlling_hand(timestamp);
        };
        self.last_right_seen_at = Some(timestamp);

        let palm_size = self.palm_size_for(points, timestamp);
        if self.gesture_enabled(GESTURE_PRIMARY_CLICK) {
            let was_active = self.primary_pinch.is_active();
            let evidence = points.primary_pinch_evidence(
                palm_size,
                if was_active {
                    self.settings.pinch_exit
                } else {
                    self.settings.pinch_enter
                },
            );
            let action = self.primary_pinch.update(
                timestamp,
                evidence,
                self.last_cursor,
                self.gesture_enabled(GESTURE_DRAG),
                self.settings.drag_hold_seconds,
            );
            if was_active || self.primary_pinch.is_active() || action != PinchAction::None {
                return self.handle_primary_pinch(
                    action,
                    evidence,
                    points,
                    timestamp,
                    primary_click_count,
                    true,
                );
            }
        } else {
            self.primary_pinch.reset();
        }

        if self.gesture_enabled(GESTURE_SECONDARY_CLICK) {
            let was_active = self.secondary_pinch.is_active();
            let evidence = points.secondary_pinch_evidence(
                palm_size,
                if was_active {
                    self.settings.pinch_exit
                } else {
                    self.settings.pinch_enter
                },
            );
            let action = self.secondary_pinch.update(
                timestamp,
                evidence,
                self.last_cursor,
                false,
                self.settings.drag_hold_seconds,
            );
            if was_active || self.secondary_pinch.is_active() || action != PinchAction::None {
                return self.handle_secondary_pinch(action, points);
            }
        } else {
            self.secondary_pinch.reset();
        }

        if self.gesture_enabled(GESTURE_SCROLL) && points.is_v_sign() {
            self.state = GestureState::Scrolling;
            let midpoint_y = points.scroll_midpoint_y().expect("validated scroll points");
            let Some(previous) = self.scroll_anchor_y.replace(midpoint_y) else {
                return Vec::new();
            };
            let delta = midpoint_y - previous;
            if delta.abs() >= self.settings.scroll_dead_zone {
                return vec![Command::Scroll {
                    delta_y: delta * self.settings.scroll_gain,
                }];
            }
            return Vec::new();
        }
        self.scroll_anchor_y = None;

        if self.gesture_enabled(GESTURE_POINTER)
            && self.pointer_is_extended(points.index_extended(), timestamp)
            && let Some(index_tip) = points.index_tip
        {
            self.state = GestureState::Pointer;
            return self
                .cursor_for(index_tip, timestamp)
                .map(|cursor| {
                    vec![Command::Move {
                        x: cursor.0,
                        y: cursor.1,
                    }]
                })
                .unwrap_or_default();
        }

        self.state = points.resting_state();
        Vec::new()
    }

    fn process_precision(&mut self, points: Option<HandPoints>, timestamp: f64) -> Vec<Command> {
        let Some(points) = points.filter(|points| points.has_signal()) else {
            return self.handle_missing_controlling_hand(timestamp);
        };
        self.last_right_seen_at = Some(timestamp);
        let mut commands = Vec::new();
        if let Some(palm) = points.palm_center()
            && let Some(cursor) = self.relative_cursor(palm, timestamp, 0.40)
        {
            commands.push(Command::Move {
                x: cursor.0,
                y: cursor.1,
            });
        }
        commands.extend(self.process_assist_clicks(points, timestamp, 1, true));
        self.state = if self.primary_pinch.is_dragging() {
            GestureState::Dragging
        } else {
            GestureState::Pointer
        };
        commands
    }

    fn process_locked(&mut self, points: Option<HandPoints>, timestamp: f64) -> Vec<Command> {
        let Some(points) = points.filter(|points| points.has_signal()) else {
            return self.handle_missing_controlling_hand(timestamp);
        };
        self.last_right_seen_at = Some(timestamp);
        self.state = GestureState::Pointer;
        self.process_assist_clicks(points, timestamp, 1, false)
    }

    fn process_assist_clicks(
        &mut self,
        points: HandPoints,
        timestamp: f64,
        primary_click_count: u8,
        allow_drag: bool,
    ) -> Vec<Command> {
        let palm_size = self.palm_size_for(points, timestamp);
        if self.gesture_enabled(GESTURE_PRIMARY_CLICK) {
            let was_active = self.primary_pinch.is_active();
            let evidence = points.primary_pinch_evidence(
                palm_size,
                if was_active {
                    self.settings.pinch_exit
                } else {
                    self.settings.pinch_enter
                },
            );
            let action = self.primary_pinch.update(
                timestamp,
                evidence,
                self.last_cursor,
                allow_drag && self.gesture_enabled(GESTURE_DRAG),
                self.settings.drag_hold_seconds,
            );
            if was_active || self.primary_pinch.is_active() || action != PinchAction::None {
                return self.handle_primary_pinch(
                    action,
                    evidence,
                    points,
                    timestamp,
                    primary_click_count,
                    false,
                );
            }
        } else {
            self.primary_pinch.reset();
        }

        if self.gesture_enabled(GESTURE_SECONDARY_CLICK) {
            let was_active = self.secondary_pinch.is_active();
            let evidence = points.secondary_pinch_evidence(
                palm_size,
                if was_active {
                    self.settings.pinch_exit
                } else {
                    self.settings.pinch_enter
                },
            );
            let action = self.secondary_pinch.update(
                timestamp,
                evidence,
                self.last_cursor,
                false,
                self.settings.drag_hold_seconds,
            );
            if was_active || self.secondary_pinch.is_active() || action != PinchAction::None {
                return self.handle_secondary_pinch(action, points);
            }
        } else {
            self.secondary_pinch.reset();
        }
        Vec::new()
    }

    fn process_assist_scroll(
        &mut self,
        points: Option<HandPoints>,
        timestamp: f64,
    ) -> Vec<Command> {
        let Some(points) = points.filter(|points| points.has_signal()) else {
            return self.handle_missing_controlling_hand(timestamp);
        };
        self.last_right_seen_at = Some(timestamp);
        self.primary_pinch.reset();
        self.secondary_pinch.reset();
        self.state = GestureState::Scrolling;
        let Some(palm) = points.palm_center() else {
            return Vec::new();
        };
        let Some(previous) = self.assist_scroll_anchor_y.replace(palm.y) else {
            return Vec::new();
        };
        let delta = palm.y - previous;
        if delta.abs() < self.settings.scroll_dead_zone {
            return Vec::new();
        }
        vec![Command::Scroll {
            delta_y: delta * self.settings.scroll_gain,
        }]
    }

    fn process_assist_drag(&mut self, points: Option<HandPoints>, timestamp: f64) -> Vec<Command> {
        let Some(points) = points.filter(|points| points.has_signal()) else {
            return self.handle_missing_controlling_hand(timestamp);
        };
        self.last_right_seen_at = Some(timestamp);
        self.primary_pinch.reset();
        self.secondary_pinch.reset();
        self.state = GestureState::Dragging;
        let Some(palm) = points.palm_center() else {
            return Vec::new();
        };
        if !self.assist_drag_down {
            self.initialize_relative_cursor(palm, timestamp);
            self.assist_drag_down = true;
            return vec![Command::MouseDown {
                button: Button::Left,
                x: self.last_cursor.0,
                y: self.last_cursor.1,
            }];
        }
        self.relative_cursor(palm, timestamp, 0.40)
            .map(|cursor| {
                vec![Command::Move {
                    x: cursor.0,
                    y: cursor.1,
                }]
            })
            .unwrap_or_default()
    }

    fn apply_assist_transition(
        &mut self,
        transition: AssistTransition,
        right: Option<HandPoints>,
        timestamp: f64,
    ) -> Vec<Command> {
        let mut commands = Vec::new();
        commands.extend(self.end_window_grab());
        if transition.from == AssistMode::Drag && self.assist_drag_down {
            commands.push(Command::MouseUp {
                button: Button::Left,
                x: self.last_cursor.0,
                y: self.last_cursor.1,
            });
            self.assist_drag_down = false;
        }
        self.primary_pinch.reset();
        self.secondary_pinch.reset();
        self.scroll_anchor_y = None;
        self.reset_assist_anchors();
        commands.push(Command::AssistChanged {
            mode: transition.to,
        });

        if matches!(transition.to, AssistMode::Precision | AssistMode::Drag)
            && let Some(palm) = right.and_then(HandPoints::palm_center)
        {
            self.initialize_relative_cursor(palm, timestamp);
        }
        if transition.to == AssistMode::Drag && right.and_then(HandPoints::palm_center).is_some() {
            self.assist_drag_down = true;
            commands.push(Command::MouseDown {
                button: Button::Left,
                x: self.last_cursor.0,
                y: self.last_cursor.1,
            });
        }
        commands
    }

    fn handle_left_fist(&mut self, timestamp: f64) -> Option<Vec<Command>> {
        if self.left_fist_latched {
            return None;
        }
        let started = *self.left_fist_started_at.get_or_insert(timestamp);
        if timestamp - started < self.settings.fist_hold_seconds {
            return None;
        }

        let next_paused = !self.paused;
        let mut commands = self.release_commands();
        self.reset_right_state();
        if self.assist_tracker.clear().is_some() {
            commands.push(Command::AssistChanged {
                mode: AssistMode::None,
            });
        }
        self.paused = next_paused;
        self.state = if next_paused {
            GestureState::Paused
        } else {
            GestureState::NoHand
        };
        self.left_fist_latched = true;
        self.left_fist_started_at = Some(started);
        commands.push(Command::PauseChanged {
            paused: next_paused,
        });
        Some(commands)
    }

    fn handle_primary_pinch(
        &mut self,
        action: PinchAction,
        evidence: PinchEvidence,
        points: HandPoints,
        timestamp: f64,
        click_count: u8,
        move_drag_with_index: bool,
    ) -> Vec<Command> {
        match action {
            PinchAction::Click { anchor } => {
                self.state = points.resting_state();
                vec![Command::Click {
                    button: Button::Left,
                    x: anchor.0,
                    y: anchor.1,
                    count: click_count,
                }]
            }
            PinchAction::StartDrag { anchor } => {
                self.state = GestureState::Dragging;
                vec![Command::MouseDown {
                    button: Button::Left,
                    x: anchor.0,
                    y: anchor.1,
                }]
            }
            PinchAction::EndDrag => {
                self.state = points.resting_state();
                vec![Command::MouseUp {
                    button: Button::Left,
                    x: self.last_cursor.0,
                    y: self.last_cursor.1,
                }]
            }
            PinchAction::Cancelled => {
                self.state = points.resting_state();
                Vec::new()
            }
            PinchAction::None if self.primary_pinch.is_dragging() => {
                self.state = GestureState::Dragging;
                if move_drag_with_index
                    && evidence == PinchEvidence::Closed
                    && let Some(index_tip) = points.index_tip
                {
                    if let Some(cursor) = self.cursor_for(index_tip, timestamp) {
                        return vec![Command::Move {
                            x: cursor.0,
                            y: cursor.1,
                        }];
                    }
                    return Vec::new();
                }
                Vec::new()
            }
            PinchAction::None => {
                self.state = GestureState::PrimaryPinchPending;
                Vec::new()
            }
        }
    }

    fn handle_secondary_pinch(&mut self, action: PinchAction, points: HandPoints) -> Vec<Command> {
        match action {
            PinchAction::Click { anchor } => {
                self.state = points.resting_state();
                vec![Command::Click {
                    button: Button::Right,
                    x: anchor.0,
                    y: anchor.1,
                    count: 1,
                }]
            }
            PinchAction::Cancelled => {
                self.state = points.resting_state();
                Vec::new()
            }
            PinchAction::None => {
                self.state = GestureState::Pointer;
                Vec::new()
            }
            PinchAction::StartDrag { .. } | PinchAction::EndDrag => {
                unreachable!("secondary pinch never enables drag")
            }
        }
    }

    fn palm_size_for(&mut self, points: HandPoints, timestamp: f64) -> Option<f64> {
        match points.palm_size() {
            Some(size) => {
                self.last_palm_size = Some((size, timestamp));
                Some(size)
            }
            None => self.last_palm_size.and_then(|(size, measured_at)| {
                (timestamp - measured_at <= self.settings.hand_loss_seconds).then_some(size)
            }),
        }
    }

    fn cursor_for(&mut self, point: Point, timestamp: f64) -> Option<(f64, f64)> {
        let filtered = self
            .cursor_filter
            .update(point, timestamp, self.settings.smoothing);
        if let Some(mut handoff) = self.external_pointer {
            let Some(hand_anchor) = handoff.hand_anchor else {
                handoff.hand_anchor = Some(filtered);
                self.external_pointer = Some(handoff);
                self.last_cursor = handoff.cursor_anchor;
                return Some(handoff.cursor_anchor);
            };
            let screen = self.settings.screen;
            let target = (
                (handoff.cursor_anchor.0 - (filtered.x - hand_anchor.x) * screen.width * 0.45)
                    .clamp(screen.origin_x, screen.origin_x + screen.width),
                (handoff.cursor_anchor.1 - (filtered.y - hand_anchor.y) * screen.height * 0.45)
                    .clamp(screen.origin_y, screen.origin_y + screen.height),
            );
            self.last_cursor = target;
            return Some(target);
        }
        self.engaged_cursor(filtered)
    }

    /// Keep the cursor still until the index tip travels past a deliberate
    /// distance, then follow at the calibrated gain from the cursor's current
    /// position. The first recognized point never teleports the pointer.
    fn engaged_cursor(&mut self, filtered: Point) -> Option<(f64, f64)> {
        let anchor = *self.pointer_anchor.get_or_insert(filtered);
        if !self.pointer_armed {
            if distance(filtered, anchor) < POINTER_ENGAGE_DISTANCE {
                return None;
            }
            self.pointer_armed = true;
            self.pointer_origin_hand = Some(anchor);
            self.pointer_origin_cursor = self.last_cursor;
        }
        let origin_hand = self.pointer_origin_hand.unwrap_or(filtered);
        let origin = self.pointer_origin_cursor;
        let calibration = self.settings.calibration;
        let screen = self.settings.screen;
        let x_span = (calibration.max_x - calibration.min_x).max(0.25);
        let y_span = (calibration.max_y - calibration.min_y).max(0.20);
        let target = (
            (origin.0 - (filtered.x - origin_hand.x) / x_span * screen.width)
                .clamp(screen.origin_x, screen.origin_x + screen.width),
            (origin.1 - (filtered.y - origin_hand.y) / y_span * screen.height)
                .clamp(screen.origin_y, screen.origin_y + screen.height),
        );
        self.last_cursor = target;
        Some(target)
    }

    fn process_window_grab(
        &mut self,
        points: Option<HandPoints>,
        timestamp: f64,
        explicit_right: bool,
    ) -> (Vec<Command>, bool) {
        let usable = points.filter(|points| points.has_signal());
        let update = if let Some(points) = usable {
            self.last_right_seen_at = Some(timestamp);
            self.window_grab.update(
                timestamp,
                points.extended_finger_count(),
                points.is_fist(),
                explicit_right,
            )
        } else {
            self.window_grab
                .update_missing(timestamp, self.settings.hand_loss_seconds)
        };

        let mut commands = Vec::new();
        match update.action {
            WindowGrabAction::None => {}
            WindowGrabAction::Arm => {
                if let Some(palm) = usable.and_then(HandPoints::stable_palm) {
                    self.initialize_window_cursor(palm, timestamp);
                } else {
                    self.window_grab.reset();
                    self.reset_window_anchors();
                    return (commands, false);
                }
            }
            WindowGrabAction::Begin => {
                if let Some(palm) = usable.and_then(HandPoints::stable_palm) {
                    self.initialize_window_cursor(palm, timestamp);
                    commands.push(Command::WindowGrabBegin {
                        x: self.window_anchor_cursor.0,
                        y: self.window_anchor_cursor.1,
                    });
                } else {
                    self.window_grab.reset();
                    self.reset_window_anchors();
                    return (commands, false);
                }
            }
            WindowGrabAction::Move => {
                if let Some(palm) = usable.and_then(HandPoints::stable_palm)
                    && let Some(cursor) = self.window_cursor_for(palm, timestamp)
                {
                    commands.push(Command::WindowMove {
                        x: cursor.0,
                        y: cursor.1,
                    });
                }
            }
            WindowGrabAction::End => {
                self.reset_window_anchors();
                commands.push(Command::WindowGrabEnd);
            }
        }
        (commands, update.suppress_standard)
    }

    fn initialize_window_cursor(&mut self, palm: Point, timestamp: f64) {
        self.window_cursor_filter.reset();
        let filtered = self.window_cursor_filter.update(palm, timestamp, 0.85);
        self.window_anchor_palm = Some(filtered);
        self.window_anchor_cursor = self.last_cursor;
    }

    fn window_cursor_for(&mut self, palm: Point, timestamp: f64) -> Option<(f64, f64)> {
        let anchor = self.window_anchor_palm?;
        let filtered = self.window_cursor_filter.update(palm, timestamp, 0.85);
        let screen = self.settings.screen;
        let x = (self.window_anchor_cursor.0 - (filtered.x - anchor.x) * screen.width)
            .clamp(screen.origin_x, screen.origin_x + screen.width);
        let y = (self.window_anchor_cursor.1 - (filtered.y - anchor.y) * screen.height)
            .clamp(screen.origin_y, screen.origin_y + screen.height);
        self.last_cursor = (x, y);
        Some((x, y))
    }

    fn initialize_relative_cursor(&mut self, palm: Point, timestamp: f64) {
        self.assist_cursor_filter.reset();
        let filtered = self.assist_cursor_filter.update(
            palm,
            timestamp,
            (self.settings.smoothing * 0.65).clamp(0.05, 1.0),
        );
        self.assist_anchor_palm = Some(filtered);
        self.assist_anchor_cursor = self.last_cursor;
    }

    fn relative_cursor(&mut self, palm: Point, timestamp: f64, gain: f64) -> Option<(f64, f64)> {
        if self.assist_anchor_palm.is_none() {
            self.initialize_relative_cursor(palm, timestamp);
            return None;
        }
        let filtered = self.assist_cursor_filter.update(
            palm,
            timestamp,
            (self.settings.smoothing * 0.65).clamp(0.05, 1.0),
        );
        let anchor = self.assist_anchor_palm.expect("initialized above");
        let screen = self.settings.screen;
        let x = (self.assist_anchor_cursor.0 - (filtered.x - anchor.x) * screen.width * gain)
            .clamp(screen.origin_x, screen.origin_x + screen.width);
        let y = (self.assist_anchor_cursor.1 - (filtered.y - anchor.y) * screen.height * gain)
            .clamp(screen.origin_y, screen.origin_y + screen.height);
        if (x - self.last_cursor.0).hypot(y - self.last_cursor.1) < 0.5 {
            return None;
        }
        self.last_cursor = (x, y);
        Some((x, y))
    }

    fn pointer_is_extended(&mut self, measurement: Option<bool>, timestamp: f64) -> bool {
        match measurement {
            Some(true) => {
                self.index_extended_at = Some(timestamp);
                true
            }
            Some(false) => {
                self.index_extended_at = None;
                false
            }
            None => self
                .index_extended_at
                .is_some_and(|last| timestamp - last <= 0.10),
        }
    }

    fn handle_missing_controlling_hand(&mut self, timestamp: f64) -> Vec<Command> {
        let timed_out = self
            .last_right_seen_at
            .is_some_and(|last| timestamp - last >= self.settings.hand_loss_seconds);
        if !timed_out {
            return Vec::new();
        }
        let commands = self.release_commands();
        self.reset_right_state();
        self.state = if self.paused {
            GestureState::Paused
        } else {
            GestureState::NoHand
        };
        commands
    }

    fn release_commands(&mut self) -> Vec<Command> {
        let must_release_left = self.primary_pinch.is_dragging() || self.assist_drag_down;
        self.assist_drag_down = false;
        let mut commands = Vec::new();
        if must_release_left {
            commands.push(Command::MouseUp {
                button: Button::Left,
                x: self.last_cursor.0,
                y: self.last_cursor.1,
            });
        }
        commands.extend(self.end_window_grab());
        commands
    }

    fn end_window_grab(&mut self) -> Vec<Command> {
        let update = self.window_grab.force_end();
        self.reset_window_anchors();
        matches!(update.action, WindowGrabAction::End)
            .then_some(Command::WindowGrabEnd)
            .into_iter()
            .collect()
    }

    fn reset_window_anchors(&mut self) {
        self.window_cursor_filter.reset();
        self.window_anchor_palm = None;
        self.window_anchor_cursor = self.last_cursor;
    }

    fn reset_right_state(&mut self) {
        self.primary_pinch.reset();
        self.secondary_pinch.reset();
        self.scroll_anchor_y = None;
        self.last_palm_size = None;
        self.last_right_seen_at = None;
        self.index_extended_at = None;
        self.cursor_filter.reset();
        self.external_pointer = None;
        self.pointer_anchor = None;
        self.pointer_armed = false;
        self.pointer_origin_hand = None;
        self.pointer_origin_cursor = self.last_cursor;
        self.reset_assist_anchors();
        self.window_grab.reset();
        self.reset_window_anchors();
    }

    fn reset_assist_anchors(&mut self) {
        self.assist_cursor_filter.reset();
        self.assist_anchor_palm = None;
        self.assist_anchor_cursor = self.last_cursor;
        self.assist_scroll_anchor_y = None;
    }
}

#[derive(Clone, Copy)]
struct HandPoints {
    wrist: Option<Point>,
    thumb_mcp: Option<Point>,
    thumb_ip: Option<Point>,
    thumb_tip: Option<Point>,
    index_mcp: Option<Point>,
    index_pip: Option<Point>,
    index_tip: Option<Point>,
    middle_mcp: Option<Point>,
    middle_pip: Option<Point>,
    middle_tip: Option<Point>,
    ring_mcp: Option<Point>,
    ring_pip: Option<Point>,
    ring_tip: Option<Point>,
    little_mcp: Option<Point>,
    little_pip: Option<Point>,
    little_tip: Option<Point>,
}

impl HandPoints {
    fn from_frame(frame: &HandFrame, confidence: f64) -> Self {
        Self {
            wrist: frame.point(JointKind::Wrist, confidence),
            thumb_mcp: frame.point(JointKind::ThumbMcp, confidence),
            thumb_ip: frame.point(JointKind::ThumbIp, confidence),
            thumb_tip: frame.point(JointKind::ThumbTip, confidence),
            index_mcp: frame.point(JointKind::IndexMcp, confidence),
            index_pip: frame.point(JointKind::IndexPip, confidence),
            index_tip: frame.point(JointKind::IndexTip, confidence),
            middle_mcp: frame.point(JointKind::MiddleMcp, confidence),
            middle_pip: frame.point(JointKind::MiddlePip, confidence),
            middle_tip: frame.point(JointKind::MiddleTip, confidence),
            ring_mcp: frame.point(JointKind::RingMcp, confidence),
            ring_pip: frame.point(JointKind::RingPip, confidence),
            ring_tip: frame.point(JointKind::RingTip, confidence),
            little_mcp: frame.point(JointKind::LittleMcp, confidence),
            little_pip: frame.point(JointKind::LittlePip, confidence),
            little_tip: frame.point(JointKind::LittleTip, confidence),
        }
    }

    fn has_signal(self) -> bool {
        [
            self.wrist,
            self.thumb_mcp,
            self.thumb_ip,
            self.thumb_tip,
            self.index_mcp,
            self.index_pip,
            self.index_tip,
            self.middle_mcp,
            self.middle_pip,
            self.middle_tip,
            self.ring_mcp,
            self.ring_pip,
            self.ring_tip,
            self.little_mcp,
            self.little_pip,
            self.little_tip,
        ]
        .iter()
        .any(Option::is_some)
    }

    fn palm_size(self) -> Option<f64> {
        Some(distance(self.wrist?, self.middle_mcp?).max(0.05))
    }

    fn stable_palm(self) -> Option<Point> {
        let wrist = self.wrist?;
        let knuckles = [
            self.index_mcp, self.middle_mcp, self.ring_mcp, self.little_mcp,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        if knuckles.is_empty() {
            return Some(wrist);
        }
        let count = knuckles.len() as f64;
        let (x, y, confidence) = knuckles.iter().fold((0.0, 0.0, 0.0), |sum, point| {
            (sum.0 + point.x, sum.1 + point.y, sum.2 + point.confidence)
        });
        Some(Point::new(
            wrist.x * 0.75 + (x / count) * 0.25,
            wrist.y * 0.75 + (y / count) * 0.25,
            wrist.confidence.min(confidence / count),
        ))
    }

    fn palm_center(self) -> Option<Point> {
        let wrist = self.wrist?;
        let middle = self.middle_mcp?;
        Some(Point::new(
            (wrist.x + middle.x) / 2.0,
            (wrist.y + middle.y) / 2.0,
            wrist.confidence.min(middle.confidence),
        ))
    }

    fn primary_pinch_evidence(self, palm_size: Option<f64>, threshold: f64) -> PinchEvidence {
        match self.primary_pinch_ratio(palm_size) {
            Some(ratio) if ratio <= threshold => PinchEvidence::Closed,
            Some(_) => PinchEvidence::Open,
            None => PinchEvidence::Unknown,
        }
    }

    fn primary_pinch_ratio(self, palm_size: Option<f64>) -> Option<f64> {
        Some(distance(self.thumb_tip?, self.index_tip?) / palm_size?)
    }

    fn secondary_pinch_evidence(self, palm_size: Option<f64>, threshold: f64) -> PinchEvidence {
        match self.secondary_pinch_ratio(palm_size) {
            Some(ratio) if ratio <= threshold => PinchEvidence::Closed,
            Some(_) => PinchEvidence::Open,
            None => PinchEvidence::Unknown,
        }
    }

    fn secondary_pinch_ratio(self, palm_size: Option<f64>) -> Option<f64> {
        Some(distance(self.thumb_tip?, self.middle_tip?) / palm_size?)
    }

    fn index_extended(self) -> Option<bool> {
        Some(finger_extended(
            self.index_mcp?,
            self.index_pip?,
            self.index_tip?,
        ))
    }

    fn middle_extended(self) -> Option<bool> {
        Some(finger_extended(
            self.middle_mcp?,
            self.middle_pip?,
            self.middle_tip?,
        ))
    }

    fn ring_extended(self) -> Option<bool> {
        Some(finger_extended(
            self.ring_mcp?,
            self.ring_pip?,
            self.ring_tip?,
        ))
    }

    fn little_extended(self) -> Option<bool> {
        Some(finger_extended(
            self.little_mcp?,
            self.little_pip?,
            self.little_tip?,
        ))
    }

    fn index_folded(self) -> Option<bool> {
        Some(finger_folded(
            self.index_mcp?,
            self.index_pip?,
            self.index_tip?,
        ))
    }

    fn middle_folded(self) -> Option<bool> {
        Some(finger_folded(
            self.middle_mcp?,
            self.middle_pip?,
            self.middle_tip?,
        ))
    }

    fn ring_folded(self) -> Option<bool> {
        Some(finger_folded(
            self.ring_mcp?,
            self.ring_pip?,
            self.ring_tip?,
        ))
    }

    fn little_folded(self) -> Option<bool> {
        Some(finger_folded(
            self.little_mcp?,
            self.little_pip?,
            self.little_tip?,
        ))
    }

    fn thumb_up(self) -> bool {
        self.index_folded() == Some(true)
            && self.middle_folded() == Some(true)
            && self.ring_folded() == Some(true)
            && self.little_folded() == Some(true)
            && self
                .thumb_mcp
                .zip(self.thumb_ip)
                .zip(self.thumb_tip)
                .is_some_and(|((mcp, ip), tip)| {
                    finger_extended(mcp, ip, tip)
                        && self.wrist.is_some_and(|wrist| {
                            tip.y > wrist.y + self.palm_size().unwrap_or(0.20) * 1.20
                        })
                })
    }

    fn is_fist(self) -> bool {
        self.index_folded() == Some(true)
            && self.middle_folded() == Some(true)
            && self.ring_folded() == Some(true)
            && self.little_folded() == Some(true)
    }

    fn extended_finger_count(self) -> Option<u8> {
        let fingers = [
            self.index_extended()?,
            self.middle_extended()?,
            self.ring_extended()?,
            self.little_extended()?,
        ];
        Some(fingers.into_iter().map(u8::from).sum())
    }

    fn is_v_sign(self) -> bool {
        self.index_extended() == Some(true)
            && self.middle_extended() == Some(true)
            && self.ring_folded() == Some(true)
            && self.little_folded() == Some(true)
    }

    fn assist_mode(self) -> AssistMode {
        if self.index_extended() == Some(true)
            && self.middle_folded() == Some(true)
            && self.ring_folded() == Some(true)
            && self.little_folded() == Some(true)
        {
            AssistMode::CursorLock
        } else if self.thumb_up() {
            AssistMode::DoubleClick
        } else if self.index_extended() == Some(true)
            && self.middle_extended() == Some(true)
            && self.ring_extended() == Some(true)
            && self.little_folded() == Some(true)
        {
            AssistMode::Drag
        } else if self.is_v_sign() {
            AssistMode::Scroll
        } else if self.index_extended() == Some(true)
            && self.middle_extended() == Some(true)
            && self.ring_extended() == Some(true)
            && self.little_extended() == Some(true)
        {
            AssistMode::Precision
        } else {
            AssistMode::None
        }
    }

    fn resting_state(self) -> GestureState {
        if self.is_v_sign() {
            GestureState::Scrolling
        } else if self.index_extended() == Some(true) {
            GestureState::Pointer
        } else {
            GestureState::NoHand
        }
    }

    fn scroll_midpoint_y(self) -> Option<f64> {
        Some((self.index_tip?.y + self.middle_tip?.y) / 2.0)
    }
}

fn finger_extended(mcp: Point, pip: Point, tip: Point) -> bool {
    let Some((alignment, proximal_length, reach)) = finger_geometry(mcp, pip, tip) else {
        return false;
    };
    alignment >= 0.35 && reach >= proximal_length * 1.35
}

fn finger_folded(mcp: Point, pip: Point, tip: Point) -> bool {
    let Some((alignment, proximal_length, reach)) = finger_geometry(mcp, pip, tip) else {
        return false;
    };
    alignment <= 0.20 && reach <= proximal_length * 1.25
}

fn finger_geometry(mcp: Point, pip: Point, tip: Point) -> Option<(f64, f64, f64)> {
    let proximal = (pip.x - mcp.x, pip.y - mcp.y);
    let distal = (tip.x - pip.x, tip.y - pip.y);
    let proximal_length = proximal.0.hypot(proximal.1);
    let distal_length = distal.0.hypot(distal.1);
    if proximal_length < 0.01 || distal_length < 0.01 {
        return None;
    }
    let alignment =
        (proximal.0 * distal.0 + proximal.1 * distal.1) / (proximal_length * distal_length);
    Some((alignment, proximal_length, distance(mcp, tip)))
}

const POINTER_ENGAGE_DISTANCE: f64 = 0.045;

fn rising_hand(frame: Option<&HandFrame>, confidence: f64) -> Option<RisingHand> {
    let points = HandPoints::from_frame(frame?, confidence);
    Some(RisingHand {
        wrist: points.wrist?,
        open: points.extended_finger_count() == Some(4),
    })
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
