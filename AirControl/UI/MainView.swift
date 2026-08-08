import SwiftUI

struct MainView: View {
    @EnvironmentObject private var model: AppModel

    var body: some View {
        HStack(spacing: 0) {
            preview
            controls
                .frame(width: 310)
                .background(.regularMaterial)
        }
        .frame(minWidth: 920, minHeight: 600)
        .task { await model.start() }
        .sheet(isPresented: $model.showingHelp) { HelpView() }
    }

    private var preview: some View {
        ZStack(alignment: .topLeading) {
            Color.black
            CameraPreview(session: model.camera.session)
            JointOverlay(joints: model.detectedJoints)
            VStack(alignment: .leading, spacing: 6) {
                Label(model.runState.title, systemImage: statusIcon)
                    .font(.headline)
                Text(model.lastGesture).font(.caption).foregroundStyle(.secondary)
                if let detail = model.runState.detailMessage {
                    Text(detail)
                        .font(.caption)
                        .foregroundStyle(.red)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            .padding(12)
            .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 12))
            .padding(16)
        }
        .clipShape(RoundedRectangle(cornerRadius: 18))
        .padding(16)
    }

    private var controls: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                Text("AirControl").font(.largeTitle.bold())
                Text("在本机用手势控制 Mac").foregroundStyle(.secondary)
                permissionCard(
                    title: "摄像头",
                    granted: model.cameraAuthorized,
                    action: { PermissionManager.openCameraSettings() }
                )
                permissionCard(
                    title: "辅助功能",
                    granted: model.accessibilityGranted,
                    action: { PermissionManager.openAccessibilitySettings() }
                )
                Picker("摄像头", selection: Binding(
                    get: { model.selectedCameraID },
                    set: { id in Task { await model.selectCamera(id) } }
                )) {
                    ForEach(model.cameras, id: \.uniqueID) { camera in
                        Text(camera.localizedName).tag(Optional(camera.uniqueID))
                    }
                }
                Picker("目标显示器", selection: Binding(
                    get: { model.selectedDisplayID },
                    set: { value in model.selectDisplay(value) }
                )) {
                    ForEach(model.displays) { display in
                        Text(display.name).tag(Optional(display.id))
                    }
                }
                VStack(alignment: .leading) {
                    HStack { Text("光标平滑"); Spacer(); Text(model.smoothing, format: .number.precision(.fractionLength(2))) }
                    Slider(value: Binding(
                        get: { model.smoothing },
                        set: { value in model.updateSmoothing(value) }
                    ), in: 0.05...1.0)
                }
                VStack(alignment: .leading, spacing: 8) {
                    Toggle("视线辅助", isOn: Binding(
                        get: { model.gazeAssistEnabled },
                        set: { model.setGazeAssistEnabled($0) }
                    ))
                    Text("无手时用视线粗定位；手进入画面后从当前位置精细接管，不使用眨眼点击。")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text(model.gazeStatus)
                        .font(.caption)
                        .foregroundStyle(model.gazeAssistEnabled ? .mint : .secondary)
                    if case .calibratingGaze = model.runState {
                        ProgressView(value: model.gazeCalibrationProgress)
                        Button("取消视线校准") { model.cancelGazeCalibration() }
                    } else if model.gazeCalibrated {
                        Button("重新进行九点校准") { model.beginGazeCalibration() }
                    } else {
                        Button("开始九点校准") { model.beginGazeCalibration() }
                    }
                }
                .padding(12)
                .background(.quaternary, in: RoundedRectangle(cornerRadius: 12))
                VStack(alignment: .leading, spacing: 8) {
                    Text("启用的手势").fontWeight(.medium)
                    ForEach(GestureOption.allCases) { option in
                        Toggle(option.title, isOn: Binding(
                            get: { model.gestureEnabled(option) },
                            set: { model.updateGesture(option, enabled: $0) }
                        ))
                    }
                    Text("修改手势会先安全停止控制，需要重新开启。")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                VStack(alignment: .leading, spacing: 6) {
                    Text("左手辅助（固定开启）").fontWeight(.medium)
                    Text("张掌精准稳定 · 食指锁定光标 · 竖拇指双击 · V 形掌控滚动 · 三指拖拽 · 握拳暂停")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .padding(12)
                .background(.quaternary, in: RoundedRectangle(cornerRadius: 12))
                VStack(alignment: .leading, spacing: 8) {
                    Text("活动范围校准").fontWeight(.medium)
                    if case .calibrating = model.runState {
                        Text(model.calibrationMessage).font(.caption).foregroundStyle(.secondary)
                        ProgressView(value: model.calibrationProgress)
                        Button("取消校准") { model.cancelCalibration() }
                    } else {
                        Text(model.calibrationMessage.isEmpty ? "用两个稳定位置匹配舒适活动范围" : model.calibrationMessage)
                            .font(.caption).foregroundStyle(.secondary)
                        Button("开始两点校准") { model.beginCalibration() }
                    }
                }
                .padding(12)
                .background(.quaternary, in: RoundedRectangle(cornerRadius: 12))
                Button(action: model.toggleControl) {
                    Label(controlButtonTitle, systemImage: controlButtonIcon)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                }
                .buttonStyle(.borderedProminent)
                Button("手势与安全说明") { model.showingHelp = true }
                    .frame(maxWidth: .infinity)
            }
            .padding(22)
        }
    }

    private func permissionCard(title: String, granted: Bool, action: @escaping () -> Void) -> some View {
        HStack {
            Image(systemName: granted ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                .foregroundStyle(granted ? .green : .orange)
            VStack(alignment: .leading) {
                Text(title).fontWeight(.medium)
                Text(granted ? "已授权" : "需要授权").font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            if !granted { Button("设置", action: action) }
        }
        .padding(12)
        .background(.quaternary, in: RoundedRectangle(cornerRadius: 12))
    }

    private var statusIcon: String {
        switch model.runState {
        case .controlling: "cursorarrow.motionlines"
        case .calibrating, .calibratingGaze: "scope"
        case .paused: "pause.circle.fill"
        case .recovering: "arrow.triangle.2.circlepath.circle.fill"
        case .error: "exclamationmark.octagon.fill"
        default: "camera.fill"
        }
    }

    private var controlButtonTitle: String {
        switch model.runState {
        case .controlling, .paused, .recovering: "停止控制"
        default: "开启控制"
        }
    }

    private var controlButtonIcon: String {
        switch model.runState {
        case .controlling, .paused, .recovering: "stop.fill"
        default: "play.fill"
        }
    }
}
