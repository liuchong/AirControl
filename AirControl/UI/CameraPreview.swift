import AVFoundation
import SwiftUI

struct CameraPreview: NSViewRepresentable {
    let session: AVCaptureSession
    let hands: [[StandardJoint]]
    let showsImage: Bool

    func makeNSView(context: Context) -> PreviewView {
        let view = PreviewView()
        view.previewLayer.session = session
        view.hands = hands
        view.showsImage = showsImage
        return view
    }

    func updateNSView(_ nsView: PreviewView, context: Context) {
        nsView.previewLayer.session = session
        nsView.hands = hands
        nsView.showsImage = showsImage
        nsView.needsLayout = true
    }
}

final class PreviewView: NSView {
    let previewLayer = AVCaptureVideoPreviewLayer()
    var hands: [[StandardJoint]] = []
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
            let points = Dictionary(uniqueKeysWithValues: visible.map { ($0.kind, mirroredPoint($0)) })
            let segments = Self.bones.compactMap { start, end -> (CGPoint, CGPoint)? in
                guard let a = points[start], let b = points[end] else { return nil }
                return (a, b)
            }
            return (segments, Array(points.values))
        }
        jointView.needsDisplay = true
    }

    /// Vision points use a bottom-left origin. The preview is a mirror, so the
    /// view matches the person looking at the screen: their right hand is on
    /// the right, and the top of the head stays at the top.
    private func mirroredPoint(_ joint: StandardJoint) -> CGPoint {
        let capture = CGPoint(x: joint.x, y: 1 - joint.y)
        if previewLayer.connection != nil, bounds.width > 1, bounds.height > 1 {
            let converted = previewLayer.layerPointConverted(fromCaptureDevicePoint: capture)
            if converted.x.isFinite, converted.y.isFinite {
                return converted
            }
        }
        return CGPoint(x: (1 - joint.x) * bounds.width, y: (1 - joint.y) * bounds.height)
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

    override var isOpaque: Bool { false }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    override func draw(_ dirtyRect: NSRect) {
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
