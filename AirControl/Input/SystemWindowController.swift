import ApplicationServices
import CoreGraphics

struct WindowMoveSession: Equatable {
    let windowOrigin: CGPoint
    let pointerAnchor: CGPoint

    func windowOrigin(for pointer: CGPoint) -> CGPoint {
        CGPoint(
            x: windowOrigin.x + pointer.x - pointerAnchor.x,
            y: windowOrigin.y + pointer.y - pointerAnchor.y
        )
    }
}

final class SystemWindowController {
    private var window: AXUIElement?
    private var session: WindowMoveSession?

    func begin(at pointer: CGPoint) throws -> Bool {
        end()
        guard AXIsProcessTrusted() else {
            throw CursorControllerError.accessibilityDenied
        }

        let systemWide = AXUIElementCreateSystemWide()
        var hitElement: AXUIElement?
        guard AXUIElementCopyElementAtPosition(
            systemWide,
            Float(pointer.x),
            Float(pointer.y),
            &hitElement
        ) == .success, let hitElement else {
            return false
        }
        guard let targetWindow = window(containing: hitElement),
              positionIsSettable(for: targetWindow),
              let origin = position(of: targetWindow) else {
            return false
        }

        window = targetWindow
        session = WindowMoveSession(windowOrigin: origin, pointerAnchor: pointer)
        _ = AXUIElementPerformAction(targetWindow, kAXRaiseAction as CFString)
        return true
    }

    func move(to pointer: CGPoint) throws -> Bool {
        guard AXIsProcessTrusted() else {
            end()
            throw CursorControllerError.accessibilityDenied
        }
        guard let window, let session else { return false }

        var origin = session.windowOrigin(for: pointer)
        guard let value = AXValueCreate(.cgPoint, &origin) else {
            end()
            return false
        }
        let error = AXUIElementSetAttributeValue(
            window,
            kAXPositionAttribute as CFString,
            value
        )
        guard error == .success else {
            end()
            if error == .apiDisabled {
                throw CursorControllerError.accessibilityDenied
            }
            return false
        }
        return true
    }

    func end() {
        window = nil
        session = nil
    }

    private func window(containing element: AXUIElement) -> AXUIElement? {
        var value: CFTypeRef?
        if AXUIElementCopyAttributeValue(
            element,
            kAXWindowAttribute as CFString,
            &value
        ) == .success, let value {
            return unsafeDowncast(value, to: AXUIElement.self)
        }

        var roleValue: CFTypeRef?
        guard AXUIElementCopyAttributeValue(
            element,
            kAXRoleAttribute as CFString,
            &roleValue
        ) == .success,
        let role = roleValue as? String,
        role == kAXWindowRole as String else {
            return nil
        }
        return element
    }

    private func positionIsSettable(for window: AXUIElement) -> Bool {
        var settable = DarwinBoolean(false)
        return AXUIElementIsAttributeSettable(
            window,
            kAXPositionAttribute as CFString,
            &settable
        ) == .success && settable.boolValue
    }

    private func position(of window: AXUIElement) -> CGPoint? {
        var value: CFTypeRef?
        guard AXUIElementCopyAttributeValue(
            window,
            kAXPositionAttribute as CFString,
            &value
        ) == .success,
        let value,
        CFGetTypeID(value) == AXValueGetTypeID() else {
            return nil
        }
        let axValue = unsafeDowncast(value, to: AXValue.self)
        guard AXValueGetType(axValue) == .cgPoint else { return nil }
        var point = CGPoint.zero
        return AXValueGetValue(axValue, .cgPoint, &point) ? point : nil
    }
}
