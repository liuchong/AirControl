import XCTest
@testable import AirControl

final class GestureFixtureTests: XCTestCase {
    func testShortClickFixtureCrossesTheRealRustFFIBoundary() throws {
        try assertFixture("short_click")
    }

    func testRightClickFixtureCrossesTheRealRustFFIBoundary() throws {
        try assertFixture("right_click")
    }

    func testHandLossDuringDragReleasesThroughTheRealRustFFIBoundary() throws {
        try assertFixture("drag_hand_loss")
    }

    func testPartialLandmarkPointerFixtureCrossesTheRealRustFFIBoundary() throws {
        try assertFixture("pointer_partial_landmarks")
    }

    func testPinchAnchorFixtureCrossesTheRealRustFFIBoundary() throws {
        try assertFixture("pinch_anchor")
    }

    func testPinchDropoutFixtureCrossesTheRealRustFFIBoundary() throws {
        try assertFixture("pinch_dropout")
    }

    func testBimanualPrecisionClickCrossesTheRealRustFFIBoundary() throws {
        let commands = try commandsForFixture("bimanual_precision_click")
        XCTAssertTrue(commands.contains { command in
            if case .click(.left, _, _, 1) = command { return true }
            return false
        })
    }

    func testBimanualDoubleClickCrossesTheRealRustFFIBoundary() throws {
        let commands = try commandsForFixture("bimanual_double_click")
        XCTAssertTrue(commands.contains { command in
            if case .click(.left, _, _, 2) = command { return true }
            return false
        })
    }

    func testBimanualDragLossCrossesTheRealRustFFIBoundary() throws {
        try assertFixture("bimanual_drag_loss")
    }

    func testWindowGrabProcessCrossesTheRealRustFFIBoundary() throws {
        let commands = try commandsForFixture("window_grab")
        XCTAssertEqual(
            commands.compactMap { command -> UInt32? in
                switch command {
                case .windowGrabBegin: 8
                case .windowMove: 9
                case .windowGrabEnd: 10
                default: nil
                }
            },
            [8, 9, 10]
        )
    }

    func testDisabledPrimaryClickCrossesTheRealRustFFIBoundary() throws {
        var settings = try CoreSettings.rustDefaults()
        settings.enabledGestures &= ~(1 << 1)
        let engine = try CoreEngine(settings: settings)
        var commands: [CoreCommand] = []
        commands += try engine.process(timestamp: 0.00, joints: TestHand.joints(primaryPinch: true))
        commands += try engine.process(timestamp: 0.05, joints: TestHand.joints(primaryPinch: true))
        commands += try engine.process(timestamp: 0.20, joints: TestHand.joints())
        XCTAssertTrue(commands.allSatisfy { $0.kindCode != 2 && $0.kindCode != 3 && $0.kindCode != 6 })
    }

    func testHorizontalPointerCrossesTheRealRustFFIBoundary() throws {
        let engine = try CoreEngine(settings: .rustDefaults())
        let commands = try engine.process(timestamp: 0.0, joints: TestHand.horizontalPointer(indexX: 0.82))
        XCTAssertTrue(commands.contains { $0.kindCode == 1 })
        XCTAssertFalse(commands.contains { $0.kindCode == 5 })
    }

    func testUnrelatedOutOfBoundsJointDoesNotDropPointerAcrossTheRealRustFFIBoundary() throws {
        let engine = try CoreEngine(settings: .rustDefaults())
        let commands = try engine.process(timestamp: 0.0, joints: TestHand.jointsWithOutOfBoundsRingTip())
        XCTAssertTrue(commands.contains { $0.kindCode == 1 })
    }

    func testPrimaryPinchFreezesAndClicksAtAnchorAcrossTheRealRustFFIBoundary() throws {
        let engine = try CoreEngine(settings: .rustDefaults())
        let initial = try engine.process(timestamp: 0.00, joints: TestHand.pointer(indexX: 0.44))
        let anchor = try XCTUnwrap(initial.compactMap { command -> (Double, Double)? in
            guard case .move(let x, let y) = command else { return nil }
            return (x, y)
        }.first)

        let first = try engine.process(timestamp: 0.05, joints: TestHand.pinched(indexX: 0.54, indexY: 0.72))
        let second = try engine.process(timestamp: 0.10, joints: TestHand.pinched(indexX: 0.56, indexY: 0.70))
        XCTAssertFalse((first + second).contains { $0.kindCode == 1 })

        let firstOpen = try engine.process(timestamp: 0.20, joints: TestHand.joints())
        XCTAssertFalse(firstOpen.contains { $0.kindCode == 2 || $0.kindCode == 3 || $0.kindCode == 6 })
        let click = try engine.process(timestamp: 0.24, joints: TestHand.joints())
        let clickPoints = click.compactMap { command -> (Double, Double)? in
            switch command {
            case .click(.left, let x, let y, 1):
                return (x, y)
            default:
                return nil
            }
        }
        XCTAssertEqual(clickPoints.count, 1)
        XCTAssertTrue(clickPoints.allSatisfy {
            abs($0.0 - anchor.0) < 0.000_001 && abs($0.1 - anchor.1) < 0.000_001
        })
    }

