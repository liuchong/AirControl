import AVFoundation
import SwiftUI

struct CameraPreview: NSViewRepresentable {
    let session: AVCaptureSession
    let hands: [[StandardJoint]]
    let head: [FaceChain]
    let showsImage: Bool

    func makeNSView(context: Context) -> PreviewView {
        let view = PreviewView()
        view.previewLayer.session = session
        view.hands = hands
        view.head = head
        view.showsImage = showsImage
        return view
    }

    func updateNSView(_ nsView: PreviewView, context: Context) {
        nsView.previewLayer.session = session
        nsView.hands = hands
        nsView.head = head
        nsView.showsImage = showsImage
        nsView.needsLayout = true
    }
}

final class PreviewView: NSView {
    let previewLayer = AVCaptureVideoPreviewLayer()
    var hands: [[StandardJoint]] = []
    var head: [FaceChain] = []
    var showsImage = false {
        didSet { previewLayer.opacity = showsImage ? 1 : 0 }
    }

    private let jointView = JointDrawingView()

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer?.backgroundColor = NSColor.black.cgColor
        previewLayer.videoGravity = .resizeAspect
        previewLayer.opacity = 0
        layer?.addSublayer(previewLayer)
        jointView.frame = bounds
        addSubview(jointView)
    }

    required init?(coder: NSCoder) { nil }

    override func layout() {
        super.layout()
        previewLayer.frame = bounds
        jointView.frame = bounds
        if let connection = previewLayer.connection, connection.isVideoMirroringSupported {
            connection.automaticallyAdjustsVideoMirroring = false
            connection.isVideoMirrored = true
        }
        refreshJoints()
    }

    private func refreshJoints() {
        jointView.hands = hands.map { hand in
            let visible = hand.filter { $0.confidence >= 0.35 }
            let points = Dictionary(uniqueKeysWithValues: visible.map { ($0.kind, mirroredPoint(x: $0.x, y: $0.y)) })
            let segments = Self.bones.compactMap { start, end -> (CGPoint, CGPoint)? in
                guard let a = points[start], let b = points[end] else { return nil }
                return (a, b)
            }
            return (segments, Array(points.values))
        }
        jointView.head = head.map { chain in
            let points = chain.points
                .filter { $0.confidence >= 0.35 }
                .map { mirroredPoint(x: $0.x, y: $0.y) }
            return (points, chain.closed)
        }
        jointView.needsDisplay = true
    }

    /// 水平镜像让观看者的右侧落在画面右侧。纵轴再把预览层的换算结果翻回视图，
    /// 这样头顶和举手都留在画面上方。
    private func mirroredPoint(x: Double, y: Double) -> CGPoint {
        let capture = CGPoint(x: x, y: 1 - y)
        var converted: CGPoint?
        if previewLayer.connection != nil {
            let point = previewLayer.layerPointConverted(fromCaptureDevicePoint: capture)
            if point.x.isFinite, point.y.isFinite {
                converted = point
            }
        }
        return PreviewMapping.viewPoint(
            visionX: capture.x,
            visionY: CGFloat(y),
            bounds: bounds,
            convertedLayerPoint: converted
        )
    }

    private static let bones: [(StandardJoint.Kind, StandardJoint.Kind)] = [
        (.wrist, .thumbMCP), (.thumbMCP, .thumbIP), (.thumbIP, .thumbTip),
        (.wrist, .indexMCP), (.indexMCP, .indexPIP), (.indexPIP, .indexTip),
        (.wrist, .middleMCP), (.middleMCP, .middlePIP), (.middlePIP, .middleTip),
        (.wrist, .ringMCP), (.ringMCP, .ringPIP), (.ringPIP, .ringTip),
        (.wrist, .littleMCP), (.littleMCP, .littlePIP), (.littlePIP, .littleTip)
    ]
}

private final class JointDrawingView: NSView {
    var hands: [([(CGPoint, CGPoint)], [CGPoint])] = []
    var head: [([CGPoint], Bool)] = []

    override var isOpaque: Bool { false }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    override func draw(_ dirtyRect: NSRect) {
        let face = NSBezierPath()
        face.lineWidth = 1.5
        for (points, closed) in head {
            guard let first = points.first else { continue }
            face.move(to: first)
            for point in points.dropFirst() {
                face.line(to: point)
            }
            if closed { face.close() }
        }
        NSColor.systemOrange.withAlphaComponent(0.9).setStroke()
        face.stroke()
        NSColor.systemOrange.setFill()
        for (points, _) in head {
            for point in points {
                NSBezierPath(ovalIn: CGRect(x: point.x - 3, y: point.y - 3, width: 6, height: 6)).fill()
            }
        }

        NSColor.systemMint.withAlphaComponent(0.85).setStroke()
        let bones = NSBezierPath()
        bones.lineWidth = 2
        for hand in hands {
            for (start, end) in hand.0 {
                bones.move(to: start)
                bones.line(to: end)
            }
        }
        bones.stroke()
        NSColor.systemMint.setFill()
        for hand in hands {
            for point in hand.1 {
                NSBezierPath(ovalIn: CGRect(x: point.x - 4, y: point.y - 4, width: 8, height: 8)).fill()
            }
        }
    }
}
