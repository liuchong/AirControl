use crate::ScreenRect;

pub const GAZE_TARGETS: [(f64, f64); 9] = [
    (0.12, 0.12),
    (0.50, 0.12),
    (0.88, 0.12),
    (0.12, 0.50),
    (0.50, 0.50),
    (0.88, 0.50),
    (0.12, 0.88),
    (0.50, 0.88),
    (0.88, 0.88),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GazeSample {
    pub x: f64,
    pub y: f64,
    pub confidence: f64,
    pub eyes_open: bool,
}

impl GazeSample {
    pub const fn new(x: f64, y: f64, confidence: f64, eyes_open: bool) -> Self {
        Self {
            x,
            y,
            confidence,
            eyes_open,
        }
    }

    fn is_valid(self, minimum_confidence: f64) -> bool {
        self.eyes_open
            && self.x.is_finite()
            && self.y.is_finite()
            && self.confidence.is_finite()
            && (0.0..=1.0).contains(&self.x)
            && (0.0..=1.0).contains(&self.y)
            && (minimum_confidence..=1.0).contains(&self.confidence)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GazeProfile {
    pub x_bias: f64,
    pub x_from_x: f64,
    pub x_from_y: f64,
    pub y_bias: f64,
    pub y_from_x: f64,
    pub y_from_y: f64,
    pub raw_min_x: f64,
    pub raw_max_x: f64,
    pub raw_min_y: f64,
    pub raw_max_y: f64,
}

impl GazeProfile {
    pub fn fit(samples: &[GazeSample; 9]) -> Option<Self> {
        let mut normal = [[0.0; 3]; 3];
        let mut target_x = [0.0; 3];
        let mut target_y = [0.0; 3];
        let mut raw_min_x = f64::INFINITY;
        let mut raw_max_x = f64::NEG_INFINITY;
        let mut raw_min_y = f64::INFINITY;
        let mut raw_max_y = f64::NEG_INFINITY;

        for (sample, target) in samples.iter().zip(GAZE_TARGETS) {
            if !sample.is_valid(0.0) {
                return None;
            }
            let basis = [1.0, sample.x, sample.y];
            for row in 0..3 {
                target_x[row] += basis[row] * target.0;
                target_y[row] += basis[row] * target.1;
                for column in 0..3 {
                    normal[row][column] += basis[row] * basis[column];
                }
            }
            raw_min_x = raw_min_x.min(sample.x);
            raw_max_x = raw_max_x.max(sample.x);
            raw_min_y = raw_min_y.min(sample.y);
            raw_max_y = raw_max_y.max(sample.y);
        }

        let x = solve_3x3(normal, target_x)?;
        let y = solve_3x3(normal, target_y)?;
        let profile = Self {
            x_bias: x[0],
            x_from_x: x[1],
            x_from_y: x[2],
            y_bias: y[0],
            y_from_x: y[1],
            y_from_y: y[2],
            raw_min_x,
            raw_max_x,
            raw_min_y,
            raw_max_y,
        };
        profile.is_valid().then_some(profile)
    }

    pub fn is_valid(self) -> bool {
        [
            self.x_bias,
            self.x_from_x,
            self.x_from_y,
            self.y_bias,
            self.y_from_x,
            self.y_from_y,
            self.raw_min_x,
            self.raw_max_x,
            self.raw_min_y,
            self.raw_max_y,
        ]
        .iter()
        .all(|value| value.is_finite())
            && self.raw_max_x - self.raw_min_x >= 0.08
            && self.raw_max_y - self.raw_min_y >= 0.06
            && self.raw_min_x >= 0.0
            && self.raw_max_x <= 1.0
            && self.raw_min_y >= 0.0
            && self.raw_max_y <= 1.0
            && [
                self.x_bias,
                self.x_from_x,
                self.x_from_y,
                self.y_bias,
                self.y_from_x,
                self.y_from_y,
            ]
            .iter()
            .all(|value| value.abs() <= 100.0)
    }

    pub fn map_normalized(self, sample: GazeSample) -> (f64, f64) {
        (
            self.x_bias + self.x_from_x * sample.x + self.x_from_y * sample.y,
            self.y_bias + self.y_from_x * sample.x + self.y_from_y * sample.y,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GazeCalibrationUpdate {
    Progress { stage: usize, progress: f64 },
    TargetCaptured { stage: usize },
    Completed(GazeProfile),
    InvalidProfile,
}

pub struct GazeCalibrationTracker {
    minimum_confidence: f64,
    hold_seconds: f64,
    stability_radius: f64,
    stage: usize,
    hold_started_at: Option<f64>,
    anchor: Option<GazeSample>,
    samples: Vec<GazeSample>,
    captured: Vec<GazeSample>,
}

impl GazeCalibrationTracker {
    pub fn new(minimum_confidence: f64, hold_seconds: f64, stability_radius: f64) -> Self {
        Self {
            minimum_confidence: minimum_confidence.clamp(0.0, 1.0),
            hold_seconds: hold_seconds.max(0.1),
            stability_radius: stability_radius.max(0.001),
            stage: 0,
            hold_started_at: None,
            anchor: None,
            samples: Vec::new(),
            captured: Vec::with_capacity(9),
        }
    }

    pub fn update(&mut self, timestamp: f64, sample: Option<GazeSample>) -> GazeCalibrationUpdate {
        let Some(sample) = sample.filter(|sample| sample.is_valid(self.minimum_confidence)) else {
            self.reset_hold();
            return GazeCalibrationUpdate::Progress {
                stage: self.stage,
                progress: 0.0,
            };
        };
        if !timestamp.is_finite() {
            self.reset_hold();
            return GazeCalibrationUpdate::Progress {
                stage: self.stage,
                progress: 0.0,
            };
        }

        let stable = self.anchor.is_none_or(|anchor| {
            (anchor.x - sample.x).hypot(anchor.y - sample.y) <= self.stability_radius
        });
        if !stable {
            self.reset_hold();
        }
        if self.anchor.is_none() {
            self.anchor = Some(sample);
            self.hold_started_at = Some(timestamp);
        }
        self.samples.push(sample);
        let elapsed = (timestamp - self.hold_started_at.unwrap_or(timestamp)).max(0.0);
        if elapsed < self.hold_seconds || self.samples.len() < 2 {
            return GazeCalibrationUpdate::Progress {
                stage: self.stage,
                progress: (elapsed / self.hold_seconds).clamp(0.0, 1.0),
            };
        }

        let captured_stage = self.stage;
        self.captured.push(average(&self.samples));
        self.reset_hold();
        self.stage += 1;
        if self.stage < GAZE_TARGETS.len() {
            return GazeCalibrationUpdate::TargetCaptured {
                stage: captured_stage,
            };
        }

        let samples: [GazeSample; 9] = self
            .captured
            .as_slice()
            .try_into()
            .expect("nine stages capture nine samples");
        self.reset();
        GazeProfile::fit(&samples)
            .map(GazeCalibrationUpdate::Completed)
            .unwrap_or(GazeCalibrationUpdate::InvalidProfile)
    }

    pub fn reset(&mut self) {
        self.stage = 0;
        self.captured.clear();
        self.reset_hold();
    }

    fn reset_hold(&mut self) {
        self.hold_started_at = None;
        self.anchor = None;
        self.samples.clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GazeError {
    InvalidProfile,
    InvalidSettings,
}

pub struct GazeMapper {
    profile: GazeProfile,
    screen: ScreenRect,
    minimum_confidence: f64,
    smoothing: f64,
    filtered: Option<(f64, f64)>,
    last_timestamp: Option<f64>,
}

impl GazeMapper {
    pub fn new(
        profile: GazeProfile,
        screen: ScreenRect,
        minimum_confidence: f64,
        smoothing: f64,
    ) -> Result<Self, GazeError> {
        if !profile.is_valid() {
            return Err(GazeError::InvalidProfile);
        }
        if !screen.is_valid()
            || !minimum_confidence.is_finite()
            || !(0.0..=1.0).contains(&minimum_confidence)
            || !smoothing.is_finite()
            || !(0.05..=1.0).contains(&smoothing)
        {
            return Err(GazeError::InvalidSettings);
        }
        Ok(Self {
            profile,
            screen,
            minimum_confidence,
            smoothing,
            filtered: None,
            last_timestamp: None,
        })
    }

    pub fn update(&mut self, timestamp: f64, sample: Option<GazeSample>) -> Option<(f64, f64)> {
        if !timestamp.is_finite() {
            return None;
        }
        let sample = sample.filter(|sample| sample.is_valid(self.minimum_confidence))?;
        let mapped = self.profile.map_normalized(sample);
        if !mapped.0.is_finite() || !mapped.1.is_finite() {
            return None;
        }
        let target = (mapped.0.clamp(0.0, 1.0), mapped.1.clamp(0.0, 1.0));
        let filtered = if let Some(previous) = self.filtered {
            let elapsed = self
                .last_timestamp
                .map(|last| timestamp - last)
                .unwrap_or(1.0 / 60.0)
                .clamp(1.0 / 240.0, 0.10);
            let time_constant = 0.36 - 0.28 * self.smoothing;
            let alpha = 1.0 - (-elapsed / time_constant).exp();
            (
                previous.0 + alpha * (target.0 - previous.0),
                previous.1 + alpha * (target.1 - previous.1),
            )
        } else {
            target
        };
        self.filtered = Some(filtered);
        self.last_timestamp = Some(timestamp);
        Some((
            self.screen.origin_x + filtered.0 * self.screen.width,
            self.screen.origin_y + filtered.1 * self.screen.height,
        ))
    }

    pub fn reset(&mut self) {
        self.filtered = None;
        self.last_timestamp = None;
    }
}

fn average(samples: &[GazeSample]) -> GazeSample {
    let count = samples.len() as f64;
    GazeSample::new(
        samples.iter().map(|sample| sample.x).sum::<f64>() / count,
        samples.iter().map(|sample| sample.y).sum::<f64>() / count,
        samples.iter().map(|sample| sample.confidence).sum::<f64>() / count,
        true,
    )
}

fn solve_3x3(matrix: [[f64; 3]; 3], values: [f64; 3]) -> Option<[f64; 3]> {
    let mut augmented = [[0.0; 4]; 3];
    for row in 0..3 {
        augmented[row][..3].copy_from_slice(&matrix[row]);
        augmented[row][3] = values[row];
    }
    for pivot in 0..3 {
        let best = (pivot..3).max_by(|left, right| {
            augmented[*left][pivot]
                .abs()
                .total_cmp(&augmented[*right][pivot].abs())
        })?;
        augmented.swap(pivot, best);
        let divisor = augmented[pivot][pivot];
        if !divisor.is_finite() || divisor.abs() < 1e-10 {
            return None;
        }
        for value in augmented[pivot].iter_mut().skip(pivot) {
            *value /= divisor;
        }
        let pivot_row = augmented[pivot];
        for (row, values) in augmented.iter_mut().enumerate() {
            if row == pivot {
                continue;
            }
            let factor = values[pivot];
            for (column, value) in values.iter_mut().enumerate().skip(pivot) {
                *value -= factor * pivot_row[column];
            }
        }
    }
    Some([augmented[0][3], augmented[1][3], augmented[2][3]])
}
