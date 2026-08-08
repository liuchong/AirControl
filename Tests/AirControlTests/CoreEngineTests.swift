import XCTest
@testable import AirControl

final class CoreEngineTests: XCTestCase {
    func testDefaultSettingsComeFromRustAndAreValid() throws {
        let settings = try CoreSettings.rustDefaults()
        XCTAssertEqual(settings.version, 1)
        XCTAssertGreaterThan(settings.pinchExit, settings.pinchEnter)
    }

    func testStopIsSafeBeforeAnyFrame() throws {
        let engine = try CoreEngine(settings: .rustDefaults())
        XCTAssertEqual(try engine.stop(), [])
    }
}
