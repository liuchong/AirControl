# AirControl 本地稳定签名

## 目标

AirControl 在同一台开发用 Mac 上反复构建和覆盖安装时，保持同一个 macOS 代码身份，使摄像头和辅助功能授权能够识别后续版本仍是同一个应用。

## 当前问题

当前构建关闭 Xcode 签名，安装时再使用临时签名。临时签名的指定要求只有本次二进制的代码哈希；代码变化后哈希随之变化，macOS 无法把新版本识别为已经授权的 AirControl。

`IntentCapTray` 的现有安装包使用固定 Apple Development 证书和固定标识，并非临时签名。`Lark Agent` 的状态栏应用虽然是临时签名，但不请求摄像头、屏幕录制或辅助功能，不能作为此权限场景的对照实现。

## 确认方案

- 在当前用户的登录钥匙串中创建名为 `AirControl Local Code Signing` 的本地自签名代码签名身份。
- 私钥只保存在钥匙串，不写入仓库、构建目录、日志或测试夹具。
- 应用固定使用标识 `com.liuchong.AirControl`。
- 本地安装只接受这个签名身份；身份缺失或签名验证失败时立即停止，不回退到临时签名。
- 自动化测试仍以关闭签名的方式构建，避免普通单元测试依赖个人钥匙串；只有本地安装和专门的签名集成验证访问签名身份。
- 首次从临时签名迁移到固定签名时，macOS 可能要求重新授权一次。此后只有删除证书、更换签名身份或更换应用标识才应再次迁移授权。

## 状态与决策规则

签名流程只有以下结果：

- `identity-ready`：钥匙串中存在可用于签名的固定身份。
- `signed`：应用已签名，严格验证通过，指定要求不是仅绑定代码哈希。
- `blocked`：身份缺失、应用标识不符、签名失败或验证失败；禁止安装。

安装流程必须先构建，再签名和验证，最后替换 `/Applications/AirControl.app`。旧应用移动到可恢复的临时备份目录；安装失败时恢复旧应用。

## BDD 验收场景

### 相同身份跨构建稳定

Given 登录钥匙串存在 `AirControl Local Code Signing`

When 使用不同源码内容构建两个 `com.liuchong.AirControl` 应用并分别执行本地签名

Then 两个应用都通过严格签名验证，指定要求完全相同，且指定要求不是只包含代码哈希

### 缺少身份时安全失败

Given 请求使用一个不存在的签名身份

When 执行签名检查或本地安装

Then 命令返回失败并说明缺少身份，不修改已安装应用，也不改用临时签名

### 应用标识不匹配时安全失败

Given 待签名应用的 `CFBundleIdentifier` 不是 `com.liuchong.AirControl`

When 执行本地签名

Then 命令返回失败且不签名该应用

### 首次迁移和后续覆盖安装

Given 当前安装版本仍是临时签名

When 首次安装固定签名版本并完成系统授权

Then AirControl 能进入控制状态

And 如果辅助功能列表中的旧行仍绑定临时签名，则移除旧行并通过“添加”选择当前 `/Applications/AirControl.app`

When 再次用同一身份构建并覆盖安装

Then 新旧版本指定要求相同，AirControl 重启后辅助功能仍显示已授权并能进入控制状态

## 测试落点

- `integration_test/stable_local_signing.sh`：覆盖身份缺失、应用标识不匹配、两份不同二进制的指定要求一致性。
- `scripts/verify.sh`：只做不依赖个人钥匙串的静态安装流程检查，保留全仓自动化可移植性。
- 本机端到端验证：首次迁移时清除旧临时签名记录，再连续执行固定签名覆盖安装，比较指定要求并在重启后检查 AirControl 权限状态。

## 文档和入口同步

- `README.md` 说明首次创建本地身份和日常安装入口。
- `docs/testing.md` 记录签名集成验证与权限迁移验证。
- `docs/specs/air-control.md` 记录长期安装身份规则。
- AirControl 没有命令行产品入口，因此 `/help` 和产品命令 `--help` 不适用；构建入口的帮助由 Makefile 目标和脚本错误信息承担。

## 非目标

- 不配置 Apple 账号、Apple Development 或 Developer ID。
- 不解决 App Store、公开下载、Gatekeeper 公证或跨机器分发。
- 不把 macOS 签名逻辑放进 Rust 共享核心。
- 不改变手势、权限种类或运行时控制逻辑。
