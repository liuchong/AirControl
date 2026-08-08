use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;

use crate::{
    AssistMode, Button, Calibration, CalibrationStage, CalibrationTracker, CalibrationUpdate,
    Command, Engine, GazeCalibrationTracker, GazeCalibrationUpdate, GazeMapper, GazeProfile,
    GazeSample, HandFrame, Handedness, HandsFrame, JointKind, Point, ScreenRect, Settings,
};

pub const ABI_VERSION: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ACStatus {
    Ok = 0,
    InvalidArgument = 1,
    InvalidSettings = 2,
    BufferTooSmall = 3,
    InternalError = 4,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ACJoint {
    pub kind: u32,
    pub handedness: u32,
    pub x: f64,
    pub y: f64,
    pub confidence: f64,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ACSettings {
    pub version: u32,
    pub enabled_gestures: u32,
    pub smoothing: f64,
    pub minimum_confidence: f64,
    pub pinch_enter: f64,
    pub pinch_exit: f64,
    pub drag_hold_seconds: f64,
    pub fist_hold_seconds: f64,
    pub hand_loss_seconds: f64,
    pub scroll_gain: f64,
    pub scroll_dead_zone: f64,
    pub calibration_min_x: f64,
    pub calibration_max_x: f64,
    pub calibration_min_y: f64,
    pub calibration_max_y: f64,
    pub screen_origin_x: f64,
    pub screen_origin_y: f64,
    pub screen_width: f64,
    pub screen_height: f64,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ACCommand {
    pub kind: u32,
    pub button: u32,
    pub flags: u32,
    pub reserved: u32,
    pub x: f64,
    pub y: f64,
    pub value: f64,
}

pub struct ACEngine {
    inner: Engine,
}

pub struct ACCalibrationTracker {
    inner: CalibrationTracker,
    minimum_confidence: f64,
}

pub struct ACGazeCalibrationTracker {
    inner: GazeCalibrationTracker,
}

pub struct ACGazeMapper {
    inner: GazeMapper,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ACCalibrationResult {
    pub kind: u32,
    pub stage: u32,
    pub progress: f64,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ACGazeSample {
    pub x: f64,
    pub y: f64,
    pub confidence: f64,
    pub eyes_open: u32,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ACGazeProfile {
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

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ACGazeCalibrationResult {
    pub kind: u32,
    pub stage: u32,
    pub progress: f64,
    pub profile: ACGazeProfile,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ACGazePoint {
    pub present: u32,
    pub x: f64,
    pub y: f64,
}

#[unsafe(no_mangle)]
pub extern "C" fn ac_abi_version() -> u32 {
    ABI_VERSION
}

#[unsafe(no_mangle)]
/// Writes versioned default settings into caller-owned memory.
///
/// # Safety
/// `output` must be null or point to writable, properly aligned memory for one `ACSettings`.
pub unsafe extern "C" fn ac_default_settings(output: *mut ACSettings) -> ACStatus {
    ffi_boundary(|| {
        if output.is_null() {
            return ACStatus::InvalidArgument;
        }
        unsafe { ptr::write(output, ACSettings::from(Settings::default())) };
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Creates an engine owned by the caller.
///
/// # Safety
/// `settings` must point to a readable `ACSettings`, and `output` must point to writable memory for
/// one engine pointer. Destroy a returned engine exactly once with [`ac_engine_destroy`].
pub unsafe extern "C" fn ac_engine_create(
    settings: *const ACSettings,
    output: *mut *mut ACEngine,
) -> ACStatus {
    ffi_boundary(|| {
        if settings.is_null() || output.is_null() {
            return ACStatus::InvalidArgument;
        }
        let settings = match Settings::try_from(unsafe { &*settings }) {
            Ok(settings) => settings,
            Err(()) => return ACStatus::InvalidSettings,
        };
        let engine = match Engine::new(settings) {
            Ok(engine) => engine,
            Err(_) => return ACStatus::InvalidSettings,
        };
        unsafe { ptr::write(output, Box::into_raw(Box::new(ACEngine { inner: engine }))) };
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Processes one timestamped set of handed joints and writes abstract commands.
///
/// # Safety
/// `engine` must be live. `joints` must reference `joint_count` readable elements when nonzero.
/// `output` must reference `output_capacity` writable elements when nonzero, and `output_count`
/// must point to writable memory for one `usize`.
pub unsafe extern "C" fn ac_engine_process(
    engine: *mut ACEngine,
    timestamp: f64,
    joints: *const ACJoint,
    joint_count: usize,
    output: *mut ACCommand,
    output_capacity: usize,
    output_count: *mut usize,
) -> ACStatus {
    ffi_boundary(|| {
        if engine.is_null()
            || output_count.is_null()
            || !timestamp.is_finite()
            || (joint_count > 0 && joints.is_null())
            || (output_capacity > 0 && output.is_null())
        {
            return ACStatus::InvalidArgument;
        }
        unsafe { ptr::write(output_count, 0) };
        let input = if joint_count == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(joints, joint_count) }
        };
        let mut left = HandFrame::empty(timestamp);
        let mut right = HandFrame::empty(timestamp);
        let mut unknown = HandFrame::empty(timestamp);
        let mut left_present = false;
        let mut right_present = false;
        let mut unknown_present = false;
        for joint in input {
            let Some(kind) = joint_kind(joint.kind) else {
                return ACStatus::InvalidArgument;
            };
            let Some(handedness) = handedness(joint.handedness) else {
                return ACStatus::InvalidArgument;
            };
            match handedness {
                Handedness::Left => left_present = true,
                Handedness::Right => right_present = true,
                Handedness::Unknown => unknown_present = true,
            }
            if !joint.x.is_finite()
                || !joint.y.is_finite()
                || !joint.confidence.is_finite()
                || !(0.0..=1.0).contains(&joint.x)
                || !(0.0..=1.0).contains(&joint.y)
                || !(0.0..=1.0).contains(&joint.confidence)
            {
                continue;
            }
            let point = Point::new(joint.x, joint.y, joint.confidence);
            match handedness {
                Handedness::Left => left.insert(kind, point),
                Handedness::Right => right.insert(kind, point),
                Handedness::Unknown => unknown.insert(kind, point),
            }
        }
        let mut frame = HandsFrame::new(timestamp);
        if left_present {
            frame = frame.with_hand(Handedness::Left, left);
        }
        if right_present {
            frame = frame.with_hand(Handedness::Right, right);
        }
        if unknown_present {
            frame = frame.with_hand(Handedness::Unknown, unknown);
        }
        let engine = unsafe { &mut *engine };
        let mut candidate = engine.inner.clone();
        let commands = candidate.process_hands(&frame);
        unsafe { ptr::write(output_count, commands.len()) };
        if commands.len() > output_capacity {
            return ACStatus::BufferTooSmall;
        }
        engine.inner = candidate;
        for (index, command) in commands.into_iter().map(ACCommand::from).enumerate() {
            unsafe { ptr::write(output.add(index), command) };
        }
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Stops an engine and writes any required release commands.
///
/// # Safety
/// `engine` must be live. `output` must reference `output_capacity` writable elements when
/// nonzero, and `output_count` must point to writable memory for one `usize`.
pub unsafe extern "C" fn ac_engine_stop(
    engine: *mut ACEngine,
    output: *mut ACCommand,
    output_capacity: usize,
    output_count: *mut usize,
) -> ACStatus {
    ffi_boundary(|| {
        if engine.is_null() || output_count.is_null() || (output_capacity > 0 && output.is_null()) {
            return ACStatus::InvalidArgument;
        }
        let engine = unsafe { &mut *engine };
        let mut candidate = engine.inner.clone();
        let commands = candidate.stop();
        unsafe { ptr::write(output_count, commands.len()) };
        if commands.len() > output_capacity {
            return ACStatus::BufferTooSmall;
        }
        engine.inner = candidate;
        for (index, command) in commands.into_iter().map(ACCommand::from).enumerate() {
            unsafe { ptr::write(output.add(index), command) };
        }
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Makes the next standard hand pointer continue relative to an externally moved cursor.
///
/// # Safety
/// `engine` must be a live engine pointer.
pub unsafe extern "C" fn ac_engine_rebase_pointer(
    engine: *mut ACEngine,
    x: f64,
    y: f64,
) -> ACStatus {
    ffi_boundary(|| {
        if engine.is_null() || !x.is_finite() || !y.is_finite() {
            return ACStatus::InvalidArgument;
        }
        match unsafe { &mut *engine }.inner.rebase_pointer(x, y) {
            Ok(()) => ACStatus::Ok,
            Err(_) => ACStatus::InvalidArgument,
        }
    })
}

#[unsafe(no_mangle)]
/// Clears a pending external-pointer handoff.
///
/// # Safety
/// `engine` must be a live engine pointer.
pub unsafe extern "C" fn ac_engine_clear_pointer_rebase(engine: *mut ACEngine) -> ACStatus {
    ffi_boundary(|| {
        if engine.is_null() {
            return ACStatus::InvalidArgument;
        }
        unsafe { &mut *engine }.inner.clear_pointer_rebase();
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Destroys an engine previously returned to the caller.
///
/// # Safety
/// `engine` must be null or a live, not-yet-destroyed pointer returned by [`ac_engine_create`].
pub unsafe extern "C" fn ac_engine_destroy(engine: *mut ACEngine) {
    if engine.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(engine));
    }));
}

#[unsafe(no_mangle)]
/// Creates a calibration tracker owned by the caller.
///
/// # Safety
/// `minimum_confidence` must be finite and in `0...1`. `output` must point to writable memory for
/// one tracker pointer. Destroy a returned tracker exactly once with [`ac_calibration_destroy`].
pub unsafe extern "C" fn ac_calibration_create(
    minimum_confidence: f64,
    output: *mut *mut ACCalibrationTracker,
) -> ACStatus {
    ffi_boundary(|| {
        if output.is_null()
            || !minimum_confidence.is_finite()
            || !(0.0..=1.0).contains(&minimum_confidence)
        {
            return ACStatus::InvalidArgument;
        }
        let tracker = ACCalibrationTracker {
            inner: CalibrationTracker::new(0.8, 0.025),
            minimum_confidence,
        };
        unsafe { ptr::write(output, Box::into_raw(Box::new(tracker))) };
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Adds one optional calibration point and writes the resulting calibration state.
///
/// # Safety
/// `tracker` must be live, and `output` must point to writable, properly aligned memory for one
/// `ACCalibrationResult`.
pub unsafe extern "C" fn ac_calibration_update(
    tracker: *mut ACCalibrationTracker,
    timestamp: f64,
    point_present: u32,
    x: f64,
    y: f64,
    confidence: f64,
    output: *mut ACCalibrationResult,
) -> ACStatus {
    ffi_boundary(|| {
        if tracker.is_null() || output.is_null() || !timestamp.is_finite() {
            return ACStatus::InvalidArgument;
        }
        let tracker = unsafe { &mut *tracker };
        let point = if point_present == 0 {
            None
        } else if point_present == 1
            && x.is_finite()
            && y.is_finite()
            && confidence.is_finite()
            && (0.0..=1.0).contains(&x)
            && (0.0..=1.0).contains(&y)
            && (0.0..=1.0).contains(&confidence)
        {
            (confidence >= tracker.minimum_confidence).then_some(Point::new(x, y, confidence))
        } else {
            return ACStatus::InvalidArgument;
        };
        let update = tracker.inner.update(timestamp, point);
        unsafe { ptr::write(output, ACCalibrationResult::from(update)) };
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Destroys a calibration tracker previously returned to the caller.
///
/// # Safety
/// `tracker` must be null or a live, not-yet-destroyed pointer returned by
/// [`ac_calibration_create`].
pub unsafe extern "C" fn ac_calibration_destroy(tracker: *mut ACCalibrationTracker) {
    if tracker.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(tracker));
    }));
}

#[unsafe(no_mangle)]
/// Fits a gaze profile from the fixed nine-target sample order.
///
/// # Safety
/// `samples` must reference exactly nine readable samples and `output` one writable profile.
pub unsafe extern "C" fn ac_gaze_profile_fit(
    samples: *const ACGazeSample,
    sample_count: usize,
    output: *mut ACGazeProfile,
) -> ACStatus {
    ffi_boundary(|| {
        if samples.is_null() || sample_count != 9 || output.is_null() {
            return ACStatus::InvalidArgument;
        }
        let input = unsafe { slice::from_raw_parts(samples, sample_count) };
        let mut converted = [GazeSample::new(0.0, 0.0, 0.0, false); 9];
        for (target, sample) in converted.iter_mut().zip(input) {
            let Some(value) = gaze_sample(sample.x, sample.y, sample.confidence, sample.eyes_open)
            else {
                return ACStatus::InvalidArgument;
            };
            *target = value;
        }
        let Some(profile) = GazeProfile::fit(&converted) else {
            return ACStatus::InvalidSettings;
        };
        unsafe { ptr::write(output, ACGazeProfile::from(profile)) };
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Creates the platform-independent nine-point gaze calibration tracker.
///
/// # Safety
/// `output` must point to writable memory for one tracker pointer.
pub unsafe extern "C" fn ac_gaze_calibration_create(
    minimum_confidence: f64,
    output: *mut *mut ACGazeCalibrationTracker,
) -> ACStatus {
    ffi_boundary(|| {
        if output.is_null()
            || !minimum_confidence.is_finite()
            || !(0.0..=1.0).contains(&minimum_confidence)
        {
            return ACStatus::InvalidArgument;
        }
        let tracker = ACGazeCalibrationTracker {
            inner: GazeCalibrationTracker::new(minimum_confidence, 0.65, 0.02),
        };
        unsafe { ptr::write(output, Box::into_raw(Box::new(tracker))) };
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Adds one optional gaze sample to calibration.
///
/// # Safety
/// `tracker` must be live and `output` must point to one writable result.
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn ac_gaze_calibration_update(
    tracker: *mut ACGazeCalibrationTracker,
    timestamp: f64,
    sample_present: u32,
    x: f64,
    y: f64,
    confidence: f64,
    eyes_open: u32,
    output: *mut ACGazeCalibrationResult,
) -> ACStatus {
    ffi_boundary(|| {
        if tracker.is_null() || output.is_null() || !timestamp.is_finite() {
            return ACStatus::InvalidArgument;
        }
        let sample = match sample_present {
            0 => None,
            1 => match gaze_sample(x, y, confidence, eyes_open) {
                Some(sample) => Some(sample),
                None => return ACStatus::InvalidArgument,
            },
            _ => return ACStatus::InvalidArgument,
        };
        let update = unsafe { &mut *tracker }.inner.update(timestamp, sample);
        unsafe { ptr::write(output, ACGazeCalibrationResult::from(update)) };
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Destroys a gaze calibration tracker.
///
/// # Safety
/// `tracker` must be null or a live tracker returned by `ac_gaze_calibration_create`.
pub unsafe extern "C" fn ac_gaze_calibration_destroy(tracker: *mut ACGazeCalibrationTracker) {
    if tracker.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(tracker));
    }));
}

#[unsafe(no_mangle)]
/// Creates a gaze mapper for one target screen.
///
/// # Safety
/// `profile` must point to a readable profile and `output` to one writable mapper pointer.
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn ac_gaze_mapper_create(
    profile: *const ACGazeProfile,
    screen_origin_x: f64,
    screen_origin_y: f64,
    screen_width: f64,
    screen_height: f64,
    minimum_confidence: f64,
    smoothing: f64,
    output: *mut *mut ACGazeMapper,
) -> ACStatus {
    ffi_boundary(|| {
        if profile.is_null() || output.is_null() {
            return ACStatus::InvalidArgument;
        }
        let profile = match GazeProfile::try_from(unsafe { &*profile }) {
            Ok(profile) => profile,
            Err(()) => return ACStatus::InvalidSettings,
        };
        let screen = ScreenRect {
            origin_x: screen_origin_x,
            origin_y: screen_origin_y,
            width: screen_width,
            height: screen_height,
        };
        let mapper = match GazeMapper::new(profile, screen, minimum_confidence, smoothing) {
            Ok(mapper) => mapper,
            Err(_) => return ACStatus::InvalidSettings,
        };
        unsafe {
            ptr::write(
                output,
                Box::into_raw(Box::new(ACGazeMapper { inner: mapper })),
            )
        };
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Maps one optional gaze sample. Missing or untrusted samples return `present = 0`.
///
/// # Safety
/// `mapper` must be live and `output` must point to one writable point.
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn ac_gaze_mapper_update(
    mapper: *mut ACGazeMapper,
    timestamp: f64,
    sample_present: u32,
    x: f64,
    y: f64,
    confidence: f64,
    eyes_open: u32,
    output: *mut ACGazePoint,
) -> ACStatus {
    ffi_boundary(|| {
        if mapper.is_null() || output.is_null() || !timestamp.is_finite() {
            return ACStatus::InvalidArgument;
        }
        let sample = match sample_present {
            0 => None,
            1 => match gaze_sample(x, y, confidence, eyes_open) {
                Some(sample) => Some(sample),
                None => return ACStatus::InvalidArgument,
            },
            _ => return ACStatus::InvalidArgument,
        };
        let point = unsafe { &mut *mapper }.inner.update(timestamp, sample);
        unsafe {
            ptr::write(
                output,
                point.map_or_else(ACGazePoint::default, |point| ACGazePoint {
                    present: 1,
                    x: point.0,
                    y: point.1,
                }),
            )
        };
        ACStatus::Ok
    })
}

#[unsafe(no_mangle)]
/// Destroys a gaze mapper.
///
/// # Safety
/// `mapper` must be null or a live mapper returned by `ac_gaze_mapper_create`.
pub unsafe extern "C" fn ac_gaze_mapper_destroy(mapper: *mut ACGazeMapper) {
    if mapper.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(mapper));
    }));
}

fn ffi_boundary(operation: impl FnOnce() -> ACStatus) -> ACStatus {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(ACStatus::InternalError)
}

fn joint_kind(raw: u32) -> Option<JointKind> {
    Some(match raw {
        0 => JointKind::Wrist,
        1 => JointKind::ThumbTip,
        2 => JointKind::IndexMcp,
        3 => JointKind::IndexPip,
        4 => JointKind::IndexTip,
        5 => JointKind::MiddleMcp,
        6 => JointKind::MiddlePip,
        7 => JointKind::MiddleTip,
        8 => JointKind::RingMcp,
        9 => JointKind::RingPip,
        10 => JointKind::RingTip,
        11 => JointKind::LittleMcp,
        12 => JointKind::LittlePip,
        13 => JointKind::LittleTip,
        14 => JointKind::ThumbMcp,
        15 => JointKind::ThumbIp,
        _ => return None,
    })
}

fn handedness(raw: u32) -> Option<Handedness> {
    Some(match raw {
        0 => Handedness::Unknown,
        1 => Handedness::Left,
        2 => Handedness::Right,
        _ => return None,
    })
}

fn gaze_sample(x: f64, y: f64, confidence: f64, eyes_open: u32) -> Option<GazeSample> {
    if !x.is_finite()
        || !y.is_finite()
        || !confidence.is_finite()
        || !(0.0..=1.0).contains(&x)
        || !(0.0..=1.0).contains(&y)
        || !(0.0..=1.0).contains(&confidence)
        || eyes_open > 1
    {
        return None;
    }
    Some(GazeSample::new(x, y, confidence, eyes_open == 1))
}

impl From<Settings> for ACSettings {
    fn from(value: Settings) -> Self {
        Self {
            version: value.version,
            enabled_gestures: value.enabled_gestures,
            smoothing: value.smoothing,
            minimum_confidence: value.minimum_confidence,
            pinch_enter: value.pinch_enter,
            pinch_exit: value.pinch_exit,
            drag_hold_seconds: value.drag_hold_seconds,
            fist_hold_seconds: value.fist_hold_seconds,
            hand_loss_seconds: value.hand_loss_seconds,
            scroll_gain: value.scroll_gain,
            scroll_dead_zone: value.scroll_dead_zone,
            calibration_min_x: value.calibration.min_x,
            calibration_max_x: value.calibration.max_x,
            calibration_min_y: value.calibration.min_y,
            calibration_max_y: value.calibration.max_y,
            screen_origin_x: value.screen.origin_x,
            screen_origin_y: value.screen.origin_y,
            screen_width: value.screen.width,
            screen_height: value.screen.height,
        }
    }
}

impl From<GazeProfile> for ACGazeProfile {
    fn from(value: GazeProfile) -> Self {
        Self {
            x_bias: value.x_bias,
            x_from_x: value.x_from_x,
            x_from_y: value.x_from_y,
            y_bias: value.y_bias,
            y_from_x: value.y_from_x,
            y_from_y: value.y_from_y,
            raw_min_x: value.raw_min_x,
            raw_max_x: value.raw_max_x,
            raw_min_y: value.raw_min_y,
            raw_max_y: value.raw_max_y,
        }
    }
}

impl TryFrom<&ACGazeProfile> for GazeProfile {
    type Error = ();

    fn try_from(value: &ACGazeProfile) -> Result<Self, Self::Error> {
        let profile = Self {
            x_bias: value.x_bias,
            x_from_x: value.x_from_x,
            x_from_y: value.x_from_y,
            y_bias: value.y_bias,
            y_from_x: value.y_from_x,
            y_from_y: value.y_from_y,
            raw_min_x: value.raw_min_x,
            raw_max_x: value.raw_max_x,
            raw_min_y: value.raw_min_y,
            raw_max_y: value.raw_max_y,
        };
        profile.is_valid().then_some(profile).ok_or(())
    }
}

impl From<GazeCalibrationUpdate> for ACGazeCalibrationResult {
    fn from(value: GazeCalibrationUpdate) -> Self {
        match value {
            GazeCalibrationUpdate::Progress { stage, progress } => Self {
                kind: 1,
                stage: stage as u32,
                progress,
                profile: ACGazeProfile::default(),
            },
            GazeCalibrationUpdate::TargetCaptured { stage } => Self {
                kind: 2,
                stage: stage as u32,
                progress: 1.0,
                profile: ACGazeProfile::default(),
            },
            GazeCalibrationUpdate::Completed(profile) => Self {
                kind: 3,
                stage: 8,
                progress: 1.0,
                profile: profile.into(),
            },
            GazeCalibrationUpdate::InvalidProfile => Self {
                kind: 4,
                stage: 8,
                progress: 0.0,
                profile: ACGazeProfile::default(),
            },
        }
    }
}

impl TryFrom<&ACSettings> for Settings {
    type Error = ();

    fn try_from(value: &ACSettings) -> Result<Self, Self::Error> {
        let settings = Self {
            version: value.version,
            smoothing: value.smoothing,
            minimum_confidence: value.minimum_confidence,
            pinch_enter: value.pinch_enter,
            pinch_exit: value.pinch_exit,
            drag_hold_seconds: value.drag_hold_seconds,
            fist_hold_seconds: value.fist_hold_seconds,
            hand_loss_seconds: value.hand_loss_seconds,
            scroll_gain: value.scroll_gain,
            scroll_dead_zone: value.scroll_dead_zone,
            calibration: Calibration {
                min_x: value.calibration_min_x,
                max_x: value.calibration_max_x,
                min_y: value.calibration_min_y,
                max_y: value.calibration_max_y,
            },
            screen: ScreenRect {
                origin_x: value.screen_origin_x,
                origin_y: value.screen_origin_y,
                width: value.screen_width,
                height: value.screen_height,
            },
            enabled_gestures: value.enabled_gestures,
        };
        settings.validate().map_err(|_| ())?;
        Ok(settings)
    }
}

impl From<Command> for ACCommand {
    fn from(value: Command) -> Self {
        match value {
            Command::Move { x, y } => Self {
                kind: 1,
                x,
                y,
                ..Self::default()
            },
            Command::Click {
                button,
                x,
                y,
                count,
            } => Self {
                kind: 6,
                button: button_code(button),
                flags: u32::from(count),
                x,
                y,
                ..Self::default()
            },
            Command::MouseDown { button, x, y } => Self {
                kind: 2,
                button: button_code(button),
                x,
                y,
                ..Self::default()
            },
            Command::MouseUp { button, x, y } => Self {
                kind: 3,
                button: button_code(button),
                x,
                y,
                ..Self::default()
            },
            Command::Scroll { delta_y } => Self {
                kind: 4,
                value: delta_y,
                ..Self::default()
            },
            Command::PauseChanged { paused } => Self {
                kind: 5,
                flags: u32::from(paused),
                ..Self::default()
            },
            Command::AssistChanged { mode } => Self {
                kind: 7,
                flags: assist_mode_code(mode),
                ..Self::default()
            },
            Command::WindowGrabBegin { x, y } => Self {
                kind: 8,
                x,
                y,
                ..Self::default()
            },
            Command::WindowMove { x, y } => Self {
                kind: 9,
                x,
                y,
                ..Self::default()
            },
            Command::WindowGrabEnd => Self {
                kind: 10,
                ..Self::default()
            },
        }
    }
}

const fn assist_mode_code(mode: AssistMode) -> u32 {
    mode as u32
}

const fn button_code(button: Button) -> u32 {
    match button {
        Button::Left => 1,
        Button::Right => 2,
    }
}

impl From<CalibrationUpdate> for ACCalibrationResult {
    fn from(value: CalibrationUpdate) -> Self {
        match value {
            CalibrationUpdate::Progress { stage, progress } => Self {
                kind: 1,
                stage: stage_code(stage),
                progress,
                ..Self::default()
            },
            CalibrationUpdate::CornerCaptured(stage) => Self {
                kind: 2,
                stage: stage_code(stage),
                progress: 1.0,
                ..Self::default()
            },
            CalibrationUpdate::Completed(calibration) => Self {
                kind: 3,
                min_x: calibration.min_x,
                max_x: calibration.max_x,
                min_y: calibration.min_y,
                max_y: calibration.max_y,
                ..Self::default()
            },
            CalibrationUpdate::InvalidRange => Self {
                kind: 4,
                ..Self::default()
            },
        }
    }
}

const fn stage_code(stage: CalibrationStage) -> u32 {
    match stage {
        CalibrationStage::TopLeft => 1,
        CalibrationStage::BottomRight => 2,
    }
}
