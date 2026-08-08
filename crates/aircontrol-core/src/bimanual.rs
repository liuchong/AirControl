#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum AssistMode {
    #[default]
    None = 0,
    Precision = 1,
    CursorLock = 2,
    DoubleClick = 3,
    Scroll = 4,
    Drag = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AssistTransition {
    pub from: AssistMode,
    pub to: AssistMode,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct AssistTracker {
    active: AssistMode,
    candidate: AssistMode,
    candidate_since: Option<f64>,
    candidate_samples: u8,
    last_left_seen_at: Option<f64>,
}

impl AssistTracker {
    const STABLE_SECONDS: f64 = 0.12;
    const LOSS_GRACE_SECONDS: f64 = 0.20;

    pub fn active(&self) -> AssistMode {
        self.active
    }

    pub fn update(
        &mut self,
        timestamp: f64,
        observed: Option<AssistMode>,
    ) -> Option<AssistTransition> {
        let target = match observed {
            Some(mode) => {
                self.last_left_seen_at = Some(timestamp);
                mode
            }
            None => {
                let timed_out = self
                    .last_left_seen_at
                    .is_some_and(|last| timestamp - last >= Self::LOSS_GRACE_SECONDS);
                if !timed_out {
                    return None;
                }
                self.last_left_seen_at = None;
                self.candidate = AssistMode::None;
                self.candidate_since = None;
                self.candidate_samples = 0;
                if self.active == AssistMode::None {
                    return None;
                }
                let transition = AssistTransition {
                    from: self.active,
                    to: AssistMode::None,
                };
                self.active = AssistMode::None;
                return Some(transition);
            }
        };

        if target == self.active {
            self.candidate = target;
            self.candidate_since = None;
            self.candidate_samples = 0;
            return None;
        }
        if target != self.candidate || self.candidate_since.is_none() {
            self.candidate = target;
            self.candidate_since = Some(timestamp);
            self.candidate_samples = 1;
            return None;
        }
        self.candidate_samples = self.candidate_samples.saturating_add(1);
        let stable = self.candidate_samples >= 2
            && timestamp - self.candidate_since.unwrap_or(timestamp) >= Self::STABLE_SECONDS;
        if !stable {
            return None;
        }

        let transition = AssistTransition {
            from: self.active,
            to: target,
        };
        self.active = target;
        self.candidate_since = None;
        self.candidate_samples = 0;
        Some(transition)
    }

    pub fn clear(&mut self) -> Option<AssistTransition> {
        let from = self.active;
        *self = Self::default();
        (from != AssistMode::None).then_some(AssistTransition {
            from,
            to: AssistMode::None,
        })
    }
}
