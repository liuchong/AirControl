use std::ptr;

use aircontrol_core::ffi::{
    ABI_VERSION, ACCalibrationResult, ACCalibrationTracker, ACCommand, ACEngine,
    ACGazeCalibrationResult, ACGazeCalibrationTracker, ACGazeMapper, ACGazePoint, ACGazeProfile,
    ACGazeSample, ACJoint, ACSettings, ACStatus, ac_abi_version, ac_calibration_create,
    ac_calibration_destroy, ac_calibration_update, ac_default_settings,
    ac_engine_clear_pointer_rebase, ac_engine_create, ac_engine_destroy, ac_engine_process,
    ac_engine_rebase_pointer, ac_engine_stop, ac_gaze_calibration_create,
    ac_gaze_calibration_destroy, ac_gaze_calibration_update, ac_gaze_mapper_create,
    ac_gaze_mapper_destroy, ac_gaze_mapper_update, ac_gaze_profile_fit,
};

#[test]
fn abi_version_is_explicit_and_stable() {
    assert_eq!(ABI_VERSION, 4);
    assert_eq!(ac_abi_version(), ABI_VERSION);
}

#[test]
fn calibration_ffi_tracks_stable_points_without_platform_logic() {
    let mut tracker: *mut ACCalibrationTracker = ptr::null_mut();
    assert_eq!(
        unsafe { ac_calibration_create(0.35, &mut tracker) },
        ACStatus::Ok
    );
    let mut result = ACCalibrationResult::default();
    assert_eq!(
        unsafe { ac_calibration_update(tracker, 0.0, 1, 0.2, 0.8, 1.0, &mut result) },
        ACStatus::Ok
    );
    assert_eq!(result.kind, 1);
    assert_eq!(
        unsafe { ac_calibration_update(tracker, 0.81, 1, 0.2, 0.8, 1.0, &mut result) },
        ACStatus::Ok
    );
    assert_eq!(result.kind, 2);
    assert_eq!(
        unsafe { ac_calibration_update(tracker, 1.0, 1, 0.8, 0.2, 1.0, &mut result) },
        ACStatus::Ok
    );
    assert_eq!(
        unsafe { ac_calibration_update(tracker, 1.81, 1, 0.8, 0.2, 1.0, &mut result) },
        ACStatus::Ok
    );
    assert_eq!(result.kind, 3);
    assert!(result.max_x - result.min_x >= 0.25);
    unsafe { ac_calibration_destroy(tracker) };
}

#[test]
fn calibration_ffi_rejects_points_below_rust_confidence_threshold() {
    let mut tracker: *mut ACCalibrationTracker = ptr::null_mut();
    assert_eq!(
        unsafe { ac_calibration_create(0.8, &mut tracker) },
        ACStatus::Ok
    );
    let mut result = ACCalibrationResult::default();
    assert_eq!(
        unsafe { ac_calibration_update(tracker, 0.0, 1, 0.2, 0.8, 0.79, &mut result) },
        ACStatus::Ok
    );
    assert_eq!(
        unsafe { ac_calibration_update(tracker, 1.0, 1, 0.2, 0.8, 0.79, &mut result) },
        ACStatus::Ok
    );
    assert_eq!(result.kind, 1);
    assert_eq!(result.progress, 0.0);
    unsafe { ac_calibration_destroy(tracker) };
}

#[test]
fn null_output_pointers_are_rejected_without_panicking() {
    assert_eq!(
        unsafe { ac_default_settings(ptr::null_mut()) },
        ACStatus::InvalidArgument
    );
    let settings = default_ffi_settings();
    assert_eq!(
        unsafe { ac_engine_create(&settings, ptr::null_mut()) },
        ACStatus::InvalidArgument
    );
}

#[test]
fn ffi_keeps_pointer_when_an_unrelated_joint_measurement_is_invalid() {
    let settings = default_ffi_settings();
    let mut engine: *mut ACEngine = ptr::null_mut();
    assert_eq!(
        unsafe { ac_engine_create(&settings, &mut engine) },
        ACStatus::Ok
    );

    let mut frame = joints(false);
    frame[10].x = 1.02;
    let mut commands = [ACCommand::default(); 8];
    let mut count = 0_usize;
    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.0,
                frame.as_ptr(),
                frame.len(),
                commands.as_mut_ptr(),
                commands.len(),
                &mut count,
            )
        },
        ACStatus::Ok
    );
    assert_eq!(
        count, 1,
        "an unrelated missing joint must not drop a pointer frame"
    );
    assert_eq!(commands[0].kind, 1);
    unsafe { ac_engine_destroy(engine) };
}

