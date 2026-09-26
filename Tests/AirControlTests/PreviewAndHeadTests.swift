import CoreGraphics
import XCTest
@testable import AirControl

final class PreviewAndHeadTests: XCTestCase {
    func testPreviewKeepsVisionTopAtTheTopOfTheView() {
        let bounds = CGRect(x: 0, y: 0, width: 640, height: 480)
        let withoutLayer = PreviewMapping.viewPoint(
            visionX: 0.25,
            visionY: 0.80,
            bounds: bounds,
            convertedLayerPoint: nil
        )
        XCTAssertEqual(withoutLayer.x, 480, accuracy: 0.001)
        XCTAssertEqual(withoutLayer.y, 384, accuracy: 0.001)

        let pictureTop = PreviewMapping.viewPoint(
            visionX: 0.50,
            visionY: 1,
            bounds: bounds,
            convertedLayerPoint: CGPoint(x: 320, y: 60)
        )
        let pictureBottom = PreviewMapping.viewPoint(
            visionX: 0.50,
            visionY: 0,
            bounds: bounds,
            convertedLayerPoint: CGPoint(x: 320, y: 420)
        )
        XCTAssertGreaterThan(pictureTop.y, pictureBottom.y)
        XCTAssertEqual(pictureTop.y, 420, accuracy: 0.001)
        XCTAssertEqual(pictureBottom.y, 60, accuracy: 0.001)
    }

    func testFaceLandmarkUsesTheSameImageSpaceAsAHandJoint() {
        let box = CGRect(x: 0.2, y: 0.3, width: 0.4, height: 0.5)
        let point = HeadGeometry.normalizedImagePoint(
            landmark: CGPoint(x: 1, y: 1),
            faceBox: box,
            imageWidth: 1000,
            imageHeight: 500
        )
        XCTAssertEqual(point?.x ?? -1, 0.6, accuracy: 0.0001)
        XCTAssertEqual(point?.y ?? -1, 0.8, accuracy: 0.0001)

        let chain = HeadGeometry.chain(
            landmarks: [CGPoint(x: 0, y: 0), CGPoint(x: 1, y: 0), CGPoint(x: 1, y: 1)],
            faceBox: box,
            imageWidth: 1000,
            imageHeight: 500,
            confidence: 0.9,
            closed: true
        )
        XCTAssertEqual(chain?.closed, true)
        XCTAssertEqual(chain?.points.first?.x ?? -1, 0.2, accuracy: 0.0001)
        XCTAssertEqual(chain?.points.first?.y ?? -1, 0.3, accuracy: 0.0001)
        XCTAssertNil(HeadGeometry.chain(
            landmarks: [CGPoint(x: 0, y: 0)],
            faceBox: box,
            imageWidth: 1000,
            imageHeight: 500,
            confidence: 0.2,
            closed: false
        ))
    }
}
