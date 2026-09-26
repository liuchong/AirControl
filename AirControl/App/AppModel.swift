import AppKit
import AVFoundation
import QuartzCore
import SwiftUI

@MainActor
final class AppModel: ObservableObject {
    enum RunState: Equatable {
        case idle
        case requestingCameraPermission
        case previewOnly
        case calibrating
        case calibratingGaze
        case controlling
        case paused
        case recovering(String)
        case error(String)

        var title: String {
            switch self {
            case .idle: "未启动"
            case .requestingCameraPermission: "等待摄像头权限"
            case .previewOnly: "仅预览"
            case .calibrating: "正在校准"
            case .calibratingGaze: "正在校准视线"
            case .controlling: "控制中"
            case .paused: "已暂停"
            case .recovering: "恢复中"
            case .error: "发生错误"
            }
        }

        var detailMessage: String? {
            switch self {
            case .recovering(let message), .error(let message): message
            default: nil
            }
        }
    }

    @Published private(set) var runState: RunState = .idle
    @Published private(set) var cameraAuthorized = false
    @Published private(set) var accessibilityGranted = false
    @Published private(set) var detectedHands: [[StandardJoint]] = []
    @Published private(set) var lastGesture = "等待手势"
    @Published private(set) var calibrationMessage = ""
    @Published private(set) var calibrationProgress = 0.0
    @Published private(set) var gazeAssistEnabled = false
    @Published private(set) var gazeCalibrated = false
    @Published private(set) var gazeStatus = "视线辅助已关闭"
    @Published private(set) var gazeCalibrationStage = 0
    @Published private(set) var gazeCalibrationProgress = 0.0
    @Published private(set) var fillLightSettings = FillLightSettings.default
    @Published private(set) var showsCameraImage = false
    @Published var selectedCameraID: String?
    @Published var selectedDisplayID: CGDirectDisplayID?
    @Published var showingHelp = false

    let camera = CameraService()
    let cameras = CameraService.availableDevices
    var displays: [DisplayDescriptor] { DisplayDescriptor.available }

    private let estimator = HandPoseEstimator()
    private let cursor = SystemCursorController()
    private var settingsStore: SettingsStore?
    private var core: CoreEngine?
    private var controlEnabled = false
    private var gesturePaused = false
    private var calibration: CalibrationController?
    private var gazeCalibration: GazeCalibrationController?
    private var gazeMapper: GazeMapper?
    private let gazeOverlay = GazeCalibrationOverlayController()
    private let fillLightOverlay = FillLightOverlayController()
    private var gazeCalibrationResumePolicy = GazeCalibrationResumePolicy()
    private var lastAnyHandSeenAt: Double?
    private var gazeSuppressedByHand = false
    private var visionFailureTracker = VisionFailureTracker(failureLimit: 5, recoverySuccessLimit: 2)
    private var visionSuspended = false
    private var controlGeneration = 0
    private var hasStarted = false
    private var hotKey: GlobalHotKey?
    private var notificationObservers: [NSObjectProtocol] = []