#[test]
fn ffi_still_rejects_unknown_joint_kinds() {
    let settings = default_ffi_settings();
    let mut engine: *mut ACEngine = ptr::null_mut();
    assert_eq!(
        unsafe { ac_engine_create(&settings, &mut engine) },
        ACStatus::Ok
    );

    let mut frame = joints(false);
    frame[10].kind = 99;
    let mut commands = [ACCommand::default(); 8];
    let mut count = 0_usize;
    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.0,
                frame.as_ptr(),
                frame.len(),
                commands.as_mut_ptr(),
                commands.len(),
                &mut count,
            )
        },
        ACStatus::InvalidArgument
    );
    unsafe { ac_engine_destroy(engine) };
}

#[test]
fn ffi_rejects_unknown_handedness_values() {
    let settings = default_ffi_settings();
    let mut engine: *mut ACEngine = ptr::null_mut();
    assert_eq!(
        unsafe { ac_engine_create(&settings, &mut engine) },
        ACStatus::Ok
    );

    let mut frame = joints(false);
    frame[0].handedness = 99;
    let mut commands = [ACCommand::default(); 8];
    let mut count = 0_usize;
    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.0,
                frame.as_ptr(),
                frame.len(),
                commands.as_mut_ptr(),
                commands.len(),
                &mut count,
            )
        },
        ACStatus::InvalidArgument
    );
    unsafe { ac_engine_destroy(engine) };
}

#[test]
fn ffi_processes_short_pinch_and_reports_required_capacity() {
    let settings = default_ffi_settings();
    let mut engine: *mut ACEngine = ptr::null_mut();
    assert_eq!(
        unsafe { ac_engine_create(&settings, &mut engine) },
        ACStatus::Ok
    );
    assert!(!engine.is_null());

    let pinched = joints(true);
    let open = joints(false);
    let mut commands = [ACCommand::default(); 8];
    let mut count = 0_usize;
    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.0,
                pinched.as_ptr(),
                pinched.len(),
                commands.as_mut_ptr(),
                commands.len(),
                &mut count,
            )
        },
        ACStatus::Ok
    );
    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.05,
                pinched.as_ptr(),
                pinched.len(),
                commands.as_mut_ptr(),
                commands.len(),
                &mut count,
            )
        },
        ACStatus::Ok
    );

    count = 0;
    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.20,
                open.as_ptr(),
                open.len(),
                commands.as_mut_ptr(),
                commands.len(),
                &mut count,
            )
        },
        ACStatus::Ok
    );
    assert_eq!(count, 0, "one open sample must not release a noisy pinch");

    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.24,
                open.as_ptr(),
                open.len(),
                ptr::null_mut(),
                0,
                &mut count,
            )
        },
        ACStatus::BufferTooSmall
    );
    assert_eq!(count, 1);

    let mut retry_commands = [ACCommand::default(); 8];
    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.24,
                open.as_ptr(),
                open.len(),
                retry_commands.as_mut_ptr(),
                retry_commands.len(),
                &mut count,
            )
        },
        ACStatus::Ok
    );
    assert_eq!(count, 1, "capacity failure must not consume click commands");
    assert_eq!(retry_commands[0].kind, 6);
    assert_eq!(retry_commands[0].button, 1);
    assert_eq!(retry_commands[0].flags, 1);
    unsafe { ac_engine_destroy(engine) };
}

#[test]
fn ffi_stop_capacity_failure_does_not_consume_drag_release() {
    let settings = default_ffi_settings();
    let mut engine: *mut ACEngine = ptr::null_mut();
    assert_eq!(
        unsafe { ac_engine_create(&settings, &mut engine) },
        ACStatus::Ok
    );

    let pinched = joints(true);
    let mut commands = [ACCommand::default(); 8];
    let mut count = 0_usize;
    for timestamp in [0.0, 0.05, 0.36] {
        assert_eq!(
            unsafe {
                ac_engine_process(
                    engine,
                    timestamp,
                    pinched.as_ptr(),
                    pinched.len(),
                    commands.as_mut_ptr(),
                    commands.len(),
                    &mut count,
                )
            },
            ACStatus::Ok
        );
    }

    assert_eq!(
        unsafe { ac_engine_stop(engine, ptr::null_mut(), 0, &mut count) },
        ACStatus::BufferTooSmall
    );
    assert_eq!(count, 1);
    assert_eq!(
        unsafe { ac_engine_stop(engine, commands.as_mut_ptr(), commands.len(), &mut count,) },
        ACStatus::Ok
    );
    assert_eq!(count, 1, "capacity failure must not consume drag release");
    assert_eq!(commands[0].kind, 3);
    assert_eq!(commands[0].button, 1);
    unsafe { ac_engine_destroy(engine) };
}

