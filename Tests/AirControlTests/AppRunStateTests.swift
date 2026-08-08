import XCTest
@testable import AirControl

final class AppRunStateTests: XCTestCase {
    func testErrorStateExposesItsDetailForTheMainWindow() {
        XCTAssertEqual(
            AppModel.RunState.error("Rust 核心返回错误状态：1").detailMessage,
            "Rust 核心返回错误状态：1"
        )
        XCTAssertNil(AppModel.RunState.controlling.detailMessage)
    }

    func testRecoveringStateExposesReasonForTheMainWindow() {
        XCTAssertEqual(
            AppModel.RunState.recovering("摄像头识别恢复中").detailMessage,
            "摄像头识别恢复中"
        )
        XCTAssertEqual(AppModel.RunState.recovering("摄像头识别恢复中").title, "恢复中")
    }
}
