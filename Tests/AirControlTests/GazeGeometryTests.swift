import CoreGraphics
import XCTest
@testable import AirControl

final class GazeGeometryTests: XCTestCase {
    func testTwoEyesProduceAnAveragePupilPositionInsideTheirOwnBounds() throws {
        let sample = try XCTUnwrap(GazeGeometry.sample(
            leftEye: [
                CGPoint(x: 0.10, y: 0.40), CGPoint(x: 0.30, y: 0.40),
                CGPoint(x: 0.30, y: 0.60), CGPoint(x: 0.10, y: 0.60)
            ],
            leftPupil: CGPoint(x: 0.22, y: 0.52),
            rightEye: [
                CGPoint(x: 0.60, y: 0.40), CGPoint(x: 0.80, y: 0.40),
                CGPoint(x: 0.80, y: 0.60), CGPoint(x: 0.60, y: 0.60)
            ],
            rightPupil: CGPoint(x: 0.72, y: 0.52),
            faceConfidence: 0.9
        ))

        XCTAssertEqual(sample.x, 0.60, accuracy: 0.000_001)
        XCTAssertEqual(sample.y, 0.60, accuracy: 0.000_001)
        XCTAssertEqual(sample.confidence, 0.9, accuracy: 0.000_001)
        XCTAssertTrue(sample.eyesOpen)
    }

    func testFlatOrMissingEyeGeometryIsRejectedAsABlinkInsteadOfMoving() {
        XCTAssertNil(GazeGeometry.sample(
            leftEye: [CGPoint(x: 0.1, y: 0.5), CGPoint(x: 0.3, y: 0.5)],
            leftPupil: CGPoint(x: 0.2, y: 0.5),
            rightEye: [CGPoint(x: 0.6, y: 0.5), CGPoint(x: 0.8, y: 0.5)],
            rightPupil: CGPoint(x: 0.7, y: 0.5),
            faceConfidence: 0.9
        ))
    }
}
