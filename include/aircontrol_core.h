#ifndef AIRCONTROL_CORE_H
#define AIRCONTROL_CORE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ACEngine ACEngine;
typedef struct ACCalibrationTracker ACCalibrationTracker;
typedef struct ACGazeCalibrationTracker ACGazeCalibrationTracker;
typedef struct ACGazeMapper ACGazeMapper;

typedef uint32_t ACStatus;
enum {
    AC_STATUS_OK = 0,
    AC_STATUS_INVALID_ARGUMENT = 1,
    AC_STATUS_INVALID_SETTINGS = 2,
    AC_STATUS_BUFFER_TOO_SMALL = 3,
    AC_STATUS_INTERNAL_ERROR = 4
};

typedef uint32_t ACGestureMask;
enum {
    AC_GESTURE_POINTER = 1u << 0,
    AC_GESTURE_PRIMARY_CLICK = 1u << 1,
    AC_GESTURE_DRAG = 1u << 2,
    AC_GESTURE_SECONDARY_CLICK = 1u << 3,
    AC_GESTURE_SCROLL = 1u << 4
};

typedef struct {
    uint32_t kind;
    uint32_t handedness;
    double x;
    double y;
    double confidence;
} ACJoint;

typedef struct {
    uint32_t version;
    uint32_t enabled_gestures;
    double smoothing;
    double minimum_confidence;
    double pinch_enter;
    double pinch_exit;
    double drag_hold_seconds;
    double fist_hold_seconds;
    double hand_loss_seconds;
    double scroll_gain;
    double scroll_dead_zone;
    double calibration_min_x;
    double calibration_max_x;
    double calibration_min_y;
    double calibration_max_y;
    double screen_origin_x;
    double screen_origin_y;
    double screen_width;
    double screen_height;
} ACSettings;

typedef struct {
    uint32_t kind;
    uint32_t button;
    uint32_t flags;
    uint32_t reserved;
    double x;
    double y;
    double value;
} ACCommand;

enum {
    AC_COMMAND_MOVE = 1,
    AC_COMMAND_MOUSE_DOWN = 2,
    AC_COMMAND_MOUSE_UP = 3,
    AC_COMMAND_SCROLL = 4,
    AC_COMMAND_PAUSE_CHANGED = 5,
    AC_COMMAND_CLICK = 6,
    AC_COMMAND_ASSIST_CHANGED = 7,
    AC_COMMAND_WINDOW_GRAB_BEGIN = 8,
    AC_COMMAND_WINDOW_MOVE = 9,
    AC_COMMAND_WINDOW_GRAB_END = 10
};

typedef struct {
    uint32_t kind;
    uint32_t stage;
    double progress;
    double min_x;
    double max_x;
    double min_y;
    double max_y;
} ACCalibrationResult;

typedef struct {
    double x;
    double y;
    double confidence;
    uint32_t eyes_open;
} ACGazeSample;

typedef struct {
    double x_bias;
    double x_from_x;
    double x_from_y;
    double y_bias;
    double y_from_x;
    double y_from_y;
    double raw_min_x;
    double raw_max_x;
    double raw_min_y;
    double raw_max_y;
} ACGazeProfile;

typedef struct {
    uint32_t kind;
    uint32_t stage;
    double progress;
    ACGazeProfile profile;
} ACGazeCalibrationResult;

typedef struct {
    uint32_t present;
    double x;
    double y;
} ACGazePoint;

uint32_t ac_abi_version(void);
ACStatus ac_default_settings(ACSettings *output);
ACStatus ac_engine_create(const ACSettings *settings, ACEngine **output);
ACStatus ac_engine_process(ACEngine *engine, double timestamp,
                           const ACJoint *joints, size_t joint_count,
                           ACCommand *output, size_t output_capacity,
                           size_t *output_count);
ACStatus ac_engine_stop(ACEngine *engine, ACCommand *output,
                        size_t output_capacity, size_t *output_count);
ACStatus ac_engine_rebase_pointer(ACEngine *engine, double x, double y);
ACStatus ac_engine_clear_pointer_rebase(ACEngine *engine);
void ac_engine_destroy(ACEngine *engine);
ACStatus ac_calibration_create(double minimum_confidence,
                               ACCalibrationTracker **output);
ACStatus ac_calibration_update(ACCalibrationTracker *tracker, double timestamp,
                               uint32_t point_present, double x, double y,
                               double confidence,
                               ACCalibrationResult *output);
void ac_calibration_destroy(ACCalibrationTracker *tracker);
ACStatus ac_gaze_profile_fit(const ACGazeSample *samples, size_t sample_count,
                             ACGazeProfile *output);
ACStatus ac_gaze_calibration_create(double minimum_confidence,
                                    ACGazeCalibrationTracker **output);
ACStatus ac_gaze_calibration_update(ACGazeCalibrationTracker *tracker,
                                    double timestamp, uint32_t sample_present,
                                    double x, double y, double confidence,
                                    uint32_t eyes_open,
                                    ACGazeCalibrationResult *output);
void ac_gaze_calibration_destroy(ACGazeCalibrationTracker *tracker);
ACStatus ac_gaze_mapper_create(const ACGazeProfile *profile,
                               double screen_origin_x, double screen_origin_y,
                               double screen_width, double screen_height,
                               double minimum_confidence, double smoothing,
                               ACGazeMapper **output);
ACStatus ac_gaze_mapper_update(ACGazeMapper *mapper, double timestamp,
                               uint32_t sample_present, double x, double y,
                               double confidence, uint32_t eyes_open,
                               ACGazePoint *output);
void ac_gaze_mapper_destroy(ACGazeMapper *mapper);

#ifdef __cplusplus
}
#endif

#endif
