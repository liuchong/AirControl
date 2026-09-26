import CoreGraphics
import Vision

struct TrackedPoint: Equatable, Sendable {
    var x: Double
    var y: Double
    var confidence: Double
}

struct FaceChain: Equatable, Sendable {
    var points: [TrackedPoint]
    var closed: Bool
}

enum HeadGeometry {
    /// 面部关键点相对脸框，原点在脸框左下角。转成与手部关节点相同的整图归一化坐标。
    static func normalizedImagePoint(
        landmark: CGPoint,
        faceBox: CGRect,
        imageWidth: Int,
        imageHeight: Int
    ) -> CGPoint? {
        guard imageWidth > 0, imageHeight > 0,
              landmark.x.isFinite, landmark.y.isFinite else { return nil }
        let image = VNImagePointForFaceLandmarkPoint(
            SIMD2(Float(landmark.x), Float(landmark.y)),
            faceBox,
            imageWidth,
            imageHeight
        )
        guard image.x.isFinite, image.y.isFinite else { return nil }
        return CGPoint(
            x: image.x / CGFloat(imageWidth),
            y: image.y / CGFloat(imageHeight)
        )
    }

    static func chain(
        landmarks: [CGPoint],
        faceBox: CGRect,
        imageWidth: Int,
        imageHeight: Int,
        confidence: Double,
        closed: Bool
    ) -> FaceChain? {
        guard confidence >= 0.35 else { return nil }
        let points = landmarks.compactMap { landmark -> TrackedPoint? in
            guard let image = normalizedImagePoint(
                landmark: landmark,
                faceBox: faceBox,
                imageWidth: imageWidth,
                imageHeight: imageHeight
            ) else { return nil }
            return TrackedPoint(
                x: Double(image.x),
                y: Double(image.y),
                confidence: confidence
            )
        }
        guard !points.isEmpty else { return nil }
        return FaceChain(points: points, closed: closed && points.count >= 3)
    }
}
