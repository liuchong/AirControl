struct VisionFailureTracker {
    enum Action: Equatable {
        case continueControl
        case forwardMissingFrame
        case forwardMissingFrameAndSuspend
        case remainSuspended
        case resumeControl
    }

    private let failureLimit: Int
    private let recoverySuccessLimit: Int
    private var consecutiveFailures = 0
    private var consecutiveSuccesses = 0
    private var suspended = false

    init(failureLimit: Int, recoverySuccessLimit: Int) {
        precondition(failureLimit > 0 && recoverySuccessLimit > 0)
        self.failureLimit = failureLimit
        self.recoverySuccessLimit = recoverySuccessLimit
    }

    mutating func recordFailure() -> Action {
        consecutiveSuccesses = 0
        if suspended { return .remainSuspended }
        consecutiveFailures += 1
        guard consecutiveFailures >= failureLimit else { return .forwardMissingFrame }
        suspended = true
        return .forwardMissingFrameAndSuspend
    }

    mutating func recordSuccess() -> Action {
        consecutiveFailures = 0
        guard suspended else { return .continueControl }
        consecutiveSuccesses += 1
        guard consecutiveSuccesses >= recoverySuccessLimit else { return .remainSuspended }
        suspended = false
        consecutiveSuccesses = 0
        return .resumeControl
    }

    mutating func reset() {
        consecutiveFailures = 0
        consecutiveSuccesses = 0
        suspended = false
    }
}
