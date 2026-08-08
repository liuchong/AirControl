import AppKit

@MainActor
final class GazeCalibrationOverlayController {
    private var panel: GazeCalibrationPanel?
    private var targetView: GazeCalibrationTargetView?
    private var eventMonitor: Any?

    func show(displayID: CGDirectDisplayID?, onCancel: @escaping @MainActor () -> Void) {
        close()
        let screen = NSScreen.screens.first { screen in
            (screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber)?.uint32Value == displayID
        } ?? NSScreen.main ?? NSScreen.screens[0]
        let panel = GazeCalibrationPanel(
            contentRect: screen.frame,
            styleMask: [.borderless],
            backing: .buffered,
            defer: false,
            screen: screen
        )
        let targetView = GazeCalibrationTargetView(frame: CGRect(origin: .zero, size: screen.frame.size))
        panel.contentView = targetView
        panel.level = .screenSaver
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.backgroundColor = .clear
        panel.isOpaque = false
        panel.hasShadow = false
        panel.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        self.panel = panel
        self.targetView = targetView
        eventMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            guard event.keyCode == 53 else { return event }
            Task { @MainActor in onCancel() }
            return nil
        }
    }

    func update(stage: Int, progress: Double) {
        targetView?.stage = stage.clamped(to: 0...8)
        targetView?.progress = progress.clamped(to: 0...1)
        targetView?.needsDisplay = true
    }

    func close() {
        if let eventMonitor {
            NSEvent.removeMonitor(eventMonitor)
            self.eventMonitor = nil
        }
        panel?.orderOut(nil)
        panel = nil
        targetView = nil
    }
}

private final class GazeCalibrationPanel: NSPanel {
    override var canBecomeKey: Bool { true }
}

private final class GazeCalibrationTargetView: NSView {
    var stage = 0
    var progress = 0.0

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        NSColor.black.withAlphaComponent(0.82).setFill()
        bounds.fill()

        let columns = [0.12, 0.50, 0.88]
        let rows = [0.88, 0.50, 0.12]
        let center = CGPoint(
            x: bounds.minX + columns[stage % 3] * bounds.width,
            y: bounds.minY + rows[stage / 3] * bounds.height
        )
        let outer = NSRect(x: center.x - 36, y: center.y - 36, width: 72, height: 72)
        NSColor.systemMint.withAlphaComponent(0.22).setFill()
        NSBezierPath(ovalIn: outer).fill()

        let start = 90.0
        let end = start - 360.0 * progress
        let ring = NSBezierPath()
        ring.appendArc(withCenter: center, radius: 28, startAngle: start, endAngle: end, clockwise: true)
        ring.lineWidth = 7
        ring.lineCapStyle = .round
        NSColor.systemMint.setStroke()
        ring.stroke()

        let dot = NSRect(x: center.x - 8, y: center.y - 8, width: 16, height: 16)
        NSColor.white.setFill()
        NSBezierPath(ovalIn: dot).fill()

        let paragraph = NSMutableParagraphStyle()
        paragraph.alignment = .center
        let title = "视线校准 \(stage + 1)/9"
        title.draw(
            in: NSRect(x: 0, y: bounds.maxY - 88, width: bounds.width, height: 38),
            withAttributes: [
                .font: NSFont.systemFont(ofSize: 26, weight: .semibold),
                .foregroundColor: NSColor.white,
                .paragraphStyle: paragraph,
            ]
        )
        "保持头部自然不动，注视圆点；按 Escape 取消".draw(
            in: NSRect(x: 0, y: bounds.maxY - 122, width: bounds.width, height: 28),
            withAttributes: [
                .font: NSFont.systemFont(ofSize: 16),
                .foregroundColor: NSColor.secondaryLabelColor,
                .paragraphStyle: paragraph,
            ]
        )
    }
}

private extension Comparable {
    func clamped(to range: ClosedRange<Self>) -> Self {
        Swift.min(Swift.max(self, range.lowerBound), range.upperBound)
    }
}
