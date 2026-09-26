use crate::Point;

const SETTLE_SECONDS: f64 = 0.10;
const RISE_SECONDS: f64 = 0.50;
const RISE_DISTANCE: f64 = 0.12;
const REQUIRED_SAMPLES: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RisingHand {
    pub wrist: Point,
    pub open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OverviewUpdate {
    pub show: bool,
    pub suppress: bool,
}

#[derive(Clone, Copy, Debug)]
enum Phase {
    Idle,
    Settling { started_at: f64, samples: u8, left: Point, right: Point },
    Rising { started_at: f64, left: Point, right: Point },
    Neutral,
}

#[derive(Clone, Debug)]
pub(crate) struct OverviewTracker {
    phase: Phase,
}

impl Default for OverviewTracker {
    fn default() -> Self {
        Self { phase: Phase::Idle }
    }
}

impl OverviewTracker {
    pub fn update(
        &mut self,
        timestamp: f64,
        left: Option<RisingHand>,
        right: Option<RisingHand>,
        blocked: bool,
    ) -> OverviewUpdate {
        if blocked {
            self.phase = Phase::Idle;
            return OverviewUpdate { show: false, suppress: false };
        }
        let Some((left, right)) = left.zip(right).filter(|(left, right)| left.open && right.open) else {
            if matches!(self.phase, Phase::Neutral) {
                self.phase = Phase::Idle;
            } else if !matches!(self.phase, Phase::Idle) {
                self.phase = Phase::Idle;
            }
            return OverviewUpdate { show: false, suppress: false };
        };

        match self.phase {
            Phase::Idle => {
                self.phase = Phase::Settling {
                    started_at: timestamp,
                    samples: 1,
                    left: left.wrist,
                    right: right.wrist,
                };
                OverviewUpdate { show: false, suppress: true }
            }
            Phase::Settling { started_at, samples, left: origin_left, right: origin_right } => {
                if distance(left.wrist, origin_left) > 0.04 || distance(right.wrist, origin_right) > 0.04 {
                    self.phase = Phase::Settling {
                        started_at: timestamp,
                        samples: 1,
                        left: left.wrist,
                        right: right.wrist,
                    };
                    return OverviewUpdate { show: false, suppress: true };
                }
                let samples = samples.saturating_add(1);
                if samples >= REQUIRED_SAMPLES && timestamp - started_at >= SETTLE_SECONDS {
                    self.phase = Phase::Rising {
                        started_at: timestamp,
                        left: left.wrist,
                        right: right.wrist,
                    };
                } else {
                    self.phase = Phase::Settling {
                        started_at,
                        samples,
                        left: origin_left,
                        right: origin_right,
                    };
                }
                OverviewUpdate { show: false, suppress: true }
            }
            Phase::Rising { started_at, left: origin_left, right: origin_right } => {
                if timestamp - started_at > RISE_SECONDS {
                    self.phase = Phase::Idle;
                    return OverviewUpdate { show: false, suppress: false };
                }
                let left_rise = left.wrist.y - origin_left.y;
                let right_rise = right.wrist.y - origin_right.y;
                let left_drift = (left.wrist.x - origin_left.x).abs();
                let right_drift = (right.wrist.x - origin_right.x).abs();
                if left_rise >= RISE_DISTANCE
                    && right_rise >= RISE_DISTANCE
                    && left_drift < 0.10
                    && right_drift < 0.10
                {
                    self.phase = Phase::Neutral;
                    return OverviewUpdate { show: true, suppress: false };
                }
                OverviewUpdate { show: false, suppress: true }
            }
            Phase::Neutral => OverviewUpdate { show: false, suppress: false },
        }
    }

    pub fn reset(&mut self) {
        self.phase = Phase::Idle;
    }
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
