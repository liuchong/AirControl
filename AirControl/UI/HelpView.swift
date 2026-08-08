import SwiftUI

struct HelpView: View {
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                HStack {
                    Text("AirControl 使用帮助").font(.title2.bold())
                    Spacer()
                    Button("完成") { dismiss() }
                }
                Grid(alignment: .leading, horizontalSpacing: 24, verticalSpacing: 12) {
                    row("伸出食指（手掌可旋转）", "移动光标")
                    row("拇指与食指短捏", "锁定当前位置后左键单击")
                    row("拇指与食指持续捏合", "原位置按下，移动拖拽，松开结束")
                    row("拇指与中指短捏", "右键单击")
                    row("食指和中指同时伸出", "上下移动进行滚动")
                    row("右手稳定张掌后快速握拳", "抓住光标下的窗口；移动掌心移动窗口，重新张掌释放")
                    row("左手张开手掌", "精准模式：右手掌心慢速、稳定地移动光标")
                    row("左手只伸食指", "锁定光标；右手仍可左键或右键单击")
                    row("左手竖起拇指", "右手一次主短捏执行真正的左键双击")
                    row("左手伸出 V 形", "移动右手掌心进行滚动，不要求右手保持 V 形")
                    row("左手伸出三指", "按住左键，移动右手掌心拖拽；收手即释放")
                    row("左手握拳保持", "暂停或恢复；右手握拳不会暂停")
                    row("视线辅助（需九点校准）", "双手离开画面时粗略移动光标；手进入后从当前位置精细控制")
                    row("屏幕辅助光源", "用目标显示器边缘柔光改善面部照明；可调整亮度和冷暖色")
                    row("⌃⌥⌘A", "全局紧急停止")
                }
                Divider()
                Text("权限与隐私").font(.headline)
                Text("摄像头画面只在内存中由 Apple Vision 处理，不保存、不上传，也不会启用麦克风。辅助功能权限只用于投递鼠标事件。")
                    .foregroundStyle(.secondary)
                Text("信号丢失、权限撤销、摄像头中断、停用或退出时，AirControl 会释放正在按住的鼠标键。")
                    .foregroundStyle(.secondary)
                Text("连续识别失败时会进入恢复中并自动重试；恢复期间摄像头继续采集，不需要反复点击停止和开始。")
                    .foregroundStyle(.secondary)
                Text("左手辅助姿势需稳定约 0.12 秒；左手短暂被遮挡 0.20 秒内会保持当前辅助，超过后会安全退出并释放拖拽。")
                    .foregroundStyle(.secondary)
                Text("开始捏合后光标会短暂锁定，连续确认手指张开后才完成点击；关节点一闪而过的遮挡不会立即取消捏合。")
                    .foregroundStyle(.secondary)
                Text("抓窗必须完整做出“稳定张掌 → 0.65 秒内收拢 → 连续两帧握拳”；单独握拳不会触发。不可移动的全屏窗口或系统表面会被安全忽略。")
                    .foregroundStyle(.secondary)
                Text("抓窗仍只使用辅助功能权限来改变窗口位置，不读取窗口内容，也不需要屏幕录制权限。")
                    .foregroundStyle(.secondary)
                Text("视线辅助只做粗定位，不会因眨眼产生点击。闭眼、丢脸或识别不可靠时光标保持原位；校准只保存数值系数，不保存人脸或眼部图像。")
                    .foregroundStyle(.secondary)
                Text("控制中开始九点校准时会先安全停止；校准成功且权限仍有效后自动恢复控制。取消、失败或权限失效不会误启动。")
                    .foregroundStyle(.secondary)
                Text("屏幕辅助光源不读取屏幕、不申请新权限，也不能关闭 macOS 的摄像头绿色隐私指示灯。")
                    .foregroundStyle(.secondary)
                Text("摄像头和目标显示器选择会在下次启动时恢复；设备不再存在时会安全回退到系统首选设备。")
                    .foregroundStyle(.secondary)
            }
            .padding(28)
        }
        .frame(width: 620, height: 720)
    }

    private func row(_ gesture: String, _ action: String) -> some View {
        GridRow {
            Text(gesture).fontWeight(.medium)
            Text(action).foregroundStyle(.secondary)
        }
    }
}
