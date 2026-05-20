import SwiftUI

@main
struct ClawPartyDesktopApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        Settings {
            EmptyView()
        }
    }
}

class AppDelegate: NSObject, NSApplicationDelegate {
    var statusBarController: StatusBarController?
    var mainWindow: NSWindow?
    var llmConfigWindow: NSWindow?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        statusBarController = StatusBarController(appDelegate: self)
        showMainWindow()
    }

    func applicationWillTerminate(_ notification: Notification) {
        ProcessManager.shared.stopClawParty()
    }

    func showMainWindow() {
        if mainWindow == nil {
            let window = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 700, height: 500),
                styleMask: [.titled, .closable, .miniaturizable, .resizable],
                backing: .buffered,
                defer: false
            )
            window.title = "ClawParty Desktop"
            window.contentView = NSHostingView(rootView: MainPanelView())
            window.center()
            window.isReleasedWhenClosed = false
            mainWindow = window
        }

        mainWindow?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    func showLLMConfigWindow(for agent: AgentInfo? = nil) {
        // 如果窗口已存在，先关闭以重新创建（因为 agent 可能不同）
        if llmConfigWindow != nil {
            llmConfigWindow?.close()
            llmConfigWindow = nil
        }

        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 540, height: 520),
            styleMask: [.titled, .closable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.title = agent != nil ? "LLM 配置 - \(agent!.displayName)" : "Agent LLM 配置"
        window.contentView = NSHostingView(rootView: LLMConfigView(initialAgent: agent))
        window.center()
        window.isReleasedWhenClosed = false
        llmConfigWindow = window

        llmConfigWindow?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    func closeLLMConfigWindow() {
        llmConfigWindow?.close()
    }
}