#[test]
fn ffi_window_grab_commands_are_transactional_through_process_and_stop() {
    let settings = default_ffi_settings();
    let mut engine: *mut ACEngine = ptr::null_mut();
    assert_eq!(
        unsafe { ac_engine_create(&settings, &mut engine) },
        ACStatus::Ok
    );

    let open = open_joints();
    let closing = closing_joints();
    let fist = fist_joints();
    let mut commands = [ACCommand::default(); 8];
    let mut count = 0_usize;
    for (timestamp, frame) in [
        (0.00, &open),
        (0.13, &open),
        (0.20, &closing),
        (0.25, &fist),
    ] {
        assert_eq!(
            unsafe {
                ac_engine_process(
                    engine,
                    timestamp,
                    frame.as_ptr(),
                    frame.len(),
                    commands.as_mut_ptr(),
                    commands.len(),
                    &mut count,
                )
            },
            ACStatus::Ok
        );
    }

    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.29,
                fist.as_ptr(),
                fist.len(),
                ptr::null_mut(),
                0,
                &mut count,
            )
        },
        ACStatus::BufferTooSmall
    );
    assert_eq!(count, 1);
    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.29,
                fist.as_ptr(),
                fist.len(),
                commands.as_mut_ptr(),
                commands.len(),
                &mut count,
            )
        },
        ACStatus::Ok
    );
    assert_eq!(count, 1, "capacity failure must not consume grab begin");
    assert_eq!(commands[0].kind, 8);

    let moved = shifted_joints(fist.clone(), 0.04, 0.03);
    assert_eq!(
        unsafe {
            ac_engine_process(
                engine,
                0.36,
                moved.as_ptr(),
                moved.len(),
                commands.as_mut_ptr(),
                commands.len(),
                &mut count,
            )
        },
        ACStatus::Ok
    );
    assert_eq!(count, 1);
    assert_eq!(commands[0].kind, 9);

    assert_eq!(
        unsafe { ac_engine_stop(engine, ptr::null_mut(), 0, &mut count) },
        ACStatus::BufferTooSmall
    );
    assert_eq!(count, 1);
    assert_eq!(
        unsafe { ac_engine_stop(engine, commands.as_mut_ptr(), commands.len(), &mut count) },
        ACStatus::Ok
    );
    assert_eq!(count, 1, "capacity failure must not consume grab release");
    assert_eq!(commands[0].kind, 10);
    unsafe { ac_engine_destroy(engine) };
}

#[test]
fn gaze_profile_mapping_and_engine_rebase_cross_the_ffi_boundary() {
    let mut samples = [ACGazeSample::default(); 9];
    for (stage, sample) in samples.iter_mut().enumerate() {
        sample.x = [0.32, 0.50, 0.68][stage % 3];
        sample.y = [0.34, 0.50, 0.66][stage / 3];
        sample.confidence = 0.95;
        sample.eyes_open = 1;
    }
    let mut profile = ACGazeProfile::default();
    assert_eq!(
        unsafe { ac_gaze_profile_fit(samples.as_ptr(), samples.len(), &mut profile) },
        ACStatus::Ok
    );

    let mut mapper: *mut ACGazeMapper = ptr::null_mut();
    assert_eq!(
        unsafe {
            ac_gaze_mapper_create(&profile, 0.0, 0.0, 1440.0, 900.0, 0.55, 0.35, &mut mapper)
        },
        ACStatus::Ok
    );
    let mut point = ACGazePoint::default();
    assert_eq!(
        unsafe { ac_gaze_mapper_update(mapper, 0.0, 1, 0.50, 0.50, 0.95, 1, &mut point) },
        ACStatus::Ok
    );
    assert_eq!(point.present, 1);
    assert!((point.x - 720.0).abs() < 2.0);
    assert!((point.y - 450.0).abs() < 2.0);

    assert_eq!(
        unsafe { ac_gaze_mapper_update(mapper, 0.1, 1, 0.50, 0.50, 0.95, 0, &mut point) },
        ACStatus::Ok
    );
    assert_eq!(
        point.present, 0,
        "blink freezes instead of producing a point"
    );
    unsafe { ac_gaze_mapper_destroy(mapper) };

    let settings = default_ffi_settings();
    let mut engine: *mut ACEngine = ptr::null_mut();
    assert_eq!(
        unsafe { ac_engine_create(&settings, &mut engine) },
        ACStatus::Ok
    );
    assert_eq!(
        unsafe { ac_engine_rebase_pointer(engine, 720.0, 450.0) },
        ACStatus::Ok
    );
    assert_eq!(
        unsafe { ac_engine_clear_pointer_rebase(engine) },
        ACStatus::Ok
    );
    assert_eq!(
        unsafe { ac_engine_rebase_pointer(engine, f64::NAN, 0.0) },
        ACStatus::InvalidArgument
    );
    unsafe { ac_engine_destroy(engine) };
}

