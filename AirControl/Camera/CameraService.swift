@preconcurrency import AVFoundation

enum CameraServiceError: Error, LocalizedError {
    case noCamera
    case cannotAddInput
    case cannotAddOutput
    case interrupted
    case disconnected

    var errorDescription: String? {
        switch self {
        case .noCamera: "没有找到可用摄像头"
        case .cannotAddInput: "无法连接所选摄像头"
        case .cannotAddOutput: "无法读取摄像头画面"
        case .interrupted: "摄像头采集已被系统中断"
        case .disconnected: "正在使用的摄像头已断开"
        }
    }
}

final class CameraService: NSObject, AVCaptureVideoDataOutputSampleBufferDelegate, @unchecked Sendable {
    let session = AVCaptureSession()
    var onFrame: (@Sendable (CMSampleBuffer) -> Void)?
    var onError: (@Sendable (Error) -> Void)?

    private let sessionQueue = DispatchQueue(label: "com.liuchong.AirControl.camera.session")
    private let frameQueue = DispatchQueue(label: "com.liuchong.AirControl.camera.frames")
    private let output = AVCaptureVideoDataOutput()
    private var notificationObservers: [NSObjectProtocol] = []

    override init() {
        super.init()
        notificationObservers.append(NotificationCenter.default.addObserver(
            forName: .AVCaptureSessionRuntimeError,
            object: session,
            queue: nil
        ) { [weak self] notification in
            let error = notification.userInfo?[AVCaptureSessionErrorKey] as? Error
                ?? CameraServiceError.interrupted
            self?.onError?(error)
        })
        notificationObservers.append(NotificationCenter.default.addObserver(
            forName: .AVCaptureSessionWasInterrupted,
            object: session,
            queue: nil
        ) { [weak self] _ in
            self?.onError?(CameraServiceError.interrupted)
        })
        notificationObservers.append(NotificationCenter.default.addObserver(
            forName: AVCaptureDevice.wasDisconnectedNotification,
            object: nil,
            queue: nil
        ) { [weak self] notification in
            guard let self, let device = notification.object as? AVCaptureDevice else { return }
            let usesDevice = self.session.inputs
                .compactMap { $0 as? AVCaptureDeviceInput }
                .contains { $0.device.uniqueID == device.uniqueID }
            if usesDevice { self.onError?(CameraServiceError.disconnected) }
        })
    }

    deinit {
        for observer in notificationObservers {
            NotificationCenter.default.removeObserver(observer)
        }
    }

    static var availableDevices: [AVCaptureDevice] {
        AVCaptureDevice.DiscoverySession(
            deviceTypes: [.builtInWideAngleCamera, .external, .continuityCamera],
            mediaType: .video,
            position: .unspecified
        ).devices
    }

    func configure(deviceID: String?) async throws {
        try await withCheckedThrowingContinuation { continuation in
            sessionQueue.async { [self] in
                do {
                    if session.isRunning { session.stopRunning() }
                    session.beginConfiguration()
                    defer { session.commitConfiguration() }
                    for input in session.inputs { session.removeInput(input) }
                    for existingOutput in session.outputs { session.removeOutput(existingOutput) }

                    let device = Self.availableDevices.first(where: { $0.uniqueID == deviceID })
                        ?? AVCaptureDevice.systemPreferredCamera
                        ?? Self.availableDevices.first
                    guard let device else { throw CameraServiceError.noCamera }
                    let input = try AVCaptureDeviceInput(device: device)
                    guard session.canAddInput(input) else { throw CameraServiceError.cannotAddInput }
                    session.addInput(input)

                    output.alwaysDiscardsLateVideoFrames = true
                    output.videoSettings = [
                        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA
                    ]
                    output.setSampleBufferDelegate(self, queue: frameQueue)
                    guard session.canAddOutput(output) else { throw CameraServiceError.cannotAddOutput }
                    session.addOutput(output)
                    session.sessionPreset = .medium
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    func start() {
        sessionQueue.async { [session] in
            if !session.isRunning { session.startRunning() }
        }
    }

    func stop() {
        sessionQueue.async { [session] in
            if session.isRunning { session.stopRunning() }
        }
    }

    func captureOutput(
        _ output: AVCaptureOutput,
        didOutput sampleBuffer: CMSampleBuffer,
        from connection: AVCaptureConnection
    ) {
        onFrame?(sampleBuffer)
    }
}
