import XCTest
@testable import AirControl

final class GazeCalibrationResumePolicyTests: XCTestCase {
    func testSuccessfulCalibrationResumesOnlyWhenControlWasActiveAndPermissionsRemainValid() {
        var policy = GazeCalibrationResumePolicy()
        policy.begin(wasControlEnabled: true)

        XCTAssertTrue(policy.finish(.completed, cameraAuthorized: true, accessibilityGranted: true))
        XCTAssertFalse(policy.finish(.completed, cameraAuthorized: true, accessibilityGranted: true))
    }

    func testPreviewCalibrationNeverStartsControl() {
        var policy = GazeCalibrationResumePolicy()
        policy.begin(wasControlEnabled: false)

        XCTAssertFalse(policy.finish(.completed, cameraAuthorized: true, accessibilityGranted: true))
    }

    func testCancellationFailureAndPermissionLossClearResumeIntent() {
        for result in [GazeCalibrationFinish.cancelled, .failed] {
            var policy = GazeCalibrationResumePolicy()
            policy.begin(wasControlEnabled: true)
            XCTAssertFalse(policy.finish(result, cameraAuthorized: true, accessibilityGranted: true))
            XCTAssertFalse(policy.finish(.completed, cameraAuthorized: true, accessibilityGranted: true))
        }

        var policy = GazeCalibrationResumePolicy()
        policy.begin(wasControlEnabled: true)
        XCTAssertFalse(policy.finish(.completed, cameraAuthorized: true, accessibilityGranted: false))
        XCTAssertFalse(policy.finish(.completed, cameraAuthorized: true, accessibilityGranted: true))
    }
}