#[test]
fn gaze_calibration_ffi_requires_stable_open_eyes() {
    let mut tracker: *mut ACGazeCalibrationTracker = ptr::null_mut();
    assert_eq!(
        unsafe { ac_gaze_calibration_create(0.55, &mut tracker) },
        ACStatus::Ok
    );
    let mut result = ACGazeCalibrationResult::default();
    assert_eq!(
        unsafe { ac_gaze_calibration_update(tracker, 0.0, 1, 0.32, 0.34, 0.95, 1, &mut result) },
        ACStatus::Ok
    );
    assert_eq!(result.kind, 1);
    assert_eq!(
        unsafe { ac_gaze_calibration_update(tracker, 0.7, 1, 0.32, 0.34, 0.95, 0, &mut result) },
        ACStatus::Ok
    );
    assert_eq!(result.kind, 1);
    assert_eq!(result.progress, 0.0);
    unsafe { ac_gaze_calibration_destroy(tracker) };
}

fn default_ffi_settings() -> ACSettings {
    let mut settings = ACSettings::default();
    assert_eq!(unsafe { ac_default_settings(&mut settings) }, ACStatus::Ok);
    settings
}

fn joints(pinched: bool) -> Vec<ACJoint> {
    let thumb = if pinched {
        (0.445, 0.815)
    } else {
        (0.25, 0.55)
    };
    vec![
        joint(0, 0.50, 0.20),
        joint(1, thumb.0, thumb.1),
        joint(2, 0.44, 0.42),
        joint(3, 0.44, 0.62),
        joint(4, 0.44, 0.82),
        joint(5, 0.53, 0.44),
        joint(6, 0.54, 0.60),
        joint(7, 0.55, 0.78),
        joint(8, 0.59, 0.42),
        joint(9, 0.60, 0.53),
        joint(10, 0.60, 0.48),
        joint(11, 0.64, 0.39),
        joint(12, 0.66, 0.49),
        joint(13, 0.66, 0.44),
    ]
}

fn open_joints() -> Vec<ACJoint> {
    let mut frame = joints(false);
    set_joint(&mut frame, 9, 0.60, 0.58);
    set_joint(&mut frame, 10, 0.61, 0.76);
    set_joint(&mut frame, 12, 0.66, 0.55);
    set_joint(&mut frame, 13, 0.68, 0.72);
    frame
}

fn closing_joints() -> Vec<ACJoint> {
    let mut frame = open_joints();
    set_joint(&mut frame, 4, 0.44, 0.55);
    set_joint(&mut frame, 7, 0.54, 0.53);
    frame
}

fn fist_joints() -> Vec<ACJoint> {
    let mut frame = joints(false);
    set_joint(&mut frame, 4, 0.44, 0.55);
    set_joint(&mut frame, 7, 0.54, 0.53);
    frame
}

fn shifted_joints(mut frame: Vec<ACJoint>, dx: f64, dy: f64) -> Vec<ACJoint> {
    for joint in &mut frame {
        joint.x += dx;
        joint.y += dy;
    }
    frame
}

fn set_joint(frame: &mut [ACJoint], kind: u32, x: f64, y: f64) {
    let joint = frame
        .iter_mut()
        .find(|joint| joint.kind == kind)
        .expect("fixture must contain requested joint");
    joint.x = x;
    joint.y = y;
}

fn joint(kind: u32, x: f64, y: f64) -> ACJoint {
    ACJoint {
        kind,
        handedness: 2,
        x,
        y,
        confidence: 1.0,
    }
}
