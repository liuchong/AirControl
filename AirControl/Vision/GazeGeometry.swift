import CoreGraphics

enum GazeGeometry {
    static func sample(
        leftEye: [CGPoint],
        leftPupil: CGPoint,
        rightEye: [CGPoint],
        rightPupil: CGPoint,
        faceConfidence: Double
    ) -> StandardGazeSample? {
        guard faceConfidence.isFinite, (0...1).contains(faceConfidence),
              let left = normalizedPupil(eye: leftEye, pupil: leftPupil),
              let right = normalizedPupil(eye: rightEye, pupil: rightPupil) else { return nil }
        return StandardGazeSample(
            x: (left.x + right.x) / 2,
            y: (left.y + right.y) / 2,
            confidence: faceConfidence,
            eyesOpen: true
        )
    }

    private static func normalizedPupil(eye: [CGPoint], pupil: CGPoint) -> CGPoint? {
        guard eye.count >= 4, pupil.x.isFinite, pupil.y.isFinite else { return nil }
        let minX = eye.map(\.x).min() ?? 0
        let maxX = eye.map(\.x).max() ?? 0
        let minY = eye.map(\.y).min() ?? 0
        let maxY = eye.map(\.y).max() ?? 0
        let width = maxX - minX
        let height = maxY - minY
        guard width >= 0.01, height >= 0.005, height / width >= 0.08 else { return nil }
        let x = (pupil.x - minX) / width
        let y = (pupil.y - minY) / height
        guard (-0.15...1.15).contains(x), (-0.15...1.15).contains(y) else { return nil }
        return CGPoint(x: x.clamped(to: 0...1), y: y.clamped(to: 0...1))
    }
}

private extension CGFloat {
    func clamped(to range: ClosedRange<CGFloat>) -> CGFloat {
        Swift.min(Swift.max(self, range.lowerBound), range.upperBound)
    }
}
