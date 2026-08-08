const EVIDENCE_GRACE_SECONDS: f64 = 0.08;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PinchEvidence {
    Closed,
    Open,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PinchAction {
    None,
    Cancelled,
    Click { anchor: (f64, f64) },
    StartDrag { anchor: (f64, f64) },
    EndDrag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PinchPhase {
    Idle,
    Arming,
    Pressed,
    Dragging,
}

#[derive(Clone)]
pub(crate) struct PinchTracker {
    phase: PinchPhase,
    closed_samples: u8,
    open_samples: u8,
    started_at: Option<f64>,
    last_closed_at: Option<f64>,
    anchor: (f64, f64),
}

impl Default for PinchTracker {
    fn default() -> Self {
        Self {
            phase: PinchPhase::Idle,
            closed_samples: 0,
            open_samples: 0,
            started_at: None,
            last_closed_at: None,
            anchor: (0.0, 0.0),
        }
    }
}

impl PinchTracker {
    pub(crate) fn is_active(&self) -> bool {
        self.phase != PinchPhase::Idle
    }

    pub(crate) fn is_dragging(&self) -> bool {
        self.phase == PinchPhase::Dragging
    }

    pub(crate) fn update(
        &mut self,
        timestamp: f64,
        evidence: PinchEvidence,
        cursor_anchor: (f64, f64),
        drag_enabled: bool,
        drag_hold_seconds: f64,
    ) -> PinchAction {
        match evidence {
            PinchEvidence::Closed => {
                self.observe_closed(timestamp, cursor_anchor, drag_enabled, drag_hold_seconds)
            }
            PinchEvidence::Open => self.observe_open(),
            PinchEvidence::Unknown => self.observe_unknown(timestamp),
        }
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    fn observe_closed(
        &mut self,
        timestamp: f64,
        cursor_anchor: (f64, f64),
        drag_enabled: bool,
        drag_hold_seconds: f64,
    ) -> PinchAction {
        if self.phase == PinchPhase::Idle {
            self.phase = PinchPhase::Arming;
            self.closed_samples = 1;
            self.open_samples = 0;
            self.started_at = Some(timestamp);
            self.last_closed_at = Some(timestamp);
            self.anchor = cursor_anchor;
            return PinchAction::None;
        }

        self.closed_samples = self.closed_samples.saturating_add(1);
        self.open_samples = 0;
        self.last_closed_at = Some(timestamp);
        if self.phase == PinchPhase::Arming && self.closed_samples >= 2 {
            self.phase = PinchPhase::Pressed;
        }
        if self.phase == PinchPhase::Pressed
            && drag_enabled
            && self
                .started_at
                .is_some_and(|started| timestamp - started >= drag_hold_seconds)
        {
            self.phase = PinchPhase::Dragging;
            return PinchAction::StartDrag {
                anchor: self.anchor,
            };
        }
        PinchAction::None
    }

    fn observe_open(&mut self) -> PinchAction {
        match self.phase {
            PinchPhase::Idle => PinchAction::None,
            PinchPhase::Arming => {
                self.reset();
                PinchAction::Cancelled
            }
            PinchPhase::Pressed | PinchPhase::Dragging => {
                self.open_samples = self.open_samples.saturating_add(1);
                if self.open_samples < 2 {
                    return PinchAction::None;
                }
                let action = if self.phase == PinchPhase::Dragging {
                    PinchAction::EndDrag
                } else {
                    PinchAction::Click {
                        anchor: self.anchor,
                    }
                };
                self.reset();
                action
            }
        }
    }

    fn observe_unknown(&mut self, timestamp: f64) -> PinchAction {
        if !self.is_active()
            || self
                .last_closed_at
                .is_some_and(|last| timestamp - last <= EVIDENCE_GRACE_SECONDS)
        {
            return PinchAction::None;
        }

        let action = match self.phase {
            PinchPhase::Idle | PinchPhase::Arming => PinchAction::Cancelled,
            PinchPhase::Pressed => PinchAction::Click {
                anchor: self.anchor,
            },
            PinchPhase::Dragging => PinchAction::EndDrag,
        };
        self.reset();
        action
    }
}
