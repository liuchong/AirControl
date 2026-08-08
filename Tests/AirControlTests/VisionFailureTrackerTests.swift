import XCTest
@testable import AirControl

final class VisionFailureTrackerTests: XCTestCase {
    func testEveryFailureForwardsMissingFrameAndFifthSuspendsWithoutStoppingCamera() {
        var tracker = VisionFailureTracker(failureLimit: 5, recoverySuccessLimit: 2)
        for _ in 0..<4 {
            XCTAssertEqual(tracker.recordFailure(), .forwardMissingFrame)
        }
        XCTAssertEqual(tracker.recordFailure(), .forwardMissingFrameAndSuspend)
        XCTAssertEqual(tracker.recordFailure(), .remainSuspended)
    }

    func testTwoConsecutiveSuccessfulFramesResumeAfterSuspension() {
        var tracker = VisionFailureTracker(failureLimit: 5, recoverySuccessLimit: 2)
        for _ in 0..<5 { _ = tracker.recordFailure() }
        XCTAssertEqual(tracker.recordSuccess(), .remainSuspended)
        XCTAssertEqual(tracker.recordSuccess(), .resumeControl)
    }

    func testFailureInterruptsRecoverySuccessCount() {
        var tracker = VisionFailureTracker(failureLimit: 5, recoverySuccessLimit: 2)
        for _ in 0..<5 { _ = tracker.recordFailure() }
        XCTAssertEqual(tracker.recordSuccess(), .remainSuspended)
        XCTAssertEqual(tracker.recordFailure(), .remainSuspended)
        XCTAssertEqual(tracker.recordSuccess(), .remainSuspended)
        XCTAssertEqual(tracker.recordSuccess(), .resumeControl)
    }

    func testResetStartsAFreshSession() {
        var tracker = VisionFailureTracker(failureLimit: 5, recoverySuccessLimit: 2)
        for _ in 0..<5 { _ = tracker.recordFailure() }
        tracker.reset()
        XCTAssertEqual(tracker.recordSuccess(), .continueControl)
        XCTAssertEqual(tracker.recordFailure(), .forwardMissingFrame)
    }

    func testSuccessfulFrameResetsConsecutiveFailureCountBeforeSuspension() {
        var tracker = VisionFailureTracker(failureLimit: 5, recoverySuccessLimit: 2)
        for _ in 0..<4 { _ = tracker.recordFailure() }
        XCTAssertEqual(tracker.recordSuccess(), .continueControl)
        XCTAssertEqual(tracker.recordFailure(), .forwardMissingFrame)
    }
}
