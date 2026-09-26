import AppKit
import ApplicationServices
import CoreGraphics

enum AccessibilityCoordinates {
    static func point(fromQuartz quartz: CGPoint, primaryHeight: CGFloat) -> CGPoint {
        CGPoint(x: quartz.x, y: primaryHeight - quartz.y)
    }
}

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

        let axPointer = AccessibilityCoordinates.point(
            fromQuartz: pointer,
            primaryHeight: Self.primaryHeight
        )
        guard let targetWindow = movableWindow(at: axPointer),
              let origin = position(of: targetWindow) else {
            return false
        }

        window = targetWindow
        session = WindowMoveSession(windowOrigin: origin, pointerAnchor: axPointer)
        _ = AXUIElementPerformAction(targetWindow, kAXRaiseAction as CFString)
        return true
    }

    func move(to pointer: CGPoint) throws -> Bool {
        guard AXIsProcessTrusted() else {
            end()
            throw CursorControllerError.accessibilityDenied
        }
        guard let window, let session else { return false }

        let axPointer = AccessibilityCoordinates.point(
            fromQuartz: pointer,
            primaryHeight: Self.primaryHeight
        )
        var origin = session.windowOrigin(for: axPointer)
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

    private static var primaryHeight: CGFloat {
        NSScreen.screens.first {
            abs($0.frame.origin.x) < 0.5 && abs($0.frame.origin.y) < 0.5
        }?.frame.height ?? NSScreen.main?.frame.height ?? 0
    }

    private func movableWindow(at axPoint: CGPoint) -> AXUIElement? {
        let systemWide = AXUIElementCreateSystemWide()
        var hitElement: AXUIElement?
        if AXUIElementCopyElementAtPosition(
            systemWide,
            Float(axPoint.x),
            Float(axPoint.y),
            &hitElement
        ) == .success, let hitElement, let found = settableWindow(from: hitElement) {
            return found
        }
        return windowFromOnScreenList(at: axPoint)
    }

    private func settableWindow(from element: AXUIElement) -> AXUIElement? {
        var current: AXUIElement? = element
        for _ in 0..<16 {
            guard let element = current else { return nil }
            if let window = referencedWindow(of: element), positionIsSettable(for: window) {
                return window
            }
            if isWindow(element), positionIsSettable(for: element) {
                return element
            }
            var parent: CFTypeRef?
            guard AXUIElementCopyAttributeValue(
                element,
                kAXParentAttribute as CFString,
                &parent
            ) == .success, let parent else {
                return nil
            }
            current = unsafeDowncast(parent, to: AXUIElement.self)
        }
        return nil
    }

    private func windowFromOnScreenList(at axPoint: CGPoint) -> AXUIElement? {
        let options = CGWindowListOption([.optionOnScreenOnly, .excludeDesktopElements])
        guard let list = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
            return nil
        }
        for entry in list {
            let layer = (entry[kCGWindowLayer as String] as? NSNumber)?.intValue ?? 0
            let pidValue = pid_t((entry[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value ?? -1)
            guard layer == 0, pidValue > 0,
                  let bounds = Self.rect(entry[kCGWindowBounds as String]),
                  bounds.contains(axPoint) else {
                continue
            }
            if let window = axWindow(pid: pidValue, matching: bounds) {
                return window
            }
        }
        return nil
    }

    private func axWindow(pid: pid_t, matching bounds: CGRect) -> AXUIElement? {
        let application = AXUIElementCreateApplication(pid)
        var value: CFTypeRef?
        guard AXUIElementCopyAttributeValue(
            application,
            kAXWindowsAttribute as CFString,
            &value
        ) == .success, let windows = value as? [AXUIElement] else {
            return nil
        }
        return windows.first { window in
            guard positionIsSettable(for: window),
                  let origin = position(of: window),
                  let size = size(of: window) else {
                return false
            }
            return abs(origin.x - bounds.origin.x) < 24
                && abs(origin.y - bounds.origin.y) < 24
                && abs(size.width - bounds.width) < 24
                && abs(size.height - bounds.height) < 24
        }
    }

    private func referencedWindow(of element: AXUIElement) -> AXUIElement? {
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

    private func isWindow(_ element: AXUIElement) -> Bool {
        var roleValue: CFTypeRef?
        guard AXUIElementCopyAttributeValue(
            element,
            kAXRoleAttribute as CFString,
            &roleValue
        ) == .success, let role = roleValue as? String else {
            return false
        }
        return role == kAXWindowRole as String
    }

    private func pid(of element: AXUIElement) -> pid_t {
        var pid: pid_t = 0
        AXUIElementGetPid(element, &pid)
        return pid
    }

    private static func rect(_ value: Any?) -> CGRect? {
        guard let dictionary = value as? [String: Any],
              let x = (dictionary["X"] as? NSNumber)?.doubleValue,
              let y = (dictionary["Y"] as? NSNumber)?.doubleValue,
              let width = (dictionary["Width"] as? NSNumber)?.doubleValue,
              let height = (dictionary["Height"] as? NSNumber)?.doubleValue else {
            return nil
        }
        return CGRect(x: x, y: y, width: width, height: height)
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

    private func size(of window: AXUIElement) -> CGSize? {
        var value: CFTypeRef?
        guard AXUIElementCopyAttributeValue(
            window,
            kAXSizeAttribute as CFString,
            &value
        ) == .success,
        let value,
        CFGetTypeID(value) == AXValueGetTypeID() else {
            return nil
        }
        let axValue = unsafeDowncast(value, to: AXValue.self)
        guard AXValueGetType(axValue) == .cgSize else { return nil }
        var size = CGSize.zero
        return AXValueGetValue(axValue, .cgSize, &size) ? size : nil
    }
}