    func testBriefMissingThumbKeepsPinchAcrossTheRealRustFFIBoundary() throws {
        let engine = try CoreEngine(settings: .rustDefaults())
        _ = try engine.process(timestamp: 0.00, joints: TestHand.pointer(indexX: 0.44))
        _ = try engine.process(timestamp: 0.05, joints: TestHand.pinched(indexX: 0.50, indexY: 0.75))
        _ = try engine.process(
            timestamp: 0.09,
            joints: TestHand.pinched(indexX: 0.51, indexY: 0.74, thumbConfidence: 0)
        )
        _ = try engine.process(timestamp: 0.12, joints: TestHand.pinched(indexX: 0.52, indexY: 0.73))

        let firstOpen = try engine.process(timestamp: 0.20, joints: TestHand.joints())
        XCTAssertFalse(firstOpen.contains { $0.kindCode == 2 || $0.kindCode == 3 || $0.kindCode == 6 })
        let secondOpen = try engine.process(timestamp: 0.24, joints: TestHand.joints())
        XCTAssertEqual(secondOpen.filter { $0.kindCode == 6 }.count, 1)
    }

    func testGazeMappingAndRelativeHandOffCrossTheRealRustFFIBoundary() throws {
        let samples = (0..<9).map { stage in
            StandardGazeSample(
                x: [0.32, 0.50, 0.68][stage % 3],
                y: [0.34, 0.50, 0.66][stage / 3],
                confidence: 0.95,
                eyesOpen: true
            )
        }
        let profile = try GazeProfile.fitForTesting(samples)
        let mapper = try GazeMapper(
            profile: profile,
            screen: CGRect(x: 0, y: 0, width: 1440, height: 900),
            minimumConfidence: 0.55,
            smoothing: 0.35
        )
        let point = try XCTUnwrap(mapper.update(timestamp: 0, sample: samples[4]))
        XCTAssertEqual(point.x, 720, accuracy: 2)
        XCTAssertEqual(point.y, 450, accuracy: 2)

        let engine = try CoreEngine(settings: .rustDefaults())
        try engine.rebasePointer(to: point)
        let first = try engine.process(timestamp: 0.10, joints: TestHand.pointer(indexX: 0.30))
        XCTAssertTrue(first.contains { command in
            if case .move(let x, let y) = command {
                return abs(x - point.x) < 0.001 && abs(y - point.y) < 0.001
            }
            return false
        })
    }

    private func assertFixture(_ name: String) throws {
        let (fixture, commands) = try loadFixture(name)
        let kinds = commands.map(\.kindCode)
        if let expectedAllCommandKinds = fixture.expectedAllCommandKinds {
            XCTAssertEqual(kinds, expectedAllCommandKinds)
        } else {
            XCTAssertEqual(kinds.filter { $0 == 2 || $0 == 3 || $0 == 6 }, fixture.expectedCommandKinds)
        }
    }

    private func commandsForFixture(_ name: String) throws -> [CoreCommand] {
        let (fixture, commands) = try loadFixture(name)
        XCTAssertEqual(commands.map(\.kindCode), fixture.expectedAllCommandKinds)
        return commands
    }

