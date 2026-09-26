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
        // Ordinary pointing is only a few hundredths of the frame per sample.
        // The fast response has to begin there; reserving it for very large
        // jumps makes normal motion feel late and then catch up in steps.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_motion_follows_while_tiny_jitter_stays_slower() {
        let start = Point::new(0.40, 0.50, 1.0);
        let mut moving = CursorFilter::default();
        let mut jitter = CursorFilter::default();
        moving.update(start, 0.0, 0.32);
        jitter.update(start, 0.0, 0.32);

        let moved = moving.update(Point::new(0.50, 0.50, 1.0), 0.10, 0.32);
        let held = jitter.update(Point::new(0.404, 0.50, 1.0), 0.10, 0.32);
        let move_ratio = (moved.x - start.x) / 0.10;
        let jitter_ratio = (held.x - start.x) / 0.004;

        assert!(
            move_ratio > jitter_ratio + 0.25,
            "ordinary motion should catch up faster than jitter: move={move_ratio}, jitter={jitter_ratio}"
        );
        assert!(
            jitter_ratio < 0.7,
            "tiny jitter must remain damped, ratio={jitter_ratio}"
        );
    }
}
