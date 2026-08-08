import SwiftUI

/// A monochrome, vector version of the app mark for tiny system surfaces.
/// The system supplies the foreground color so the mark remains legible in
/// both light and dark menu bars.
struct AirControlBrandMark: Shape {
    func path(in rect: CGRect) -> Path {
        let side = min(rect.width, rect.height)
        let ox = rect.midX - side / 2
        let oy = rect.midY - side / 2
        func point(_ x: CGFloat, _ y: CGFloat) -> CGPoint {
            CGPoint(x: ox + x * side, y: oy + y * side)
        }

        var path = Path()

        // Cursor-arrow A: two aerodynamic legs sharing one sharp apex.
        path.move(to: point(0.50, 0.04))
        path.addLine(to: point(0.78, 0.86))
        path.addLine(to: point(0.56, 0.70))
        path.addLine(to: point(0.50, 0.53))
        path.addLine(to: point(0.44, 0.70))
        path.addLine(to: point(0.22, 0.86))
        path.closeSubpath()

        // Mirrored airflow wings remain distinct at status-bar scale.
        path.move(to: point(0.08, 0.31))
        path.addCurve(to: point(0.39, 0.50), control1: point(0.14, 0.40), control2: point(0.28, 0.40))
        path.addCurve(to: point(0.07, 0.42), control1: point(0.27, 0.47), control2: point(0.15, 0.43))
        path.closeSubpath()
        path.move(to: point(0.13, 0.51))
        path.addCurve(to: point(0.35, 0.62), control1: point(0.20, 0.56), control2: point(0.28, 0.56))
        path.addCurve(to: point(0.12, 0.60), control1: point(0.27, 0.60), control2: point(0.18, 0.59))
        path.closeSubpath()

        path.move(to: point(0.92, 0.31))
        path.addCurve(to: point(0.61, 0.50), control1: point(0.86, 0.40), control2: point(0.72, 0.40))
        path.addCurve(to: point(0.93, 0.42), control1: point(0.73, 0.47), control2: point(0.85, 0.43))
        path.closeSubpath()
        path.move(to: point(0.87, 0.51))
        path.addCurve(to: point(0.65, 0.62), control1: point(0.80, 0.56), control2: point(0.72, 0.56))
        path.addCurve(to: point(0.88, 0.60), control1: point(0.73, 0.60), control2: point(0.82, 0.59))
        path.closeSubpath()

        return path
    }
}

struct AirControlMenuBarIcon: View {
    let runState: AppModel.RunState

    var body: some View {
        ZStack {
            switch runState {
            case .controlling:
                AirControlBrandMark().fill(.primary)
            default:
                AirControlBrandMark()
                    .stroke(.primary, style: StrokeStyle(lineWidth: 1.35, lineJoin: .round))
            }

            if case .paused = runState {
                pauseBadge
            } else if case .recovering = runState {
                recoveryBadge
            }
        }
        .frame(width: 18, height: 18)
        .accessibilityLabel(accessibilityLabel)
    }

    private var pauseBadge: some View {
        HStack(spacing: 1.5) {
            Capsule().frame(width: 1.8, height: 6)
            Capsule().frame(width: 1.8, height: 6)
        }
        .foregroundStyle(.primary)
        .padding(2)
        .background(.background, in: Circle())
        .offset(y: 4)
    }

    private var recoveryBadge: some View {
        Image(systemName: "arrow.clockwise")
            .font(.system(size: 6, weight: .bold))
            .padding(1.5)
            .background(.background, in: Circle())
            .offset(y: 4)
    }

    private var accessibilityLabel: String {
        "AirControl，\(runState.title)"
    }
}
