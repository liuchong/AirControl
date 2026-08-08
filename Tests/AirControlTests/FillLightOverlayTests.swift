import XCTest
@testable import AirControl

@MainActor
final class FillLightOverlayTests: XCTestCase {
    func testCalibrationRefreshKeepsDisabledFillLightClosed() {
        let controller = FillLightOverlayController()

        controller.refreshForCalibration(settings: .default, displayID: nil)

        XCTAssertFalse(controller.isPresented)
    }
}
