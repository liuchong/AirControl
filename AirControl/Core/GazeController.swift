import CoreGraphics
import Foundation
import AirControlCoreFFI

struct StandardGazeSample: Equatable, Sendable {
    let x: Double
    let y: Double
    let confidence: Double
    let eyesOpen: Bool

    var ffiValue: ACGazeSample {
        var value = ACGazeSample()
        value.x = x
        value.y = y
        value.confidence = confidence
        value.eyes_open = eyesOpen ? 1 : 0
        return value
    }
}

struct GazeProfile: Codable, Equatable, Sendable {
    let xBias: Double
    let xFromX: Double
    let xFromY: Double
    let yBias: Double
    let yFromX: Double
    let yFromY: Double
    let rawMinX: Double
    let rawMaxX: Double
    let rawMinY: Double
    let rawMaxY: Double

    init(
        xBias: Double,
        xFromX: Double,
        xFromY: Double,
        yBias: Double,
        yFromX: Double,
        yFromY: Double,
        rawMinX: Double,
        rawMaxX: Double,
        rawMinY: Double,
        rawMaxY: Double
    ) {
        self.xBias = xBias
        self.xFromX = xFromX
        self.xFromY = xFromY
        self.yBias = yBias
        self.yFromX = yFromX
        self.yFromY = yFromY
        self.rawMinX = rawMinX
        self.rawMaxX = rawMaxX
        self.rawMinY = rawMinY
        self.rawMaxY = rawMaxY
    }

    init(_ value: ACGazeProfile) {
        self.init(
            xBias: value.x_bias,
            xFromX: value.x_from_x,
            xFromY: value.x_from_y,
            yBias: value.y_bias,
            yFromX: value.y_from_x,
            yFromY: value.y_from_y,
            rawMinX: value.raw_min_x,
            rawMaxX: value.raw_max_x,
            rawMinY: value.raw_min_y,
            rawMaxY: value.raw_max_y
        )
    }

    var ffiValue: ACGazeProfile {
        var value = ACGazeProfile()
        value.x_bias = xBias
        value.x_from_x = xFromX
        value.x_from_y = xFromY
        value.y_bias = yBias
        value.y_from_x = yFromX
        value.y_from_y = yFromY
        value.raw_min_x = rawMinX
        value.raw_max_x = rawMaxX
        value.raw_min_y = rawMinY
        value.raw_max_y = rawMaxY
        return value
    }

    static func fitForTesting(_ samples: [StandardGazeSample]) throws -> Self {
        var ffiSamples = samples.map(\.ffiValue)
        var output = ACGazeProfile()
        let status = ffiSamples.withUnsafeMutableBufferPointer { buffer in
            ac_gaze_profile_fit(buffer.baseAddress, buffer.count, &output)
        }
        guard status == AC_STATUS_OK else { throw CoreEngineError.rustStatus(status) }
        return Self(output)
    }
}

enum GazeCalibrationEvent: Equatable {
    case progress(stage: Int, value: Double)
    case targetCaptured(Int)
    case completed(GazeProfile)
    case invalidProfile
}

final class GazeCalibrationController {
    private var handle: OpaquePointer?

    init(minimumConfidence: Double) throws {
        var newHandle: OpaquePointer?
        let status = ac_gaze_calibration_create(minimumConfidence, &newHandle)
        guard status == AC_STATUS_OK, let newHandle else {
            throw CoreEngineError.rustStatus(status)
        }
        handle = newHandle
    }

    deinit { ac_gaze_calibration_destroy(handle) }

    func update(timestamp: Double, sample: StandardGazeSample?) throws -> GazeCalibrationEvent {
        var result = ACGazeCalibrationResult()
        let status = ac_gaze_calibration_update(
            handle,
            timestamp,
            sample == nil ? 0 : 1,
            sample?.x ?? 0,
            sample?.y ?? 0,
            sample?.confidence ?? 0,
            sample?.eyesOpen == true ? 1 : 0,
            &result
        )
        guard status == AC_STATUS_OK else { throw CoreEngineError.rustStatus(status) }
        switch result.kind {
        case 1: return .progress(stage: Int(result.stage), value: result.progress)
        case 2: return .targetCaptured(Int(result.stage))
        case 3: return .completed(GazeProfile(result.profile))
        case 4: return .invalidProfile
        default: throw CoreEngineError.invalidCommand(result.kind)
        }
    }
}

final class GazeMapper {
    private var handle: OpaquePointer?

    init(
        profile: GazeProfile,
        screen: CGRect,
        minimumConfidence: Double,
        smoothing: Double
    ) throws {
        var value = profile.ffiValue
        var newHandle: OpaquePointer?
        let status = ac_gaze_mapper_create(
            &value,
            screen.origin.x,
            screen.origin.y,
            screen.width,
            screen.height,
            minimumConfidence,
            smoothing,
            &newHandle
        )
        guard status == AC_STATUS_OK, let newHandle else {
            throw CoreEngineError.rustStatus(status)
        }
        handle = newHandle
    }

    deinit { ac_gaze_mapper_destroy(handle) }

    func update(timestamp: Double, sample: StandardGazeSample?) throws -> CGPoint? {
        var result = ACGazePoint()
        let status = ac_gaze_mapper_update(
            handle,
            timestamp,
            sample == nil ? 0 : 1,
            sample?.x ?? 0,
            sample?.y ?? 0,
            sample?.confidence ?? 0,
            sample?.eyesOpen == true ? 1 : 0,
            &result
        )
        guard status == AC_STATUS_OK else { throw CoreEngineError.rustStatus(status) }
        return result.present == 1 ? CGPoint(x: result.x, y: result.y) : nil
    }
}
