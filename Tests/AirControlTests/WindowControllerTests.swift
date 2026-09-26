import CoreGraphics
import XCTest
@testable import AirControl

final class WindowControllerTests: XCTestCase {
    func testMoveSessionPreservesGrabOffset() {
        let session = WindowMoveSession(
            windowOrigin: CGPoint(x: 120, y: 80),
            pointerAnchor: CGPoint(x: 300, y: 240)
        )

        XCTAssertEqual(
            session.windowOrigin(for: CGPoint(x: 345, y: 215)),
            CGPoint(x: 165, y: 55)
        )
    }

    func testAccessibilityHitPointFlipsQuartzYAroundThePrimaryDisplay() {
        let point = AccessibilityCoordinates.point(
            fromQuartz: CGPoint(x: 120, y: 80),
            primaryHeight: 900
        )
        XCTAssertEqual(point, CGPoint(x: 120, y: 820))
    }

    func testRaisingTheQuartzCursorMovesTheWindowUpInAccessibilitySpace() {
        let height: CGFloat = 1117
        let anchor = AccessibilityCoordinates.point(
            fromQuartz: CGPoint(x: 400, y: 300),
            primaryHeight: height
        )
        let raised = AccessibilityCoordinates.point(
            fromQuartz: CGPoint(x: 400, y: 360),
            primaryHeight: height
        )
        let session = WindowMoveSession(
            windowOrigin: CGPoint(x: 100, y: 400),
            pointerAnchor: anchor
        )

        XCTAssertEqual(
            session.windowOrigin(for: raised),
            CGPoint(x: 100, y: 340)
        )
    }

    func testMoveSessionSupportsNegativeGlobalCoordinates() {
        let session = WindowMoveSession(
            windowOrigin: CGPoint(x: -900, y: 40),
            pointerAnchor: CGPoint(x: -500, y: 180)
        )

        XCTAssertEqual(
            session.windowOrigin(for: CGPoint(x: -540, y: 220)),
            CGPoint(x: -940, y: 80)
        )
    }
}