    private func loadFixture(_ name: String) throws -> (GestureFixture, [CoreCommand]) {
        let url = try XCTUnwrap(Bundle(for: Self.self).url(forResource: name, withExtension: "json"))
        let fixture = try JSONDecoder().decode(GestureFixture.self, from: Data(contentsOf: url))
        let engine = try CoreEngine(settings: .rustDefaults())
        var commands: [CoreCommand] = []
        for frame in fixture.frames {
            var joints: [StandardJoint]
            if frame.handPresent == false {
                joints = []
            } else if let rightPose = frame.rightPose {
                joints = TestHand.rightPose(rightPose)
            } else if let pointerX = frame.pointerX {
                joints = TestHand.pointer(indexX: pointerX)
            } else if let indexX = frame.indexX, let indexY = frame.indexY {
                joints = TestHand.pinched(
                    indexX: indexX,
                    indexY: indexY,
                    thumbConfidence: frame.thumbConfidence ?? 1
                )
            } else {
                joints = TestHand.joints(
                    primaryPinch: frame.primaryPinch ?? false,
                    secondaryPinch: frame.secondaryPinch ?? false
                )
            }
            if let palmX = frame.palmX, let palmY = frame.palmY {
                joints = TestHand.shiftedPalm(joints, x: palmX, y: palmY)
            }
            if let ringTipConfidence = frame.ringTipConfidence {
                joints = joints.map { joint in
                    guard joint.kind == .ringTip else { return joint }
                    return StandardJoint(
                        kind: joint.kind,
                        x: joint.x,
                        y: joint.y,
                        confidence: ringTipConfidence
                    )
                }
            }
            if fixture.bimanual == true {
                var hands: [StandardHand] = []
                if frame.leftPresent != false, let pose = frame.leftPose {
                    hands.append(StandardHand(handedness: .left, joints: TestHand.leftPose(pose)))
                }
                if frame.handPresent != false {
                    hands.append(StandardHand(handedness: .right, joints: joints))
                }
                commands += try engine.process(timestamp: frame.timestamp, hands: hands)
            } else {
                commands += try engine.process(timestamp: frame.timestamp, joints: joints)
            }
        }
        return (fixture, commands)
    }
}

private struct GestureFixture: Decodable {
    struct Frame: Decodable {
        let timestamp: Double
        let primaryPinch: Bool?
        let secondaryPinch: Bool?
        let handPresent: Bool?
        let pointerX: Double?
        let indexX: Double?
        let indexY: Double?
        let thumbConfidence: Double?
        let ringTipConfidence: Double?
        let leftPose: String?
        let leftPresent: Bool?
        let rightPose: String?
        let palmX: Double?
        let palmY: Double?
    }
    let bimanual: Bool?
    let frames: [Frame]
    let expectedCommandKinds: [UInt32]
    let expectedAllCommandKinds: [UInt32]?
}

private enum TestHand {
    static func joints(primaryPinch: Bool = false, secondaryPinch: Bool = false) -> [StandardJoint] {
        let thumb = primaryPinch
            ? (0.445, 0.815)
            : secondaryPinch ? (0.555, 0.775) : (0.25, 0.55)
        return [
            .init(kind: .wrist, x: 0.50, y: 0.20, confidence: 1),
            .init(kind: .thumbMCP, x: 0.43, y: 0.39, confidence: 1),
            .init(kind: .thumbIP, x: 0.47, y: 0.43, confidence: 1),
            .init(kind: .thumbTip, x: thumb.0, y: thumb.1, confidence: 1),
            .init(kind: .indexMCP, x: 0.44, y: 0.42, confidence: 1),
            .init(kind: .indexPIP, x: 0.44, y: 0.62, confidence: 1),
            .init(kind: .indexTip, x: 0.44, y: 0.82, confidence: 1),
            .init(kind: .middleMCP, x: 0.53, y: 0.44, confidence: 1),
            .init(kind: .middlePIP, x: 0.54, y: 0.60, confidence: 1),
            .init(kind: .middleTip, x: 0.55, y: 0.78, confidence: 1),
            .init(kind: .ringMCP, x: 0.59, y: 0.42, confidence: 1),
            .init(kind: .ringPIP, x: 0.60, y: 0.53, confidence: 1),
            .init(kind: .ringTip, x: 0.60, y: 0.48, confidence: 1),
            .init(kind: .littleMCP, x: 0.64, y: 0.39, confidence: 1),
            .init(kind: .littlePIP, x: 0.66, y: 0.49, confidence: 1),
            .init(kind: .littleTip, x: 0.66, y: 0.44, confidence: 1)
        ]
    }

    static func jointsWithOutOfBoundsRingTip() -> [StandardJoint] {
        joints().map { joint in
            guard joint.kind == .ringTip else { return joint }
            return StandardJoint(kind: joint.kind, x: 1.02, y: joint.y, confidence: joint.confidence)
        }
    }

