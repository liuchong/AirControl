struct FillLightSettings: Codable, Equatable {
    static let `default` = FillLightSettings(enabled: false, brightness: 0.35, warmth: 0.2)

    let enabled: Bool
    let brightness: Double
    let warmth: Double

    var isValid: Bool {
        brightness.isFinite && (0.1...1.0).contains(brightness)
            && warmth.isFinite && (-1.0...1.0).contains(warmth)
    }
}
