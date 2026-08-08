use crate::Point;

const MIN_FRAME_SECONDS: f64 = 1.0 / 240.0;
const MAX_FRAME_SECONDS: f64 = 0.10;

#[derive(Clone, Default)]
pub(crate) struct CursorFilter {
    position: Option<Point>,
    last_timestamp: Option<f64>,
}

impl CursorFilter {
    pub(crate) fn update(&mut self, target: Point, timestamp: f64, smoothing: f64) -> Point {
        let Some(previous) = self.position else {
            self.position = Some(target);
            self.last_timestamp = Some(timestamp);
            return target;
        };

        let elapsed = self
            .last_timestamp
            .map(|last| timestamp - last)
            .unwrap_or(MIN_FRAME_SECONDS)
            .clamp(MIN_FRAME_SECONDS, MAX_FRAME_SECONDS);
        let displacement = (target.x - previous.x).hypot(target.y - previous.y);

        // Small corrections are deliberately damped; deliberate movement gets a shorter
        // time constant. The exponential step makes the response depend on elapsed time
        // rather than how many camera frames happened to arrive.
        let slow_time_constant = 0.20 - 0.16 * smoothing;
        let fast_time_constant = slow_time_constant * 0.28;
        let motion = (displacement / 0.08).clamp(0.0, 1.0);
        let time_constant = slow_time_constant + (fast_time_constant - slow_time_constant) * motion;
        let alpha = 1.0 - (-elapsed / time_constant).exp();

        let filtered = Point::new(
            previous.x + alpha * (target.x - previous.x),
            previous.y + alpha * (target.y - previous.y),
            target.confidence,
        );
        self.position = Some(filtered);
        self.last_timestamp = Some(timestamp);
        filtered
    }

    pub(crate) fn reset(&mut self) {
        self.position = None;
        self.last_timestamp = None;
    }
}
