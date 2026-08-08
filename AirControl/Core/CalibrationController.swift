import AirControlCoreFFI

enum CalibrationEvent: Equatable {
    case progress(stage: UInt32, value: Double)
    case cornerCaptured(UInt32)
    case completed(minX: Double, maxX: Double, minY: Double, maxY: Double)
    case invalidRange
}

final class CalibrationController {
    private var handle: OpaquePointer?

    init(minimumConfidence: Double) throws {
        var newHandle: OpaquePointer?
        let status = ac_calibration_create(minimumConfidence, &newHandle)
        guard status == AC_STATUS_OK, let newHandle else {
            throw CoreEngineError.rustStatus(status)
        }
        handle = newHandle
    }

    deinit { ac_calibration_destroy(handle) }

    func update(timestamp: Double, point: StandardJoint?) throws -> CalibrationEvent {
        var result = ACCalibrationResult()
        let status = ac_calibration_update(
            handle,
            timestamp,
            point == nil ? 0 : 1,
            point?.x ?? 0,
            point?.y ?? 0,
            point?.confidence ?? 0,
            &result
        )
        guard status == AC_STATUS_OK else { throw CoreEngineError.rustStatus(status) }
        switch result.kind {
        case 1: return .progress(stage: result.stage, value: result.progress)
        case 2: return .cornerCaptured(result.stage)
        case 3: return .completed(minX: result.min_x, maxX: result.max_x, minY: result.min_y, maxY: result.max_y)
        case 4: return .invalidRange
        default: throw CoreEngineError.invalidCommand(result.kind)
        }
    }
}
