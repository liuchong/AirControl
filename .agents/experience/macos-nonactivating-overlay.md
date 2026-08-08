# macOS 非激活覆盖层

- 仅用于视觉提示或补光的全屏覆盖层必须同时设置 `ignoresMouseEvents = true`，并让面板的 `canBecomeKey`、`canBecomeMain` 返回 `false`，否则即使背景透明也可能抢走用户输入。
- 多显示器覆盖层应以持久化显示器 ID 重新解析 `NSScreen`，显示器热插拔或用户切换目标时销毁旧面板再创建，不能只修改内容视图大小。
- 需要出现在全屏空间时使用 `canJoinAllSpaces` 与 `fullScreenAuxiliary`；同层级存在校准覆盖层时，只把透明补光边缘重新前置，避免遮挡中央校准目标。
- 覆盖层只绘制应用自己的透明窗口，不等于屏幕捕获，不应因此申请屏幕录制权限。
