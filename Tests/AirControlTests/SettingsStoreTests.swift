import XCTest
@testable import AirControl

final class SettingsStoreTests: XCTestCase {
    func testCorruptPersistedSettingsFallBackToRustDefaults() throws {
        let suiteName = "AirControlTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        defaults.set(Data("not-json".utf8), forKey: SettingsStore.storageKey)

        let store = try SettingsStore(defaults: defaults)
        XCTAssertEqual(store.settings.version, 1)
        XCTAssertTrue(store.didRecoverInvalidSettings)
    }

    func testGestureMaskPersistsWithoutChangingRustDefaults() throws {
        let suiteName = "AirControlTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = try SettingsStore(defaults: defaults)
        var settings = store.settings
        settings.enabledGestures &= ~GestureOption.secondaryClick.rawValue

        try store.save(settings)
        let reloaded = try SettingsStore(defaults: defaults)

        XCTAssertEqual(reloaded.settings.enabledGestures, settings.enabledGestures)
        XCTAssertEqual(try CoreSettings.rustDefaults().enabledGestures, 0b1_1111)
    }

    func testPlatformDeviceIdentifiersPersistAndCanBeCleared() throws {
        let suiteName = "AirControlTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = try SettingsStore(defaults: defaults)

        store.saveCameraID("camera-1")
        store.saveDisplayID(42)

        let reloaded = try SettingsStore(defaults: defaults)
        XCTAssertEqual(reloaded.cameraID, "camera-1")
        XCTAssertEqual(reloaded.displayID, 42)

        reloaded.saveCameraID(nil)
        reloaded.saveDisplayID(nil)
        XCTAssertNil(try SettingsStore(defaults: defaults).cameraID)
        XCTAssertNil(try SettingsStore(defaults: defaults).displayID)
    }

    func testGazeAssistDefaultsOffAndPersistsOnlyNumericProfile() throws {
        let suiteName = "AirControlTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = try SettingsStore(defaults: defaults)

        XCTAssertFalse(store.gazeAssistEnabled)
        XCTAssertNil(store.gazeProfile)

        let profile = GazeProfile(
            xBias: -0.5, xFromX: 2.0, xFromY: 0.0,
            yBias: -0.5, yFromX: 0.0, yFromY: 2.0,
            rawMinX: 0.3, rawMaxX: 0.7, rawMinY: 0.3, rawMaxY: 0.7
        )
        store.saveGazeAssist(enabled: true, profile: profile)

        let reloaded = try SettingsStore(defaults: defaults)
        XCTAssertTrue(reloaded.gazeAssistEnabled)
        XCTAssertEqual(reloaded.gazeProfile, profile)
    }

    func testInvalidNumericGazeProfileDisablesAssistOnReload() throws {
        let suiteName = "AirControlTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let invalidProfile = GazeProfile(
            xBias: 0, xFromX: 1, xFromY: 0,
            yBias: 0, yFromX: 0, yFromY: 1,
            rawMinX: 0.5, rawMaxX: 0.5, rawMinY: 0.5, rawMaxY: 0.5
        )
        defaults.set(try JSONEncoder().encode(invalidProfile), forKey: SettingsStore.gazeProfileKey)
        defaults.set(true, forKey: SettingsStore.gazeAssistEnabledKey)

        let store = try SettingsStore(defaults: defaults)

        XCTAssertFalse(store.gazeAssistEnabled)
        XCTAssertNil(store.gazeProfile)
    }

    func testFillLightDefaultsOffAndPersistsValidatedSettings() throws {
        let suiteName = "AirControlTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = try SettingsStore(defaults: defaults)

        XCTAssertEqual(store.fillLightSettings, .default)

        store.saveFillLight(FillLightSettings(enabled: true, brightness: 0.72, warmth: 0.4))
        XCTAssertEqual(
            try SettingsStore(defaults: defaults).fillLightSettings,
            FillLightSettings(enabled: true, brightness: 0.72, warmth: 0.4)
        )
    }

    func testInvalidFillLightSettingsFallBackToSafeDefaults() throws {
        let suiteName = "AirControlTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        defaults.set(
            Data(#"{"enabled":true,"brightness":2.0,"warmth":-4.0}"#.utf8),
            forKey: SettingsStore.fillLightSettingsKey
        )

        XCTAssertEqual(try SettingsStore(defaults: defaults).fillLightSettings, .default)
    }
}
