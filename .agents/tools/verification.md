# AirControl 验证工具

`scripts/verify.sh` 是统一验证入口，依次执行 Rust 格式检查、静态分析、全部 Cargo 测试、Xcode 工程生成、生成后摄像头用途说明与 entitlement 检查、macOS 测试构建和 Swift 单元/集成测试。

它不会启动应用、申请系统权限或投递真实鼠标事件。真实摄像头与辅助功能权限按 `docs/testing.md` 手动验收。
