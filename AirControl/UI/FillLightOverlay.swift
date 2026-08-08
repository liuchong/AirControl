import AppKit

@MainActor
final class FillLightOverlayController {
    private var panel: FillLightPanel?
    private var lightView: FillLightView?

    func apply(settings: FillLightSettings, displayID: CGDirectDisplayID?) {
        guard settings.enabled else {
            close()
            return
        }
        guard let screen = Self.screen(for: displayID) else {
            close()
            return
        }

        if panel?.screen != screen {
            close()
            let panel = FillLightPanel(
                contentRect: screen.frame,
                styleMask: [.borderless, .nonactivatingPanel],
                backing: .buffered,
                defer: false,
                screen: screen
            )
            let lightView = FillLightView(frame: CGRect(origin: .zero, size: screen.frame.size))
            panel.contentView = lightView
            panel.level = .screenSaver
            panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary]
            panel.backgroundColor = .clear
            panel.isOpaque = false
            panel.hasShadow = false
            panel.ignoresMouseEvents = true
            panel.hidesOnDeactivate = false
            panel.isReleasedWhenClosed = false
            self.panel = panel
            self.lightView = lightView
        }

        lightView?.settings = settings
        lightView?.needsDisplay = true
        panel?.orderFrontRegardless()
    }

    func bringToFront() {
        panel?.orderFrontRegardless()
    }

    func close() {
        panel?.orderOut(nil)
        panel = nil
        lightView = nil
    }

    private static func screen(for displayID: CGDirectDisplayID?) -> NSScreen? {
        NSScreen.screens.first { screen in
            (screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber)?.uint32Value
                == displayID
        } ?? NSScreen.main ?? NSScreen.screens.first
    }
}

private final class FillLightPanel: NSPanel {
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}

private final class FillLightView: NSView {
    var settings = FillLightSettings.default

    override var isOpaque: Bool { false }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard settings.enabled else { return }

        let cool = NSColor(calibratedRed: 0.80, green: 0.91, blue: 1.0, alpha: 1)
        let neutral = NSColor.white
        let warm = NSColor(calibratedRed: 1.0, green: 0.82, blue: 0.62, alpha: 1)
        let color = settings.warmth < 0
            ? neutral.blended(withFraction: -settings.warmth, of: cool) ?? neutral
            : neutral.blended(withFraction: settings.warmth, of: warm) ?? neutral
        color.withAlphaComponent(settings.brightness).setFill()

        let edge = min(72.0, max(36.0, min(bounds.width, bounds.height) * 0.055))
        let border = NSBezierPath(rect: bounds)
        border.appendRect(bounds.insetBy(dx: edge, dy: edge))
        border.windingRule = .evenOdd
        border.fill()
    }
}
