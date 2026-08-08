import CoreGraphics
import XCTest
@testable import AirControl

final class DisplaySelectionTests: XCTestCase {
    func testKeepsSelectedDisplayWhenItStillExists() {
        let displays = [
            DisplayDescriptor(id: 1, name: "内建显示器", bounds: CGRect(x: 0, y: 0, width: 100, height: 100)),
            DisplayDescriptor(id: 2, name: "外接显示器", bounds: CGRect(x: 100, y: 0, width: 100, height: 100)),
        ]

        let result = DisplayDescriptor.resolveSelection(selectedID: 2, available: displays)

        XCTAssertEqual(result.id, 2)
        XCTAssertFalse(result.didFallback)
    }

    func testFallsBackToPrimaryDisplayWhenSelectedDisplayDisappears() {
        let displays = [
            DisplayDescriptor(id: 1, name: "内建显示器", bounds: CGRect(x: 0, y: 0, width: 100, height: 100)),
        ]

        let result = DisplayDescriptor.resolveSelection(selectedID: 2, available: displays)

        XCTAssertEqual(result.id, 1)
        XCTAssertTrue(result.didFallback)
    }
}
