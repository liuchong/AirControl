import AVFoundation
import ImageIO
import Vision

final class HandPoseEstimator: @unchecked Sendable {
    private let handRequest: VNDetectHumanHandPoseRequest = {
        let request = VNDetectHumanHandPoseRequest()
        request.maximumHandCount = 2
        return request
    }()
    private let faceRequest = VNDetectFaceLandmarksRequest()
    private let stateLock = NSLock()
    private var gazeEnabled = false

    func setGazeEnabled(_ enabled: Bool) {
        stateLock.lock()
        gazeEnabled = enabled
        stateLock.unlock()
    }

    func detect(in sampleBuffer: CMSampleBuffer) throws -> VisionFrameObservation {
        guard let pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else {
            return VisionFrameObservation(hands: [], gaze: nil)
        }
        let handler = VNImageRequestHandler(cvPixelBuffer: pixelBuffer, orientation: .up)
        stateLock.lock()
        let includeGaze = gazeEnabled
        stateLock.unlock()
        if includeGaze {
            try handler.perform([handRequest, faceRequest])
        } else {
            try handler.perform([handRequest])
        }
        let observations = handRequest.results ?? []

        let mapping: [(StandardJoint.Kind, VNHumanHandPoseObservation.JointName)] = [
            (.wrist, .wrist), (.thumbMCP, .thumbMP), (.thumbIP, .thumbIP), (.thumbTip, .thumbTip),
            (.indexMCP, .indexMCP), (.indexPIP, .indexPIP), (.indexTip, .indexTip),
            (.middleMCP, .middleMCP), (.middlePIP, .middlePIP), (.middleTip, .middleTip),
            (.ringMCP, .ringMCP), (.ringPIP, .ringPIP), (.ringTip, .ringTip),
            (.littleMCP, .littleMCP), (.littlePIP, .littlePIP), (.littleTip, .littleTip)
        ]
        let hands = observations.map { observation in
            let handedness: StandardHandedness = switch observation.chirality {
            case .left: .left
            case .right: .right
            default: .unknown
            }
            let joints: [StandardJoint] = mapping.compactMap { entry -> StandardJoint? in
                let (kind, visionName) = entry
                guard let point = try? observation.recognizedPoint(visionName) else { return nil }
                return StandardJoint(
                    kind: kind,
                    x: Double(point.location.x),
                    y: Double(point.location.y),
                    confidence: Double(point.confidence)
                )
            }
            return StandardHand(handedness: handedness, joints: joints)
        }
        return VisionFrameObservation(hands: hands, gaze: includeGaze ? gazeSample() : nil)
    }

    private func gazeSample() -> StandardGazeSample? {
        guard let face = faceRequest.results?.max(by: {
            $0.boundingBox.width * $0.boundingBox.height < $1.boundingBox.width * $1.boundingBox.height
        }),
        let landmarks = face.landmarks,
        let leftEye = landmarks.leftEye?.normalizedPoints,
        let rightEye = landmarks.rightEye?.normalizedPoints,
        let leftPupil = landmarks.leftPupil?.normalizedPoints.first,
        let rightPupil = landmarks.rightPupil?.normalizedPoints.first else { return nil }
        return GazeGeometry.sample(
            leftEye: leftEye,
            leftPupil: leftPupil,
            rightEye: rightEye,
            rightPupil: rightPupil,
            faceConfidence: Double(face.confidence)
        )
    }
}

struct VisionFrameObservation: Sendable {
    let hands: [StandardHand]
    let gaze: StandardGazeSample?
}
