enum GazeCalibrationFinish {
    case completed
    case cancelled
    case failed
}

struct GazeCalibrationResumePolicy {
    private var shouldResume = false

    mutating func begin(wasControlEnabled: Bool) {
        shouldResume = wasControlEnabled
    }

    mutating func finish(
        _ result: GazeCalibrationFinish,
        cameraAuthorized: Bool,
        accessibilityGranted: Bool
    ) -> Bool {
        defer { shouldResume = false }
        return result == .completed
            && shouldResume
            && cameraAuthorized
            && accessibilityGranted
    }

    mutating func reset() {
        shouldResume = false
    }
}
