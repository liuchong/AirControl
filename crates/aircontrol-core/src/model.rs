pub const JOINT_COUNT: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(usize)]
pub enum JointKind {
    Wrist = 0,
    ThumbTip = 1,
    IndexMcp = 2,
    IndexPip = 3,
    IndexTip = 4,
    MiddleMcp = 5,
    MiddlePip = 6,
    MiddleTip = 7,
    RingMcp = 8,
    RingPip = 9,
    RingTip = 10,
    LittleMcp = 11,
    LittlePip = 12,
    LittleTip = 13,
    ThumbMcp = 14,
    ThumbIp = 15,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Handedness {
    #[default]
    Unknown = 0,
    Left = 1,
    Right = 2,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub confidence: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HandsFrame {
    pub timestamp: f64,
    left: Option<HandFrame>,
    right: Option<HandFrame>,
    unknown: Option<HandFrame>,
}

impl HandsFrame {
    pub fn new(timestamp: f64) -> Self {
        Self {
            timestamp,
            left: None,
            right: None,
            unknown: None,
        }
    }

    pub fn with_hand(mut self, handedness: Handedness, mut hand: HandFrame) -> Self {
        hand.timestamp = self.timestamp;
        match handedness {
            Handedness::Left => self.left = Some(hand),
            Handedness::Right => self.right = Some(hand),
            Handedness::Unknown => self.unknown = Some(hand),
        }
        self
    }

    pub fn left(&self) -> Option<&HandFrame> {
        self.left.as_ref()
    }

    pub fn right(&self) -> Option<&HandFrame> {
        self.right.as_ref()
    }

    pub fn controlling_hand(&self) -> Option<&HandFrame> {
        self.right.as_ref().or(self.unknown.as_ref())
    }
}

impl Point {
    pub const fn new(x: f64, y: f64, confidence: f64) -> Self {
        Self { x, y, confidence }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HandFrame {
    pub timestamp: f64,
    joints: [Option<Point>; JOINT_COUNT],
}

impl HandFrame {
    pub fn empty(timestamp: f64) -> Self {
        Self {
            timestamp,
            joints: [None; JOINT_COUNT],
        }
    }

    pub fn with(mut self, kind: JointKind, point: Point) -> Self {
        self.insert(kind, point);
        self
    }

    pub fn insert(&mut self, kind: JointKind, point: Point) {
        self.joints[kind as usize] = Some(point);
    }

    pub fn point(&self, kind: JointKind, minimum_confidence: f64) -> Option<Point> {
        self.joints[kind as usize].filter(|point| point.confidence >= minimum_confidence)
    }
}
