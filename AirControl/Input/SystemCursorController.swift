import ApplicationServices
import CoreGraphics

enum CursorControllerError: Error, LocalizedError {
    case accessibilityDenied
    case eventCreationFailed

    var errorDescription: String? {
        switch self {
        case .accessibilityDenied: "尚未授予辅助功能权限"
        case .eventCreationFailed: "无法创建系统鼠标事件"
        }
    }
}

final class SystemCursorController {
    private var leftIsDown = false
    private var rightIsDown = false
    private let windowController = SystemWindowController()

    func execute(_ commands: [CoreCommand]) throws {
        guard AXIsProcessTrusted() else {
            releaseLocallyTrackedButtons()
            throw CursorControllerError.accessibilityDenied
        }
        for command in commands { try execute(command) }
    }

    func releaseAll() {
        windowController.end()
        guard AXIsProcessTrusted() else {
            leftIsDown = false
            rightIsDown = false
            return
        }
        let position = CGEvent(source: nil)?.location ?? .zero
        if leftIsDown { postMouse(type: .leftMouseUp, position: position, button: .left) }
        if rightIsDown { postMouse(type: .rightMouseUp, position: position, button: .right) }
        leftIsDown = false
        rightIsDown = false
    }

    private func execute(_ command: CoreCommand) throws {
        switch command {
        case .move(let x, let y):
            let type: CGEventType = leftIsDown ? .leftMouseDragged : .mouseMoved
            try postMouseOrThrow(type: type, position: CGPoint(x: x, y: y), button: .left)
        case .click(let button, let x, let y, let count):
            let cgButton: CGMouseButton = button == .left ? .left : .right
            let downType: CGEventType = button == .left ? .leftMouseDown : .rightMouseDown
            let upType: CGEventType = button == .left ? .leftMouseUp : .rightMouseUp
            try postClickOrThrow(
                downType: downType,
                upType: upType,
                position: CGPoint(x: x, y: y),
                button: cgButton,
                count: count
            )
        case .mouseDown(let button, let x, let y):
            let cgButton: CGMouseButton = button == .left ? .left : .right
            let type: CGEventType = button == .left ? .leftMouseDown : .rightMouseDown
            try postMouseOrThrow(type: type, position: CGPoint(x: x, y: y), button: cgButton)
            if button == .left { leftIsDown = true } else { rightIsDown = true }
        case .mouseUp(let button, let x, let y):
            let cgButton: CGMouseButton = button == .left ? .left : .right
            let type: CGEventType = button == .left ? .leftMouseUp : .rightMouseUp
            try postMouseOrThrow(type: type, position: CGPoint(x: x, y: y), button: cgButton)
            if button == .left { leftIsDown = false } else { rightIsDown = false }
        case .scroll(let deltaY):
            guard let event = CGEvent(
                scrollWheelEvent2Source: nil,
                units: .pixel,
                wheelCount: 1,
                wheel1: Int32(deltaY.rounded().clamped(to: -2_000...2_000)),
                wheel2: 0,
                wheel3: 0
            ) else { throw CursorControllerError.eventCreationFailed }
            event.post(tap: .cghidEventTap)
        case .windowGrabBegin(let x, let y):
            _ = try windowController.begin(at: CGPoint(x: x, y: y))
        case .windowMove(let x, let y):
            let point = CGPoint(x: x, y: y)
            if try windowController.move(to: point) {
                try postMouseOrThrow(type: .mouseMoved, position: point, button: .left)
            }
        case .windowGrabEnd:
            windowController.end()
        case .showAppOverview:
            SystemAppSwitcher.show()
        case .pauseChanged, .assistChanged:
            break
        }
    }

    private func postClickOrThrow(
        downType: CGEventType,
        upType: CGEventType,
        position: CGPoint,
        button: CGMouseButton,
        count: UInt32
    ) throws {
        guard let down = CGEvent(mouseEventSource: nil, mouseType: downType, mouseCursorPosition: position, mouseButton: button),
              let up = CGEvent(mouseEventSource: nil, mouseType: upType, mouseCursorPosition: position, mouseButton: button) else {
            throw CursorControllerError.eventCreationFailed
        }
        down.setIntegerValueField(.mouseEventClickState, value: Int64(count))
        up.setIntegerValueField(.mouseEventClickState, value: Int64(count))
        down.post(tap: .cghidEventTap)
        up.post(tap: .cghidEventTap)
    }

    private func postMouseOrThrow(type: CGEventType, position: CGPoint, button: CGMouseButton) throws {
        guard let event = CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: position, mouseButton: button) else {
            throw CursorControllerError.eventCreationFailed
        }
        event.post(tap: .cghidEventTap)
    }

    private func postMouse(type: CGEventType, position: CGPoint, button: CGMouseButton) {
        CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: position, mouseButton: button)?
            .post(tap: .cghidEventTap)
    }

    private func releaseLocallyTrackedButtons() {
        leftIsDown = false
        rightIsDown = false
    }
}

private extension Double {
    func clamped(to range: ClosedRange<Double>) -> Double {
        min(max(self, range.lowerBound), range.upperBound)
    }
}
