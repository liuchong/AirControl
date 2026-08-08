import XCTest
@testable import AirControl

final class PlatformBehaviorIntegrationTests: XCTestCase {
    func testControlIntentFlowsThroughACompleteGazeCalibrationSession() {
        var policy = GazeCalibrationResumePolicy()
        policy.begin(wasControlEnabled: true)

        XCTAssertTrue(policy.finish(.completed, cameraAuthorized: true, accessibilityGranted: true))
        XCTAssertFalse(policy.finish(.completed, cameraAuthorized: true, accessibilityGranted: true))
    }

    func testFillLightSettingsSurviveStoreReload() throws {
        let suiteName = "AirControlIntegrationTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let store = try SettingsStore(defaults: defaults)
        store.saveFillLight(FillLightSettings(enabled: true, brightness: 0.8, warmth: -0.25))

        XCTAssertEqual(
            try SettingsStore(defaults: defaults).fillLightSettings,
            FillLightSettings(enabled: true, brightness: 0.8, warmth: -0.25)
        )
    }
}
