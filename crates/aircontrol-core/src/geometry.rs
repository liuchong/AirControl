use crate::model::Point;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calibration {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRect {
    pub origin_x: f64,
    pub origin_y: f64,
    pub width: f64,
    pub height: f64,
}

impl Calibration {
    pub fn from_corners(top_left: Point, bottom_right: Point) -> Option<Self> {
        let calibration = Self {
            min_x: top_left.x,
            max_x: bottom_right.x,
            min_y: bottom_right.y,
            max_y: top_left.y,
        };
        calibration.is_valid().then_some(calibration)
    }

    pub fn is_valid(&self) -> bool {
        [self.min_x, self.max_x, self.min_y, self.max_y]
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
            && self.max_x - self.min_x >= 0.25
            && self.max_y - self.min_y >= 0.20
    }

    pub fn map_mirrored(&self, point: Point, screen: ScreenRect) -> (f64, f64) {
        let normalized_x = ((self.max_x - point.x) / (self.max_x - self.min_x)).clamp(0.0, 1.0);
        let normalized_y = ((point.y - self.min_y) / (self.max_y - self.min_y)).clamp(0.0, 1.0);
        (
            screen.origin_x + normalized_x * screen.width,
            screen.origin_y + normalized_y * screen.height,
        )
    }
}

impl ScreenRect {
    pub fn is_valid(&self) -> bool {
        [self.origin_x, self.origin_y, self.width, self.height]
            .iter()
            .all(|value| value.is_finite())
            && self.width > 0.0
            && self.height > 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationStage {
    TopLeft,
    BottomRight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CalibrationUpdate {
    Progress {
        stage: CalibrationStage,
        progress: f64,
    },
    CornerCaptured(CalibrationStage),
    Completed(Calibration),
    InvalidRange,
}

pub struct CalibrationTracker {
    hold_seconds: f64,
    stability_radius: f64,
    stage: CalibrationStage,
    hold_started_at: Option<f64>,
    anchor: Option<Point>,
    samples: Vec<Point>,
    top_left: Option<Point>,
}

impl CalibrationTracker {
    pub fn new(hold_seconds: f64, stability_radius: f64) -> Self {
        Self {
            hold_seconds: hold_seconds.max(0.1),
            stability_radius: stability_radius.max(0.001),
            stage: CalibrationStage::TopLeft,
            hold_started_at: None,
            anchor: None,
            samples: Vec::new(),
            top_left: None,
        }
    }

    pub fn update(&mut self, timestamp: f64, point: Option<Point>) -> CalibrationUpdate {
        let Some(point) = point.filter(valid_point) else {
            self.reset_hold();
            return CalibrationUpdate::Progress {
                stage: self.stage,
                progress: 0.0,
            };
        };
        let stable = self.anchor.is_none_or(|anchor| {
            (anchor.x - point.x).hypot(anchor.y - point.y) <= self.stability_radius
        });
        if !stable {
            self.reset_hold();
        }
        if self.anchor.is_none() {
            self.anchor = Some(point);
            self.hold_started_at = Some(timestamp);
        }
        self.samples.push(point);
        let started = self.hold_started_at.unwrap_or(timestamp);
        let elapsed = (timestamp - started).max(0.0);
        if elapsed < self.hold_seconds {
            return CalibrationUpdate::Progress {
                stage: self.stage,
                progress: (elapsed / self.hold_seconds).clamp(0.0, 1.0),
            };
        }

        let captured = average(&self.samples);
        self.reset_hold();
        match self.stage {
            CalibrationStage::TopLeft => {
                self.top_left = Some(captured);
                self.stage = CalibrationStage::BottomRight;
                CalibrationUpdate::CornerCaptured(CalibrationStage::TopLeft)
            }
            CalibrationStage::BottomRight => {
                self.stage = CalibrationStage::TopLeft;
                let top_left = self.top_left.take();
                match top_left.and_then(|top_left| Calibration::from_corners(top_left, captured)) {
                    Some(calibration) => CalibrationUpdate::Completed(calibration),
                    None => CalibrationUpdate::InvalidRange,
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.stage = CalibrationStage::TopLeft;
        self.top_left = None;
        self.reset_hold();
    }

    fn reset_hold(&mut self) {
        self.hold_started_at = None;
        self.anchor = None;
        self.samples.clear();
    }
}

fn valid_point(point: &Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && (0.0..=1.0).contains(&point.x)
        && (0.0..=1.0).contains(&point.y)
}

fn average(points: &[Point]) -> Point {
    let count = points.len() as f64;
    let (x, y, confidence) = points.iter().fold((0.0, 0.0, 0.0), |sum, point| {
        (sum.0 + point.x, sum.1 + point.y, sum.2 + point.confidence)
    });
    Point::new(x / count, y / count, confidence / count)
}
