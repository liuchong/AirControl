import CoreGraphics
import Foundation
import AirControlCoreFFI

enum CoreEngineError: Error, LocalizedError {
    case incompatibleABI(UInt32)
    case rustStatus(UInt32)
    case invalidCommand(UInt32)

    var errorDescription: String? {
        switch self {
        case .incompatibleABI(let version): "不支持的 Rust 核心接口版本：\(version)"
        case .rustStatus(let status): "Rust 核心返回错误状态：\(status)"
        case .invalidCommand(let kind): "Rust 核心返回未知命令：\(kind)"
        }
    }
}

struct StandardJoint: Equatable, Sendable {
    enum Kind: UInt32, CaseIterable, Sendable {
        case wrist = 0
        case thumbTip = 1
        case indexMCP = 2
        case indexPIP = 3
        case indexTip = 4
        case middleMCP = 5
        case middlePIP = 6
        case middleTip = 7
        case ringMCP = 8
        case ringPIP = 9
        case ringTip = 10
        case littleMCP = 11
        case littlePIP = 12
        case littleTip = 13
        case thumbMCP = 14
        case thumbIP = 15
    }

    let kind: Kind
    let x: Double
    let y: Double
    let confidence: Double
}

enum StandardHandedness: UInt32, Equatable, Sendable {
    case unknown = 0
    case left = 1
    case right = 2
}

struct StandardHand: Equatable, Sendable {
    let handedness: StandardHandedness
    let joints: [StandardJoint]
}

enum CoreButton: UInt32, Equatable, Sendable {
    case left = 1
    case right = 2
}

enum CoreCommand: Equatable, Sendable {
    case move(x: Double, y: Double)
    case click(button: CoreButton, x: Double, y: Double, count: UInt32)
    case mouseDown(button: CoreButton, x: Double, y: Double)
    case mouseUp(button: CoreButton, x: Double, y: Double)
    case scroll(deltaY: Double)
    case pauseChanged(Bool)
    case assistChanged(CoreAssistMode)
    case windowGrabBegin(x: Double, y: Double)
    case windowMove(x: Double, y: Double)
    case windowGrabEnd
    case showAppOverview

    var kindCode: UInt32 {
        switch self {
        case .move: 1
        case .mouseDown: 2
        case .mouseUp: 3
        case .scroll: 4
        case .pauseChanged: 5
        case .click: 6
        case .assistChanged: 7
        case .windowGrabBegin: 8
        case .windowMove: 9
        case .windowGrabEnd: 10
        case .showAppOverview: 11
        }
    }
}

enum CoreAssistMode: UInt32, Equatable, Sendable {
    case none = 0
    case precision = 1
    case cursorLock = 2
    case doubleClick = 3
    case scroll = 4
    case drag = 5
}

final class CoreEngine {
    private var handle: OpaquePointer?

    init(settings: CoreSettings) throws {
        let version = ac_abi_version()
        guard version == 5 else { throw CoreEngineError.incompatibleABI(version) }
        var ffiSettings = settings.ffiValue
        var newHandle: OpaquePointer?
        let status = ac_engine_create(&ffiSettings, &newHandle)
        guard status == AC_STATUS_OK, let newHandle else {
            throw CoreEngineError.rustStatus(status)
        }
        handle = newHandle
    }

    deinit {
        ac_engine_destroy(handle)
    }

    func process(timestamp: Double, joints: [StandardJoint]) throws -> [CoreCommand] {
        try process(timestamp: timestamp, hands: [StandardHand(handedness: .right, joints: joints)])
    }

    func process(timestamp: Double, hands: [StandardHand]) throws -> [CoreCommand] {
        let ffiJoints = hands.flatMap { hand in hand.joints.map { joint -> ACJoint in
            var value = ACJoint()
            value.kind = joint.kind.rawValue
            value.handedness = hand.handedness.rawValue
            value.x = joint.x
            value.y = joint.y
            value.confidence = joint.confidence
            return value
        } }
        var output = [ACCommand](repeating: ACCommand(), count: 16)
        var outputCount = 0
        let status = ffiJoints.withUnsafeBufferPointer { jointBuffer in
            output.withUnsafeMutableBufferPointer { commandBuffer in
                ac_engine_process(
                    handle,
                    timestamp,
                    jointBuffer.baseAddress,
                    jointBuffer.count,
                    commandBuffer.baseAddress,
                    commandBuffer.count,
                    &outputCount
                )
            }
        }
        guard status == AC_STATUS_OK else { throw CoreEngineError.rustStatus(status) }
        return try output.prefix(outputCount).map(Self.decode)
    }

    func stop() throws -> [CoreCommand] {
        var output = [ACCommand](repeating: ACCommand(), count: 8)
        var outputCount = 0
        let status = output.withUnsafeMutableBufferPointer { buffer in
            ac_engine_stop(handle, buffer.baseAddress, buffer.count, &outputCount)
        }
        guard status == AC_STATUS_OK else { throw CoreEngineError.rustStatus(status) }
        return try output.prefix(outputCount).map(Self.decode)
    }

    func rebasePointer(to point: CGPoint) throws {
        let status = ac_engine_rebase_pointer(handle, point.x, point.y)
        guard status == AC_STATUS_OK else { throw CoreEngineError.rustStatus(status) }
    }

    func clearPointerRebase() throws {
        let status = ac_engine_clear_pointer_rebase(handle)
        guard status == AC_STATUS_OK else { throw CoreEngineError.rustStatus(status) }
    }

    private static func decode(_ command: ACCommand) throws -> CoreCommand {
        switch command.kind {
        case 1:
            return .move(x: command.x, y: command.y)
        case 6:
            guard let button = CoreButton(rawValue: command.button), command.flags == 1 || command.flags == 2 else {
                throw CoreEngineError.invalidCommand(command.kind)
            }
            return .click(button: button, x: command.x, y: command.y, count: command.flags)
        case 2, 3:
            guard let button = CoreButton(rawValue: command.button) else {
                throw CoreEngineError.invalidCommand(command.kind)
            }
            if command.kind == 2 {
                return .mouseDown(button: button, x: command.x, y: command.y)
            }
            return .mouseUp(button: button, x: command.x, y: command.y)
        case 4:
            return .scroll(deltaY: command.value)
        case 5:
            return .pauseChanged(command.flags != 0)
        case 7:
            guard let mode = CoreAssistMode(rawValue: command.flags) else {
                throw CoreEngineError.invalidCommand(command.kind)
            }
            return .assistChanged(mode)
        case 8:
            return .windowGrabBegin(x: command.x, y: command.y)
        case 9:
            return .windowMove(x: command.x, y: command.y)
        case 10:
            return .windowGrabEnd
        case 11:
            return .showAppOverview
        default:
            throw CoreEngineError.invalidCommand(command.kind)
        }
    }
}
