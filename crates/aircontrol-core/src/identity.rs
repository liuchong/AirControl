use crate::{HandFrame, Handedness, HandsFrame, JointKind, Point};

const OVERLAP_DISTANCE: f64 = 0.12;

#[derive(Clone, Debug, Default)]
pub(crate) struct HandIdentity {
    left_wrist: Option<Point>,
    right_wrist: Option<Point>,
}

#[derive(Clone, Debug)]
pub(crate) struct StabilizedHands {
    pub frame: HandsFrame,
    pub overlapped: bool,
}

impl HandIdentity {
    pub fn update(&mut self, frame: &HandsFrame) -> StabilizedHands {
        let left = frame.left().cloned();
        let right = frame.right().cloned();
        let unknown = frame.unknown().cloned();
        let left_wrist = left.as_ref().and_then(wrist);
        let right_wrist = right.as_ref().and_then(wrist);
        let overlapped = left_wrist
            .zip(right_wrist)
            .is_some_and(|(left, right)| distance(left, right) < OVERLAP_DISTANCE);

        if overlapped {
            let (left, right) = keep_nearest_wrists(self.left_wrist, self.right_wrist, left, right);
            self.left_wrist = left.as_ref().and_then(wrist);
            self.right_wrist = right.as_ref().and_then(wrist);
            return StabilizedHands {
                frame: assemble(frame.timestamp, left, right, unknown),
                overlapped: true,
            };
        }

        let (left, right) = keep_nearest_wrists(self.left_wrist, self.right_wrist, left, right);

        self.left_wrist = left.as_ref().and_then(wrist);
        self.right_wrist = right.as_ref().and_then(wrist);
        StabilizedHands {
            frame: assemble(frame.timestamp, left, right, unknown),
            overlapped: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

fn assemble(
    timestamp: f64,
    left: Option<HandFrame>,
    right: Option<HandFrame>,
    unknown: Option<HandFrame>,
) -> HandsFrame {
    let mut frame = HandsFrame::new(timestamp);
    if let Some(left) = left {
        frame = frame.with_hand(Handedness::Left, left);
    }
    if let Some(right) = right {
        frame = frame.with_hand(Handedness::Right, right);
    }
    if let Some(unknown) = unknown {
        frame = frame.with_hand(Handedness::Unknown, unknown);
    }
    frame
}

fn keep_nearest_wrists(
    previous_left: Option<Point>,
    previous_right: Option<Point>,
    left: Option<HandFrame>,
    right: Option<HandFrame>,
) -> (Option<HandFrame>, Option<HandFrame>) {
    let Some(previous_left) = previous_left else {
        return (left, right);
    };
    let Some(previous_right) = previous_right else {
        return (left, right);
    };
    let Some(left) = left else {
        return (None, right);
    };
    let Some(right) = right else {
        return (Some(left), None);
    };
    let (Some(left_now), Some(right_now)) = (wrist(&left), wrist(&right)) else {
        return (Some(left), Some(right));
    };
    let direct = distance(left_now, previous_left) + distance(right_now, previous_right);
    let crossed = distance(left_now, previous_right) + distance(right_now, previous_left);
    if crossed + 0.02 < direct {
        (Some(right), Some(left))
    } else {
        (Some(left), Some(right))
    }
}

fn wrist(frame: &HandFrame) -> Option<Point> {
    frame.point(JointKind::Wrist, 0.2)
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
