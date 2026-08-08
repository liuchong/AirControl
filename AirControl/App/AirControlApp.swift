import SwiftUI

@main
struct AirControlApp: App {
    @StateObject private var model = AppModel()

    var body: some Scene {
        WindowGroup {
            MainView().environmentObject(model)
        }
        .commands {
            CommandGroup(replacing: .help) {
                Button("AirControl 使用帮助") { model.showingHelp = true }
                    .keyboardShortcut("?", modifiers: .command)
            }
        }

        MenuBarExtra {
            Text(model.runState.title)
            Divider()
            Button(menuBarActionTitle) { model.toggleControl() }
            Button("显示主窗口") { NSApplication.shared.activate(ignoringOtherApps: true) }
            Button("退出 AirControl") {
                model.safeStop()
                NSApplication.shared.terminate(nil)
            }
        } label: {
            AirControlMenuBarIcon(runState: model.runState)
        }
    }

    private var menuBarActionTitle: String {
        switch model.runState {
        case .controlling, .paused, .recovering: "停止控制"
        default: "开启控制"
        }
    }
}
