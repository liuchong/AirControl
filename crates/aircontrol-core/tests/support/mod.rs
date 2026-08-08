#![allow(dead_code)]

use aircontrol_core::{HandFrame, Handedness, HandsFrame, JointKind, Point};

pub fn hand(timestamp: f64, primary_pinch: bool, secondary_pinch: bool) -> HandFrame {
    let index_tip = Point::new(0.44, 0.82, 1.0);
    let middle_tip = Point::new(0.55, 0.78, 1.0);
    let thumb_tip = if primary_pinch {
        Point::new(0.445, 0.815, 1.0)
    } else if secondary_pinch {
        Point::new(0.555, 0.775, 1.0)
    } else {
        Point::new(0.25, 0.55, 1.0)
    };

    HandFrame::empty(timestamp)
        .with(JointKind::Wrist, Point::new(0.50, 0.20, 1.0))
        .with(JointKind::ThumbTip, thumb_tip)
        .with(JointKind::IndexMcp, Point::new(0.44, 0.42, 1.0))
        .with(JointKind::IndexPip, Point::new(0.44, 0.62, 1.0))
        .with(JointKind::IndexTip, index_tip)
        .with(JointKind::MiddleMcp, Point::new(0.53, 0.44, 1.0))
        .with(JointKind::MiddlePip, Point::new(0.54, 0.60, 1.0))
        .with(JointKind::MiddleTip, middle_tip)
        .with(JointKind::RingMcp, Point::new(0.59, 0.42, 1.0))
        .with(JointKind::RingPip, Point::new(0.60, 0.53, 1.0))
        .with(JointKind::RingTip, Point::new(0.60, 0.48, 1.0))
        .with(JointKind::LittleMcp, Point::new(0.64, 0.39, 1.0))
        .with(JointKind::LittlePip, Point::new(0.66, 0.49, 1.0))
        .with(JointKind::LittleTip, Point::new(0.66, 0.44, 1.0))
}

pub fn fist(timestamp: f64) -> HandFrame {
    hand(timestamp, false, false)
        .with(JointKind::IndexTip, Point::new(0.44, 0.55, 1.0))
        .with(JointKind::MiddleTip, Point::new(0.54, 0.53, 1.0))
        .with(JointKind::RingTip, Point::new(0.60, 0.48, 1.0))
        .with(JointKind::LittleTip, Point::new(0.66, 0.44, 1.0))
}

pub fn pointer(timestamp: f64, x: f64, y: f64) -> HandFrame {
    hand(timestamp, false, false)
        .with(JointKind::IndexTip, Point::new(x, y, 1.0))
        .with(JointKind::MiddleTip, Point::new(0.54, 0.52, 1.0))
}

pub fn horizontal_pointer(timestamp: f64, x: f64) -> HandFrame {
    HandFrame::empty(timestamp)
        .with(JointKind::Wrist, Point::new(0.15, 0.50, 1.0))
        .with(JointKind::ThumbTip, Point::new(0.38, 0.72, 1.0))
        .with(JointKind::IndexMcp, Point::new(0.38, 0.56, 1.0))
        .with(JointKind::IndexPip, Point::new(0.58, 0.56, 1.0))
        .with(JointKind::IndexTip, Point::new(x, 0.56, 1.0))
        .with(JointKind::MiddleMcp, Point::new(0.40, 0.49, 1.0))
        .with(JointKind::MiddlePip, Point::new(0.55, 0.49, 1.0))
        .with(JointKind::MiddleTip, Point::new(0.48, 0.49, 1.0))
        .with(JointKind::RingMcp, Point::new(0.39, 0.44, 1.0))
        .with(JointKind::RingPip, Point::new(0.52, 0.44, 1.0))
        .with(JointKind::RingTip, Point::new(0.46, 0.44, 1.0))
        .with(JointKind::LittleMcp, Point::new(0.36, 0.39, 1.0))
        .with(JointKind::LittlePip, Point::new(0.48, 0.39, 1.0))
        .with(JointKind::LittleTip, Point::new(0.42, 0.39, 1.0))
}

pub fn hands(timestamp: f64, left: Option<HandFrame>, right: Option<HandFrame>) -> HandsFrame {
    let mut frame = HandsFrame::new(timestamp);
    if let Some(left) = left {
        frame = frame.with_hand(Handedness::Left, left);
    }
    if let Some(right) = right {
        frame = frame.with_hand(Handedness::Right, right);
    }
    frame
}

pub fn unknown_hand(timestamp: f64, hand: HandFrame) -> HandsFrame {
    HandsFrame::new(timestamp).with_hand(Handedness::Unknown, hand)
}

pub fn open_palm(timestamp: f64) -> HandFrame {
    hand(timestamp, false, false)
        .with(JointKind::ThumbMcp, Point::new(0.38, 0.40, 1.0))
        .with(JointKind::ThumbIp, Point::new(0.31, 0.51, 1.0))
        .with(JointKind::RingPip, Point::new(0.60, 0.58, 1.0))
        .with(JointKind::RingTip, Point::new(0.61, 0.76, 1.0))
        .with(JointKind::LittlePip, Point::new(0.66, 0.55, 1.0))
        .with(JointKind::LittleTip, Point::new(0.68, 0.72, 1.0))
}

pub fn index_only(timestamp: f64) -> HandFrame {
    hand(timestamp, false, false)
        .with(JointKind::ThumbMcp, Point::new(0.43, 0.39, 1.0))
        .with(JointKind::ThumbIp, Point::new(0.47, 0.43, 1.0))
        .with(JointKind::MiddleTip, Point::new(0.54, 0.53, 1.0))
}

pub fn v_sign(timestamp: f64) -> HandFrame {
    hand(timestamp, false, false)
        .with(JointKind::ThumbMcp, Point::new(0.43, 0.39, 1.0))
        .with(JointKind::ThumbIp, Point::new(0.47, 0.43, 1.0))
}

pub fn three_fingers(timestamp: f64) -> HandFrame {
    hand(timestamp, false, false)
        .with(JointKind::ThumbMcp, Point::new(0.43, 0.39, 1.0))
        .with(JointKind::ThumbIp, Point::new(0.47, 0.43, 1.0))
        .with(JointKind::RingPip, Point::new(0.60, 0.58, 1.0))
        .with(JointKind::RingTip, Point::new(0.61, 0.76, 1.0))
}

pub fn thumbs_up(timestamp: f64) -> HandFrame {
    fist(timestamp)
        .with(JointKind::ThumbMcp, Point::new(0.39, 0.40, 1.0))
        .with(JointKind::ThumbIp, Point::new(0.37, 0.56, 1.0))
        .with(JointKind::ThumbTip, Point::new(0.35, 0.75, 1.0))
}

pub fn left_fist(timestamp: f64) -> HandFrame {
    fist(timestamp)
        .with(JointKind::ThumbMcp, Point::new(0.43, 0.39, 1.0))
        .with(JointKind::ThumbIp, Point::new(0.48, 0.43, 1.0))
        .with(JointKind::ThumbTip, Point::new(0.53, 0.45, 1.0))
}

pub fn shifted_palm(mut frame: HandFrame, x: f64, y: f64) -> HandFrame {
    frame = frame.with(JointKind::Wrist, Point::new(x, y, 1.0));
    frame.with(JointKind::MiddleMcp, Point::new(x + 0.03, y + 0.24, 1.0))
}
