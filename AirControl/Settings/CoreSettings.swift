import Foundation
import AirControlCoreFFI

enum GestureOption: UInt32, CaseIterable, Identifiable {
    case pointer = 1
    case primaryClick = 2
    case drag = 4
    case secondaryClick = 8
    case scroll = 16

    var id: UInt32 { rawValue }

    var title: String {
        switch self {
        case .pointer: "移动光标"
        case .primaryClick: "左键单击"
        case .drag: "拖拽"
        case .secondaryClick: "右键单击"
        case .scroll: "双指滚动"
        }
    }
}

struct CoreSettings: Codable, Equatable, Sendable {
    var version: UInt32
    var enabledGestures: UInt32
    var smoothing: Double
    var minimumConfidence: Double
    var pinchEnter: Double
    var pinchExit: Double
    var dragHoldSeconds: Double
    var fistHoldSeconds: Double
    var handLossSeconds: Double
    var scrollGain: Double
    var scrollDeadZone: Double
    var calibrationMinX: Double
    var calibrationMaxX: Double
    var calibrationMinY: Double
    var calibrationMaxY: Double
    var screenOriginX: Double
    var screenOriginY: Double
    var screenWidth: Double
    var screenHeight: Double

    static func rustDefaults() throws -> Self {
        var value = ACSettings()
        let status = ac_default_settings(&value)
        guard status == AC_STATUS_OK else { throw CoreEngineError.rustStatus(status) }
        return Self(value)
    }

    var ffiValue: ACSettings {
        var value = ACSettings()
        value.version = version
        value.enabled_gestures = enabledGestures
        value.smoothing = smoothing
        value.minimum_confidence = minimumConfidence
        value.pinch_enter = pinchEnter
        value.pinch_exit = pinchExit
        value.drag_hold_seconds = dragHoldSeconds
        value.fist_hold_seconds = fistHoldSeconds
        value.hand_loss_seconds = handLossSeconds
        value.scroll_gain = scrollGain
        value.scroll_dead_zone = scrollDeadZone
        value.calibration_min_x = calibrationMinX
        value.calibration_max_x = calibrationMaxX
        value.calibration_min_y = calibrationMinY
        value.calibration_max_y = calibrationMaxY
        value.screen_origin_x = screenOriginX
        value.screen_origin_y = screenOriginY
        value.screen_width = screenWidth
        value.screen_height = screenHeight
        return value
    }

    private init(_ value: ACSettings) {
        version = value.version
        enabledGestures = value.enabled_gestures
        smoothing = value.smoothing
        minimumConfidence = value.minimum_confidence
        pinchEnter = value.pinch_enter
        pinchExit = value.pinch_exit
        dragHoldSeconds = value.drag_hold_seconds
        fistHoldSeconds = value.fist_hold_seconds
        handLossSeconds = value.hand_loss_seconds
        scrollGain = value.scroll_gain
        scrollDeadZone = value.scroll_dead_zone
        calibrationMinX = value.calibration_min_x
        calibrationMaxX = value.calibration_max_x
        calibrationMinY = value.calibration_min_y
        calibrationMaxY = value.calibration_max_y
        screenOriginX = value.screen_origin_x
        screenOriginY = value.screen_origin_y
        screenWidth = value.screen_width
        screenHeight = value.screen_height
    }
}