    init() {
        camera.onFrame = { [weak self] sampleBuffer in
            guard let self else { return }
            let timestamp = CACurrentMediaTime()
            do {
                let observation = try self.estimator.detect(in: sampleBuffer)
                Task { @MainActor [weak self] in
                    self?.handleVisionSuccess(
                        timestamp: timestamp,
                        hands: observation.hands,
                        gaze: observation.gaze
                    )
                }
            } catch {
                let message = error.localizedDescription
                Task { @MainActor [weak self] in
                    self?.handleVisionFailure(message, timestamp: timestamp)
                }
            }
        }
        camera.onError = { [weak self] error in
            Task { @MainActor [weak self] in
                self?.handleCameraError(error.localizedDescription)
            }
        }
        do {
            let store = try SettingsStore()
            settingsStore = store

            let preferredCameraID = AVCaptureDevice.systemPreferredCamera?.uniqueID
                ?? cameras.first?.uniqueID
            selectedCameraID = store.cameraID.flatMap { savedID in
                cameras.contains(where: { $0.uniqueID == savedID }) ? savedID : nil
            } ?? preferredCameraID
            store.saveCameraID(selectedCameraID)

            let displaySelection = DisplayDescriptor.resolveSelection(
                selectedID: store.displayID,
                available: displays
            )
            selectedDisplayID = displaySelection.id
            store.saveDisplayID(selectedDisplayID)
            try rebuildCore(using: store.settings)
            gazeAssistEnabled = store.gazeAssistEnabled
            gazeCalibrated = store.gazeProfile != nil
            fillLightSettings = store.fillLightSettings
            showsCameraImage = store.showsCameraImage
            fillLightOverlay.apply(settings: fillLightSettings, displayID: selectedDisplayID)
            if gazeAssistEnabled {
                try rebuildGazeMapper(using: store.settings)
                gazeStatus = "等待清晰视线"
            }
            estimator.setGazeEnabled(gazeAssistEnabled)
        } catch {
            runState = .error(error.localizedDescription)
        }
        hotKey = GlobalHotKey { [weak self] in
            Task { @MainActor [weak self] in self?.toggleControl() }
        }
        notificationObservers.append(NotificationCenter.default.addObserver(
            forName: NSApplication.willTerminateNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            MainActor.assumeIsolated { self?.safeStop() }
        })
        notificationObservers.append(NotificationCenter.default.addObserver(
            forName: NSApplication.didChangeScreenParametersNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor [weak self] in self?.handleDisplaysChanged() }
        })
        notificationObservers.append(NotificationCenter.default.addObserver(
            forName: NSApplication.didBecomeActiveNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor [weak self] in self?.refreshAccessibility() }
        })
    }

    func start() async {
        guard !hasStarted else { return }
        hasStarted = true
        runState = .requestingCameraPermission
        let status = PermissionManager.cameraStatus
        if status == .authorized {
            cameraAuthorized = true
        } else if status == .notDetermined {
            cameraAuthorized = await PermissionManager.requestCamera()
        } else {
            cameraAuthorized = false
        }
        refreshAccessibility()
        guard cameraAuthorized else {
            runState = .error("需要摄像头权限才能识别手势")
            return
        }
        do {
            try await camera.configure(deviceID: selectedCameraID)
            camera.start()
            runState = .previewOnly
        } catch {
            runState = .error(error.localizedDescription)
        }
    }

    func toggleControl() {
        if gazeCalibration != nil {
            cancelGazeCalibration()
            return
        }
        refreshAccessibility()
        guard cameraAuthorized else {
            PermissionManager.openCameraSettings()
            return
        }
        guard accessibilityGranted else {
            PermissionManager.promptForAccessibility()
            refreshAccessibility()
            return
        }
        if controlEnabled {
            safeStop()
        } else {
            beginControlActivation()
        }
    }

    func safeStop() {
        controlGeneration += 1
        controlEnabled = false
        gesturePaused = false
        visionSuspended = false
        visionFailureTracker.reset()
        if let core, let commands = try? core.stop() { try? cursor.execute(commands) }
        try? core?.clearPointerRebase()
        cursor.releaseAll()
        lastAnyHandSeenAt = nil
        gazeSuppressedByHand = false
        gazeCalibrationResumePolicy.reset()
        if cameraAuthorized { runState = .previewOnly }
    }

    func refreshAccessibility() {
        accessibilityGranted = PermissionManager.accessibilityGranted
        if !accessibilityGranted && controlEnabled { safeStop() }
    }

    func selectCamera(_ id: String?) async {
        safeStop()
        let available = CameraService.availableDevices
        let resolvedID = available.first(where: { $0.uniqueID == id })?.uniqueID
            ?? AVCaptureDevice.systemPreferredCamera?.uniqueID
            ?? available.first?.uniqueID
        selectedCameraID = resolvedID
        do {
            try await camera.configure(deviceID: resolvedID)
            camera.start()
            settingsStore?.saveCameraID(resolvedID)
        } catch {
            runState = .error(error.localizedDescription)
        }
    }

    func selectDisplay(_ id: CGDirectDisplayID?) {
        safeStop()
        let availableDisplays = displays
        let resolvedID = DisplayDescriptor.resolveSelection(
            selectedID: id,
            available: availableDisplays
        ).id
        selectedDisplayID = resolvedID
        guard let store = settingsStore,
              let display = availableDisplays.first(where: { $0.id == resolvedID }) else { return }
        var settings = store.settings
        settings.screenOriginX = display.bounds.origin.x
        settings.screenOriginY = display.bounds.origin.y
        settings.screenWidth = display.bounds.width
        settings.screenHeight = display.bounds.height
        do {
            try store.save(settings)
            try rebuildCore(using: settings)
            if gazeAssistEnabled { try rebuildGazeMapper(using: settings) }
            store.saveDisplayID(resolvedID)
            fillLightOverlay.apply(settings: fillLightSettings, displayID: resolvedID)
        } catch {
            runState = .error(error.localizedDescription)
        }
    }

    func updateSmoothing(_ value: Double) {
        guard let store = settingsStore else { return }
        var settings = store.settings
        settings.smoothing = value
        do {
            safeStop()
            try store.save(settings)
            try rebuildCore(using: settings)
            if gazeAssistEnabled { try rebuildGazeMapper(using: settings) }
        } catch {
            runState = .error(error.localizedDescription)
        }
    }

    var smoothing: Double { settingsStore?.settings.smoothing ?? 0.32 }

    func setShowsCameraImage(_ shows: Bool) {
        showsCameraImage = shows
        settingsStore?.saveShowsCameraImage(shows)
    }

    func updateFillLight(enabled: Bool? = nil, brightness: Double? = nil, warmth: Double? = nil) {
        guard let store = settingsStore else { return }
        let next = FillLightSettings(
            enabled: enabled ?? fillLightSettings.enabled,
            brightness: brightness ?? fillLightSettings.brightness,
            warmth: warmth ?? fillLightSettings.warmth
        )
        guard next.isValid else { return }
        store.saveFillLight(next)
        fillLightSettings = next
        fillLightOverlay.apply(settings: next, displayID: selectedDisplayID)
    }

    func gestureEnabled(_ option: GestureOption) -> Bool {
        guard let settings = settingsStore?.settings else { return false }
        return settings.enabledGestures & option.rawValue != 0
    }

    func updateGesture(_ option: GestureOption, enabled: Bool) {
        guard let store = settingsStore else { return }
        var settings = store.settings
        if enabled {
            settings.enabledGestures |= option.rawValue
        } else {
            settings.enabledGestures &= ~option.rawValue
        }
        do {
            safeStop()
            try store.save(settings)
            try rebuildCore(using: settings)
        } catch {
            runState = .error(error.localizedDescription)
        }
    }

    func setGazeAssistEnabled(_ enabled: Bool) {
        guard let store = settingsStore else { return }
        if !enabled {
            safeStop()
            store.saveGazeAssist(enabled: false, profile: store.gazeProfile)
            gazeAssistEnabled = false
            gazeMapper = nil
            estimator.setGazeEnabled(false)
            gazeStatus = store.gazeProfile == nil ? "尚未校准" : "视线辅助已关闭"
            return
        }
        guard store.gazeProfile != nil else {
            beginGazeCalibration()
            return
        }
        do {
            safeStop()
            store.saveGazeAssist(enabled: true, profile: store.gazeProfile)
            gazeAssistEnabled = true
            gazeCalibrated = true
            try rebuildGazeMapper(using: store.settings)
            estimator.setGazeEnabled(true)
            gazeStatus = "等待清晰视线"
        } catch {
            runState = .error(error.localizedDescription)
        }
    }

    func beginGazeCalibration() {
        let wasControlEnabled = controlEnabled
        safeStop()
        gazeCalibrationResumePolicy.begin(wasControlEnabled: wasControlEnabled)
        do {
            guard let settings = settingsStore?.settings else { return }
            gazeCalibration = try GazeCalibrationController(
                minimumConfidence: max(settings.minimumConfidence, 0.55)
            )
            gazeCalibrationStage = 0
            gazeCalibrationProgress = 0
            gazeStatus = "注视第 1 个圆点"
            estimator.setGazeEnabled(true)
            gazeOverlay.show(displayID: selectedDisplayID) { [weak self] in
                self?.cancelGazeCalibration()
            }
            gazeOverlay.update(stage: 0, progress: 0)
            fillLightOverlay.refreshForCalibration(
                settings: fillLightSettings,
                displayID: selectedDisplayID
            )
            runState = .calibratingGaze
        } catch {
            gazeCalibrationResumePolicy.reset()
            gazeCalibration = nil
            estimator.setGazeEnabled(gazeAssistEnabled)
            runState = .error(error.localizedDescription)
        }
    }

    func cancelGazeCalibration() {
        _ = gazeCalibrationResumePolicy.finish(
            .cancelled,
            cameraAuthorized: cameraAuthorized,
            accessibilityGranted: accessibilityGranted
        )
        gazeCalibration = nil
        gazeOverlay.close()
        gazeCalibrationProgress = 0
        estimator.setGazeEnabled(gazeAssistEnabled)
        gazeStatus = gazeAssistEnabled ? "等待清晰视线" : "视线校准已取消"
        runState = cameraAuthorized ? .previewOnly : .idle
    }

    func beginCalibration() {
        safeStop()
        do {
            guard let minimumConfidence = settingsStore?.settings.minimumConfidence else { return }
            calibration = try CalibrationController(minimumConfidence: minimumConfidence)
            calibrationMessage = "将食指稳定放在舒适活动范围的左上角"
            calibrationProgress = 0
            runState = .calibrating
        } catch {
            runState = .error(error.localizedDescription)
        }
    }

    func cancelCalibration() {
        calibration = nil
        calibrationMessage = ""
        calibrationProgress = 0
        runState = cameraAuthorized ? .previewOnly : .idle
    }

    private func activateControl(generation: Int) async {
        do {
            try await camera.configure(deviceID: selectedCameraID)
            camera.start()
            guard generation == controlGeneration, controlEnabled else { return }
            visionSuspended = false
            visionFailureTracker.reset()
            seedCursorToSystemPointer()
            runState = .controlling
        } catch {
            guard generation == controlGeneration else { return }
            controlEnabled = false
            visionSuspended = false
            runState = .error(error.localizedDescription)
        }
    }

    private func beginControlActivation() {
        controlGeneration += 1
        let generation = controlGeneration
        controlEnabled = true
        gesturePaused = false
        visionSuspended = true
        visionFailureTracker.reset()
        runState = .recovering("正在重新连接摄像头")
        Task { [weak self] in
            await self?.activateControl(generation: generation)
        }
    }

    private func handleVisionSuccess(
        timestamp: Double,
        hands: [StandardHand],
        gaze: StandardGazeSample?
    ) {
        detectedHands = hands.map(\.joints)
        switch visionFailureTracker.recordSuccess() {
        case .continueControl:
            consume(timestamp: timestamp, hands: hands, gaze: gaze)
        case .remainSuspended:
            break
        case .resumeControl:
            visionSuspended = false
            if controlEnabled { runState = .controlling }
            consume(timestamp: timestamp, hands: hands, gaze: gaze)
        case .forwardMissingFrame, .forwardMissingFrameAndSuspend:
            break
        }
    }

    private func consume(
        timestamp: Double,
        hands: [StandardHand],
        gaze: StandardGazeSample?
    ) {
        detectedHands = hands.map(\.joints)
        if let gazeCalibration {
            do {
                let event = try gazeCalibration.update(timestamp: timestamp, sample: gaze)
                handleGazeCalibration(event)
            } catch {
                cancelGazeCalibration()
                runState = .error(error.localizedDescription)
            }
            return
        }
        if let calibration {
            do {
                let controllingHand = hands.first { $0.handedness == .right }
                    ?? hands.first { $0.handedness == .unknown }
                let index = controllingHand?.joints.first { $0.kind == .indexTip }
                let event = try calibration.update(timestamp: timestamp, point: index)
                handleCalibration(event)
            } catch {
                self.calibration = nil
                runState = .error(error.localizedDescription)
            }
            return
        }
        guard controlEnabled, !visionSuspended, accessibilityGranted, let core else { return }
        do {
            let commands = try core.process(timestamp: timestamp, hands: hands)
            for command in commands {
                if case .pauseChanged(let paused) = command {
                    gesturePaused = paused
                    runState = paused ? .paused : .controlling
                }
                lastGesture = label(for: command)
            }
            try cursor.execute(commands)
            if !hands.isEmpty {
                lastAnyHandSeenAt = timestamp
                gazeSuppressedByHand = true
                if gazeAssistEnabled { gazeStatus = "手势精细控制中" }
                return
            }
            guard gazeAssistEnabled, !gesturePaused, let settings = settingsStore?.settings else {
                return
            }
            let handReleaseDelay = settings.handLossSeconds
            if let lastAnyHandSeenAt, timestamp - lastAnyHandSeenAt < handReleaseDelay {
                gazeStatus = "等待手势安全释放"
                return
            }
            if gazeSuppressedByHand {
                try rebuildGazeMapper(using: settings)
                gazeSuppressedByHand = false
            }
            guard let point = try gazeMapper?.update(timestamp: timestamp, sample: gaze) else {
                gazeStatus = "等待清晰视线"
                return
            }
            try core.rebasePointer(to: point)
            try cursor.execute([.move(x: point.x, y: point.y)])
            lastGesture = "视线粗定位"
            gazeStatus = "视线已识别"
        } catch {
            safeStop()
            runState = .error(error.localizedDescription)
        }
    }

    private func seedCursorToSystemPointer() {
        guard let core, let settings = settingsStore?.settings else { return }
        let mouse = NSEvent.mouseLocation
        let x = min(max(mouse.x, settings.screenOriginX), settings.screenOriginX + settings.screenWidth)
        let y = min(max(mouse.y, settings.screenOriginY), settings.screenOriginY + settings.screenHeight)
        guard (try? core.rebasePointer(to: CGPoint(x: x, y: y))) != nil else { return }
        try? core.clearPointerRebase()
    }

    private func rebuildCore(using original: CoreSettings) throws {
        var settings = original
        if let display = displays.first(where: { $0.id == selectedDisplayID }) ?? displays.first {
            settings.screenOriginX = display.bounds.origin.x
            settings.screenOriginY = display.bounds.origin.y
            settings.screenWidth = display.bounds.width
            settings.screenHeight = display.bounds.height
        }
        core = try CoreEngine(settings: settings)
    }

    private func rebuildGazeMapper(using settings: CoreSettings) throws {
        guard let profile = settingsStore?.gazeProfile,
              let display = displays.first(where: { $0.id == selectedDisplayID }) ?? displays.first else {
            gazeMapper = nil
            return
        }
        gazeMapper = try GazeMapper(
            profile: profile,
            screen: display.bounds,
            minimumConfidence: max(settings.minimumConfidence, 0.55),
            smoothing: min(settings.smoothing, 0.55)
        )
    }

    private func handleCameraError(_ message: String) {
        safeStop()
        camera.stop()
        runState = .error(message)
    }

    private func handleVisionFailure(_ message: String, timestamp: Double) {
        let action = visionFailureTracker.recordFailure()
        switch action {
        case .forwardMissingFrame:
            consume(timestamp: timestamp, hands: [], gaze: nil)
        case .forwardMissingFrameAndSuspend:
            consume(timestamp: timestamp, hands: [], gaze: nil)
            suspendForVisionRecovery(message: message)
        case .remainSuspended, .continueControl, .resumeControl:
            break
        }
    }

    private func suspendForVisionRecovery(message: String) {
        visionSuspended = true
        gesturePaused = false
        if let core, let commands = try? core.stop() { try? cursor.execute(commands) }
        cursor.releaseAll()
        if controlEnabled {
            runState = .recovering("摄像头识别恢复中：\(message)")
            lastGesture = "已安全释放鼠标，等待识别恢复"
        }
    }

    private func handleDisplaysChanged() {
        let availableDisplays = displays
        let selection = DisplayDescriptor.resolveSelection(
            selectedID: selectedDisplayID,
            available: availableDisplays
        )
        guard selection.didFallback else { return }

        safeStop()
        selectedDisplayID = selection.id
        guard let store = settingsStore,
              let display = availableDisplays.first(where: { $0.id == selection.id }) else {
            runState = .error("没有可用显示器")
            return
        }
        var settings = store.settings
        settings.screenOriginX = display.bounds.origin.x
        settings.screenOriginY = display.bounds.origin.y
        settings.screenWidth = display.bounds.width
        settings.screenHeight = display.bounds.height
        do {
            try store.save(settings)
            try rebuildCore(using: settings)
            if gazeAssistEnabled { try rebuildGazeMapper(using: settings) }
            store.saveDisplayID(selection.id)
            fillLightOverlay.apply(settings: fillLightSettings, displayID: selection.id)
            lastGesture = "目标显示器已断开，控制已安全停止"
            runState = .previewOnly
        } catch {
            runState = .error(error.localizedDescription)
        }
    }

    private func label(for command: CoreCommand) -> String {
        switch command {
        case .move: "移动光标"
        case .click(let button, _, _, let count):
            if button == .right { "右键单击" } else if count == 2 { "左键双击" } else { "左键单击" }
        case .mouseDown(let button, _, _): button == .left ? "按下左键" : "按下右键"
        case .mouseUp(let button, _, _): button == .left ? "释放左键" : "释放右键"
        case .scroll: "滚动"
        case .windowGrabBegin: "抓住窗口"
        case .windowMove: "移动窗口"
        case .windowGrabEnd: "松开窗口"
        case .showAppOverview: "打开应用调度台"
        case .pauseChanged(let paused): paused ? "左手握拳暂停" : "左手握拳恢复"
        case .assistChanged(let mode):
            switch mode {
            case .none: "左手辅助已结束"
            case .precision: "左掌精准稳定"
            case .cursorLock: "左手锁定光标"
            case .doubleClick: "左手双击修饰"
            case .scroll: "左手掌控滚动"
            case .drag: "左手拖拽辅助"
            }
        }
    }

    private func handleCalibration(_ event: CalibrationEvent) {
        switch event {
        case .progress(let stage, let progress):
            calibrationProgress = progress
            calibrationMessage = stage == 1
                ? "将食指稳定放在舒适活动范围的左上角"
                : "现在将食指稳定放在右下角"
        case .cornerCaptured:
            calibrationProgress = 0
            calibrationMessage = "左上角已记录，现在保持在右下角"
        case .completed(let minX, let maxX, let minY, let maxY):
            guard let store = settingsStore else { return }
            var settings = store.settings
            settings.calibrationMinX = minX
            settings.calibrationMaxX = maxX
            settings.calibrationMinY = minY
            settings.calibrationMaxY = maxY
            do {
                try store.save(settings)
                try rebuildCore(using: settings)
                calibration = nil
                calibrationProgress = 1
                calibrationMessage = "校准完成"
                runState = .previewOnly
            } catch {
                calibration = nil
                runState = .error(error.localizedDescription)
            }
        case .invalidRange:
            calibration = nil
            calibrationProgress = 0
            calibrationMessage = "校准范围过小或方向不正确，旧设置保持不变"
            runState = .previewOnly
        }
    }

    private func handleGazeCalibration(_ event: GazeCalibrationEvent) {
        switch event {
        case .progress(let stage, let progress):
            gazeCalibrationStage = stage
            gazeCalibrationProgress = progress
            gazeStatus = "注视第 \(stage + 1) 个圆点"
            gazeOverlay.update(stage: stage, progress: progress)
        case .targetCaptured(let stage):
            let next = min(stage + 1, 8)
            gazeCalibrationStage = next
            gazeCalibrationProgress = 0
            gazeStatus = "第 \(stage + 1) 点已记录，注视下一点"
            gazeOverlay.update(stage: next, progress: 0)
        case .completed(let profile):
            guard let store = settingsStore else { return }
            do {
                store.saveGazeAssist(enabled: true, profile: profile)
                gazeAssistEnabled = true
                gazeCalibrated = true
                try rebuildGazeMapper(using: store.settings)
                gazeCalibration = nil
                gazeOverlay.close()
                gazeCalibrationProgress = 1
                gazeStatus = "校准完成，等待清晰视线"
                estimator.setGazeEnabled(true)
                cameraAuthorized = PermissionManager.cameraStatus == .authorized
                refreshAccessibility()
                let shouldResume = gazeCalibrationResumePolicy.finish(
                    .completed,
                    cameraAuthorized: cameraAuthorized,
                    accessibilityGranted: accessibilityGranted
                )
                if shouldResume {
                    beginControlActivation()
                } else {
                    runState = .previewOnly
                }
            } catch {
                cancelGazeCalibration()
                runState = .error(error.localizedDescription)
            }
        case .invalidProfile:
            _ = gazeCalibrationResumePolicy.finish(
                .failed,
                cameraAuthorized: cameraAuthorized,
                accessibilityGranted: accessibilityGranted
            )
            gazeCalibration = nil
            gazeOverlay.close()
            gazeCalibrationProgress = 0
            estimator.setGazeEnabled(gazeAssistEnabled)
            gazeStatus = "校准范围不足，请保持头部不动后重试"
            runState = .previewOnly
        }
    }
}
