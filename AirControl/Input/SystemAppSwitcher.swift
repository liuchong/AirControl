import AppKit

enum SystemAppSwitcher {
    static func show() {
        let missionControl = URL(fileURLWithPath: "/System/Applications/Mission Control.app")
        if NSWorkspace.shared.open(missionControl) {
            return
        }
        revealDock()
    }

    private static func revealDock() {
        let mouse = NSEvent.mouseLocation
        let screen = NSScreen.screens.first { $0.frame.contains(mouse) } ?? NSScreen.main
        guard let screen else { return }
        let full = screen.frame
        let visible = screen.visibleFrame
        let point: CGPoint
        if visible.minY > full.minY + 1 {
            point = CGPoint(x: full.midX, y: full.minY + 1)
        } else if visible.minX > full.minX + 1 {
            point = CGPoint(x: full.minX + 1, y: full.midY)
        } else if visible.maxX < full.maxX - 1 {
            point = CGPoint(x: full.maxX - 1, y: full.midY)
        } else {
            point = CGPoint(x: full.midX, y: full.minY + 1)
        }
        CGEvent(
            mouseEventSource: nil,
            mouseType: .mouseMoved,
            mouseCursorPosition: point,
            mouseButton: .left
        )?.post(tap: .cghidEventTap)
    }
}
