import CoreGraphics
import Foundation

final class SettingsStore {
    static let storageKey = "aircontrol.settings.v1"
    static let cameraIDKey = "aircontrol.camera-id.v1"
    static let displayIDKey = "aircontrol.display-id.v1"
    static let gazeAssistEnabledKey = "aircontrol.gaze-assist-enabled.v1"
    static let gazeProfileKey = "aircontrol.gaze-profile.v1"
    static let fillLightSettingsKey = "aircontrol.fill-light-settings.v1"
    static let showsCameraImageKey = "aircontrol.shows-camera-image.v1"

    private let defaults: UserDefaults
    private(set) var settings: CoreSettings
    private(set) var didRecoverInvalidSettings = false
    private(set) var gazeAssistEnabled: Bool
    private(set) var gazeProfile: GazeProfile?
    private(set) var fillLightSettings: FillLightSettings
    private(set) var showsCameraImage: Bool

    init(defaults: UserDefaults = .standard) throws {
        self.defaults = defaults
        let decodedGazeProfile = defaults.data(forKey: Self.gazeProfileKey).flatMap {
            try? JSONDecoder().decode(GazeProfile.self, from: $0)
        }
        gazeProfile = decodedGazeProfile.flatMap { Self.isValidGazeProfile($0) ? $0 : nil }
        gazeAssistEnabled = defaults.bool(forKey: Self.gazeAssistEnabledKey) && gazeProfile != nil
        let decodedFillLight = defaults.data(forKey: Self.fillLightSettingsKey).flatMap {
            try? JSONDecoder().decode(FillLightSettings.self, from: $0)
        }
        fillLightSettings = decodedFillLight.flatMap { $0.isValid ? $0 : nil } ?? .default
        showsCameraImage = defaults.object(forKey: Self.showsCameraImageKey) as? Bool ?? false
        let fallback = try CoreSettings.rustDefaults()
        guard let data = defaults.data(forKey: Self.storageKey) else {
            settings = fallback
            return
        }
        do {
            let decoded = try JSONDecoder().decode(CoreSettings.self, from: data)
            _ = try CoreEngine(settings: decoded)
            settings = decoded
        } catch {
            settings = fallback
            didRecoverInvalidSettings = true
        }
    }

    func save(_ settings: CoreSettings) throws {
        _ = try CoreEngine(settings: settings)
        let data = try JSONEncoder().encode(settings)
        defaults.set(data, forKey: Self.storageKey)
        self.settings = settings
        didRecoverInvalidSettings = false
    }

    var cameraID: String? {
        defaults.string(forKey: Self.cameraIDKey)
    }

    var displayID: UInt32? {
        guard let number = defaults.object(forKey: Self.displayIDKey) as? NSNumber,
              number.uint64Value <= UInt32.max else { return nil }
        return number.uint32Value
    }

    func saveCameraID(_ id: String?) {
        if let id {
            defaults.set(id, forKey: Self.cameraIDKey)
        } else {
            defaults.removeObject(forKey: Self.cameraIDKey)
        }
    }

    func saveDisplayID(_ id: UInt32?) {
        if let id {
            defaults.set(NSNumber(value: id), forKey: Self.displayIDKey)
        } else {
            defaults.removeObject(forKey: Self.displayIDKey)
        }
    }

    func saveGazeAssist(enabled: Bool, profile: GazeProfile?) {
        let validProfile = profile.flatMap { Self.isValidGazeProfile($0) ? $0 : nil }
        gazeProfile = validProfile
        gazeAssistEnabled = enabled && validProfile != nil
        defaults.set(gazeAssistEnabled, forKey: Self.gazeAssistEnabledKey)
        if let validProfile, let data = try? JSONEncoder().encode(validProfile) {
            defaults.set(data, forKey: Self.gazeProfileKey)
        } else {
            defaults.removeObject(forKey: Self.gazeProfileKey)
        }
    }

    func saveShowsCameraImage(_ shows: Bool) {
        showsCameraImage = shows
        defaults.set(shows, forKey: Self.showsCameraImageKey)
    }

    func saveFillLight(_ settings: FillLightSettings) {
        let validated = settings.isValid ? settings : .default
        fillLightSettings = validated
        if let data = try? JSONEncoder().encode(validated) {
            defaults.set(data, forKey: Self.fillLightSettingsKey)
        }
    }

    private static func isValidGazeProfile(_ profile: GazeProfile) -> Bool {
        (try? GazeMapper(
            profile: profile,
            screen: CGRect(x: 0, y: 0, width: 1, height: 1),
            minimumConfidence: 0.55,
            smoothing: 0.35
        )) != nil
    }
}
