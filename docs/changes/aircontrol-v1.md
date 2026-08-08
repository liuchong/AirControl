# AirControl v1 变更设计

## 已确认目标

从空仓库实现原生 macOS 摄像头手势控制应用。核心结果是通过单手完成光标移动、左右键、拖拽、滚动和安全暂停。

## 结构设计

- `crates/aircontrol-core`：与平台无关的 Rust 数据模型、坐标映射、平滑、设置校验、校准和手势状态机；这是唯一业务实现。
- `include/aircontrol_core.h`：Rust 与各平台外壳共同遵守的 C 兼容接口。
- `Camera`：摄像头枚举、权限和只保留最新帧的捕获会话。
- `Vision`：将帧转换为标准 `HandPoseFrame`。
- `Core`：Swift 跨语言适配器，只把标准关键点传给 Rust 并解码抽象命令，不复制业务规则。
- `Input`：将 Rust 返回的抽象鼠标命令投递到 Core Graphics；测试使用记录器替代。
- `App/UI`：组合服务、状态机、菜单栏、窗口、设置和帮助。
- `Tests`：纯逻辑测试；`integration_test`：从关键点 fixture 到抽象鼠标命令的真实编排测试。

## 硬规则与门禁原文

> 任何方案要等用户确定方可执行

> 每个任务必须有明确且有界的范围、核心结果和停止条件。

> 所有行为变更必须遵守 `workspace rules/bdd-tdd-development-flow.md`，先定义行为和 BDD 验收场景，检查与既有行为是否冲突，再用 TDD 写验证代码后实现。

> 保证每一个逻辑开发、逻辑修改的集成测试覆盖率是最高优先级硬规则。

> 只要行为可测试，必须先把 BDD 场景转成失败测试或可执行验证，再改生产代码。

## TDD 落点

- Rust `engine_tests`：点击、拖拽、滚动、暂停、丢手释放和互斥优先级。
- Rust `geometry_tests`：镜像、纵轴、多显示器边界和校准校验。
- Rust `settings_tests`：默认值、版本和越界拒绝。
- Rust `ffi_tests`：空指针、容量、ABI 版本、停止释放和内存所有权边界。
- Swift `AirControlIntegrationTests`：fixture 序列经真实 C 兼容接口驱动 Rust 引擎，再检查 Swift 解码后的鼠标命令。
- Rust `ffi_tests` 回归门禁：缓冲区不足不得消耗点击/释放命令；校准最低置信度必须由 Rust 判定。
- Swift `VisionFailureTrackerTests`：每次 Vision 错误都要求转交未检测到手帧，第 5 次才额外终止处理链路。
- Swift `SettingsStoreTests`：摄像头和显示器设备标识可以持久化、清除并恢复。

## 配置、安装和帮助影响

- 固定 bundle ID `com.liuchong.AirControl`，最低 macOS 14，Swift 6；Rust 使用稳定工具链与标准库，不添加运行时第三方依赖。
- Xcode 构建脚本按当前架构构建 Rust 静态库；Cargo 测试可独立运行。后续 Windows/Linux 外壳复用 crate 和 C 兼容头文件。
- Debug 构建不写入 `/Applications`；用户可从 Xcode 运行或打开构建产物。
- 用户帮助由 README、Help 菜单与权限卡片提供；没有 CLI 帮助入口。
- 第一版不提交、不推送、不安装，除非用户另行确认。
