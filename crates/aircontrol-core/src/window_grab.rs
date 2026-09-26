const OPEN_HOLD_SECONDS: f64 = 0.12;
const CLOSE_TIMEOUT_SECONDS: f64 = 0.65;
const REQUIRED_SAMPLES: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowGrabAction {
    None,
    Arm,
    Begin,
    Move,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowGrabUpdate {
    pub action: WindowGrabAction,
    pub suppress_standard: bool,
}

#[derive(Clone, Copy, Debug)]
enum Phase {
    Idle,
    OpenCandidate {
        started_at: f64,
        samples: u8,
    },
    Armed {
        closing_started_at: Option<f64>,
        fist_samples: u8,
    },
    Grabbed {
        open_samples: u8,
    },
    AwaitNeutral,
}

#[derive(Clone, Debug)]
pub(crate) struct WindowGrabTracker {
    phase: Phase,
    last_seen_at: Option<f64>,
}

impl Default for WindowGrabTracker {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            last_seen_at: None,
        }
    }
}

impl WindowGrabTracker {
    pub fn update(
        &mut self,
        timestamp: f64,
        extended_fingers: Option<u8>,
        is_fist: bool,
        allowed: bool,
    ) -> WindowGrabUpdate {
        if !allowed {
            return self.force_end();
        }
        self.last_seen_at = Some(timestamp);
        let fully_open = extended_fingers == Some(4);

        match self.phase {
            Phase::Idle => {
                if fully_open {
                    self.phase = Phase::OpenCandidate {
                        started_at: timestamp,
                        samples: 1,
                    };
                }
                idle_update()
            }
            Phase::OpenCandidate {
                started_at,
                samples,
            } => {
                if !fully_open {
                    self.phase = Phase::Idle;
                    return idle_update();
                }
                let samples = samples.saturating_add(1);
                if samples >= REQUIRED_SAMPLES && timestamp - started_at >= OPEN_HOLD_SECONDS {
                    self.phase = Phase::Armed {
                        closing_started_at: None,
                        fist_samples: 0,
                    };
                    return WindowGrabUpdate {
                        action: WindowGrabAction::Arm,
                        suppress_standard: true,
                    };
                }
                self.phase = Phase::OpenCandidate {
                    started_at,
                    samples,
                };
                idle_update()
            }
            Phase::Armed {
                mut closing_started_at,
                mut fist_samples,
            } => {
                if fully_open {
                    if closing_started_at.is_some() {
                        self.phase = Phase::OpenCandidate {
                            started_at: timestamp,
                            samples: 1,
                        };
                        return idle_update();
                    }
                    return WindowGrabUpdate {
                        action: WindowGrabAction::None,
                        suppress_standard: true,
                    };
                }
                if extended_fingers.is_none() {
                    self.phase = Phase::Idle;
                    return idle_update();
                }
                let started_at = *closing_started_at.get_or_insert(timestamp);
                if timestamp - started_at > CLOSE_TIMEOUT_SECONDS {
                    self.phase = Phase::Idle;
                    return idle_update();
                }
                if is_fist {
                    fist_samples = fist_samples.saturating_add(1);
                    if fist_samples >= REQUIRED_SAMPLES {
                        self.phase = Phase::Grabbed { open_samples: 0 };
                        return WindowGrabUpdate {
                            action: WindowGrabAction::Begin,
                            suppress_standard: true,
                        };
                    }
                } else {
                    fist_samples = 0;
                }
                self.phase = Phase::Armed {
                    closing_started_at,
                    fist_samples,
                };
                WindowGrabUpdate {
                    action: WindowGrabAction::None,
                    suppress_standard: true,
                }
            }
            Phase::Grabbed { mut open_samples } => {
                if fully_open {
                    open_samples = open_samples.saturating_add(1);
                    if open_samples >= REQUIRED_SAMPLES {
                        self.phase = Phase::AwaitNeutral;
                        return WindowGrabUpdate {
                            action: WindowGrabAction::End,
                            suppress_standard: true,
                        };
                    }
                    self.phase = Phase::Grabbed { open_samples };
                    return WindowGrabUpdate {
                        action: WindowGrabAction::None,
                        suppress_standard: true,
                    };
                }
                self.phase = Phase::Grabbed { open_samples: 0 };
                WindowGrabUpdate {
                    action: WindowGrabAction::Move,
                    suppress_standard: true,
                }
            }
            Phase::AwaitNeutral => {
                if !fully_open && extended_fingers.is_some() {
                    self.phase = Phase::Idle;
                }
                idle_update()
            }
        }
    }

    pub fn update_missing(&mut self, timestamp: f64, hand_loss_seconds: f64) -> WindowGrabUpdate {
        if matches!(self.phase, Phase::Grabbed { .. }) {
            let timed_out = self
                .last_seen_at
                .is_some_and(|last| timestamp - last >= hand_loss_seconds);
            if timed_out {
                self.phase = Phase::Idle;
                self.last_seen_at = None;
                return WindowGrabUpdate {
                    action: WindowGrabAction::End,
                    suppress_standard: true,
                };
            }
            return WindowGrabUpdate {
                action: WindowGrabAction::None,
                suppress_standard: true,
            };
        }
        self.reset();
        idle_update()
    }

    pub fn force_end(&mut self) -> WindowGrabUpdate {
        let was_grabbed = matches!(self.phase, Phase::Grabbed { .. });
        self.reset();
        WindowGrabUpdate {
            action: if was_grabbed {
                WindowGrabAction::End
            } else {
                WindowGrabAction::None
            },
            suppress_standard: was_grabbed,
        }
    }

    pub fn is_engaged(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
    }

    pub fn reset(&mut self) {
        self.phase = Phase::Idle;
        self.last_seen_at = None;
    }
}

const fn idle_update() -> WindowGrabUpdate {
    WindowGrabUpdate {
        action: WindowGrabAction::None,
        suppress_standard: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fist_without_open_candidate_stays_idle() {
        let mut tracker = WindowGrabTracker::default();
        for timestamp in [0.0, 0.1, 0.8] {
            assert_eq!(
                tracker.update(timestamp, Some(0), true, true),
                idle_update()
            );
        }
    }

    #[test]
    fn complete_sequence_requires_two_open_and_two_fist_samples() {
        let mut tracker = WindowGrabTracker::default();
        assert_eq!(tracker.update(0.00, Some(4), false, true), idle_update());
        assert_eq!(
            tracker.update(0.13, Some(4), false, true).action,
            WindowGrabAction::Arm
        );
        assert_eq!(
            tracker.update(0.20, Some(2), false, true).action,
            WindowGrabAction::None
        );
        assert_eq!(
            tracker.update(0.25, Some(0), true, true).action,
            WindowGrabAction::None
        );
        assert_eq!(
            tracker.update(0.29, Some(0), true, true).action,
            WindowGrabAction::Begin
        );
    }
}