    static func pointer(indexX: Double) -> [StandardJoint] {
        joints().map { joint in
            switch joint.kind {
            case .indexTip:
                StandardJoint(kind: joint.kind, x: indexX, y: 0.82, confidence: 1)
            case .middleTip:
                StandardJoint(kind: joint.kind, x: 0.54, y: 0.52, confidence: 1)
            default:
                joint
            }
        }
    }

    static func pinched(
        indexX: Double,
        indexY: Double,
        thumbConfidence: Double = 1
    ) -> [StandardJoint] {
        joints(primaryPinch: true).map { joint in
            switch joint.kind {
            case .indexTip:
                StandardJoint(kind: joint.kind, x: indexX, y: indexY, confidence: 1)
            case .thumbTip:
                StandardJoint(
                    kind: joint.kind,
                    x: indexX + 0.005,
                    y: indexY - 0.005,
                    confidence: thumbConfidence
                )
            default:
                joint
            }
        }
    }

    static func horizontalPointer(indexX: Double) -> [StandardJoint] {
        [
            .init(kind: .wrist, x: 0.15, y: 0.50, confidence: 1),
            .init(kind: .thumbTip, x: 0.38, y: 0.72, confidence: 1),
            .init(kind: .indexMCP, x: 0.38, y: 0.56, confidence: 1),
            .init(kind: .indexPIP, x: 0.58, y: 0.56, confidence: 1),
            .init(kind: .indexTip, x: indexX, y: 0.56, confidence: 1),
            .init(kind: .middleMCP, x: 0.40, y: 0.49, confidence: 1),
            .init(kind: .middlePIP, x: 0.55, y: 0.49, confidence: 1),
            .init(kind: .middleTip, x: 0.48, y: 0.49, confidence: 1),
            .init(kind: .ringMCP, x: 0.39, y: 0.44, confidence: 1),
            .init(kind: .ringPIP, x: 0.52, y: 0.44, confidence: 1),
            .init(kind: .ringTip, x: 0.46, y: 0.44, confidence: 1),
            .init(kind: .littleMCP, x: 0.36, y: 0.39, confidence: 1),
            .init(kind: .littlePIP, x: 0.48, y: 0.39, confidence: 1),
            .init(kind: .littleTip, x: 0.42, y: 0.39, confidence: 1)
        ]
    }

    static func leftPose(_ pose: String) -> [StandardJoint] {
        var result = joints()
        switch pose {
        case "openPalm":
            result = replacing(result, .ringPIP, x: 0.60, y: 0.58)
            result = replacing(result, .ringTip, x: 0.61, y: 0.76)
            result = replacing(result, .littlePIP, x: 0.66, y: 0.55)
            result = replacing(result, .littleTip, x: 0.68, y: 0.72)
        case "thumbsUp":
            result = replacing(result, .indexTip, x: 0.44, y: 0.55)
            result = replacing(result, .middleTip, x: 0.54, y: 0.53)
            result = replacing(result, .thumbMCP, x: 0.39, y: 0.40)
            result = replacing(result, .thumbIP, x: 0.37, y: 0.56)
            result = replacing(result, .thumbTip, x: 0.35, y: 0.75)
        case "threeFingers":
            result = replacing(result, .ringPIP, x: 0.60, y: 0.58)
            result = replacing(result, .ringTip, x: 0.61, y: 0.76)
        default:
            XCTFail("Unknown left pose \(pose)")
        }
        return result
    }

    static func rightPose(_ pose: String) -> [StandardJoint] {
        switch pose {
        case "openPalm":
            return leftPose("openPalm")
        case "closing":
            return replacing(
                replacing(leftPose("openPalm"), .indexTip, x: 0.44, y: 0.55),
                .middleTip,
                x: 0.54,
                y: 0.53
            )
        case "fist":
            return replacing(
                replacing(joints(), .indexTip, x: 0.44, y: 0.55),
                .middleTip,
                x: 0.54,
                y: 0.53
            )
        default:
            XCTFail("Unknown right pose \(pose)")
            return joints()
        }
    }

    static func shiftedPalm(_ joints: [StandardJoint], x: Double, y: Double) -> [StandardJoint] {
        var result = replacing(joints, .wrist, x: x, y: y)
        result = replacing(result, .middleMCP, x: x + 0.03, y: y + 0.24)
        return result
    }

    private static func replacing(
        _ joints: [StandardJoint],
        _ kind: StandardJoint.Kind,
        x: Double,
        y: Double
    ) -> [StandardJoint] {
        joints.map { joint in
            joint.kind == kind ? StandardJoint(kind: kind, x: x, y: y, confidence: 1) : joint
        }
    }
}
