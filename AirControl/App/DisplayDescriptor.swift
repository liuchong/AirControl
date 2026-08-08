import AppKit
import CoreGraphics

struct DisplayDescriptor: Identifiable, Hashable {
    struct Selection: Equatable {
        let id: CGDirectDisplayID?
        let didFallback: Bool
    }

    let id: CGDirectDisplayID
    let name: String
    let bounds: CGRect

    static var available: [Self] {
        NSScreen.screens.compactMap { screen in
            guard let number = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber else {
                return nil
            }
            let id = CGDirectDisplayID(number.uint32Value)
            return Self(id: id, name: screen.localizedName, bounds: CGDisplayBounds(id))
        }
    }

    static func resolveSelection(
        selectedID: CGDirectDisplayID?,
        available: [Self]
    ) -> Selection {
        if let selectedID, available.contains(where: { $0.id == selectedID }) {
            return Selection(id: selectedID, didFallback: false)
        }
        return Selection(id: available.first?.id, didFallback: selectedID != nil)
    }
}
