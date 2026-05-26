import Cocoa
import SwiftUI
import UserNotifications

class StatusBarController: NSObject {
    private var statusItem: NSStatusItem!
    private var menu: NSMenu!
    private var processManager: ProcessManager
    private var timer: Timer?
    private weak var appDelegate: AppDelegate?

    init(appDelegate: AppDelegate) {
        self.appDelegate = appDelegate
        self.processManager = ProcessManager.shared
        super.init()

        setupStatusItem()
        setupMenu()
        startStatusUpdateTimer()
    }

    private func setupStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.button?.image = createMenuBarImage(isRunning: false)
        statusItem.button?.image?.isTemplate = true
    }

    private func setupMenu() {
        menu = NSMenu()
        menu.delegate = self

        // 打开面板
        let openPanelItem = NSMenuItem(
            title: "打开面板",
            action: #selector(openPanel),
            keyEquivalent: "o"
        )
        openPanelItem.target = self
        menu.addItem(openPanelItem)

        menu.addItem(NSMenuItem.separator())

        // ClawParty 控制
        let startClawItem = NSMenuItem(
            title: "启动 ClawParty",
            action: #selector(startClawParty),
            keyEquivalent: ""
        )
        startClawItem.target = self
        menu.addItem(startClawItem)

        let stopClawItem = NSMenuItem(
            title: "停止 ClawParty",
            action: #selector(stopClawParty),
            keyEquivalent: ""
        )
        stopClawItem.target = self
        menu.addItem(stopClawItem)

        menu.addItem(NSMenuItem.separator())

        // 下载
        let downloadItem = NSMenuItem(
            title: "下载最新版本...",
            action: #selector(openDownloadPage),
            keyEquivalent: "d"
        )
        downloadItem.target = self
        menu.addItem(downloadItem)

        // 访问
        let visitItem = NSMenuItem(
            title: "访问 ClawParty",
            action: #selector(openClawPartyWeb),
            keyEquivalent: "v"
        )
        visitItem.target = self
        menu.addItem(visitItem)

        menu.addItem(NSMenuItem.separator())

        // 配置
        let configItem = NSMenuItem(
            title: "Agent LLM 配置...",
            action: #selector(showConfig),
            keyEquivalent: ","
        )
        configItem.target = self
        menu.addItem(configItem)

        menu.addItem(NSMenuItem.separator())

        // 退出
        let quitItem = NSMenuItem(
            title: "退出",
            action: #selector(quitApp),
            keyEquivalent: "q"
        )
        quitItem.target = self
        menu.addItem(quitItem)

        statusItem.menu = menu
    }

    private func startStatusUpdateTimer() {
        timer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] _ in
            self?.updateMenuState()
        }
        updateMenuState()
    }

    @objc private func updateMenuState() {
        DispatchQueue.main.async { [weak self] in
            guard let self = self else { return }

            let clawRunning = self.processManager.isClawPartyRunning

            self.statusItem.button?.image = self.createMenuBarImage(isRunning: clawRunning)

            for item in self.menu.items {
                if item.title == "启动 ClawParty" {
                    item.isEnabled = !clawRunning
                } else if item.title == "停止 ClawParty" {
                    item.isEnabled = clawRunning
                }
            }
        }
    }

    private func createMenuBarImage(isRunning: Bool) -> NSImage? {
        let size = NSSize(width: 18, height: 18)
        let image = NSImage(size: size)

        image.lockFocus()

        let context = NSGraphicsContext.current?.cgContext

        // 绘制龙虾形状（简化版）
        let bodyRect = CGRect(x: 6, y: 4, width: 6, height: 8)
        let bodyPath = CGPath(ellipseIn: bodyRect, transform: nil)
        context?.addPath(bodyPath)
        context?.setFillColor(isRunning ? CGColor(red: 0.9, green: 0.3, blue: 0.2, alpha: 1.0) : CGColor(red: 0.5, green: 0.5, blue: 0.5, alpha: 1.0))
        context?.fillPath()

        // 钳子
        let leftClaw = CGMutablePath()
        leftClaw.move(to: CGPoint(x: 5, y: 10))
        leftClaw.addQuadCurve(to: CGPoint(x: 2, y: 12), control: CGPoint(x: 3, y: 13))
        leftClaw.addQuadCurve(to: CGPoint(x: 5, y: 8), control: CGPoint(x: 4, y: 9))
        context?.addPath(leftClaw)
        context?.setStrokeColor(isRunning ? CGColor(red: 0.9, green: 0.3, blue: 0.2, alpha: 1.0) : CGColor(red: 0.5, green: 0.5, blue: 0.5, alpha: 1.0))
        context?.setLineWidth(1.5)
        context?.strokePath()

        let rightClaw = CGMutablePath()
        rightClaw.move(to: CGPoint(x: 13, y: 10))
        rightClaw.addQuadCurve(to: CGPoint(x: 16, y: 12), control: CGPoint(x: 15, y: 13))
        rightClaw.addQuadCurve(to: CGPoint(x: 13, y: 8), control: CGPoint(x: 14, y: 9))
        context?.addPath(rightClaw)
        context?.strokePath()

        // 眼睛
        let leftEye = CGRect(x: 7, y: 10, width: 1.5, height: 1.5)
        let rightEye = CGRect(x: 10, y: 10, width: 1.5, height: 1.5)
        context?.addEllipse(in: leftEye)
        context?.addEllipse(in: rightEye)
        context?.setFillColor(CGColor(red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0))
        context?.fillPath()

        image.unlockFocus()
        image.isTemplate = true
        return image
    }

    @objc private func openPanel() {
        appDelegate?.showMainWindow()
    }

    @objc private func startClawParty() {
        processManager.startClawParty { success, error in
            DispatchQueue.main.async {
                if success {
                    self.showNotification(title: "ClawParty 已启动", body: "服务正在运行")
                } else {
                    self.showNotification(title: "启动失败", body: error ?? "未知错误")
                }
                self.updateMenuState()
            }
        }
    }

    @objc private func stopClawParty() {
        processManager.stopClawParty()
        showNotification(title: "ClawParty 已停止", body: "服务已关闭")
        updateMenuState()
    }

    @objc private func showConfig() {
        appDelegate?.showLLMConfigWindow()
    }

    @objc private func openDownloadPage() {
        guard let url = URL(string: "https://github.com/clawparty-ai/clawparty/releases") else { return }
        NSWorkspace.shared.open(url)
    }

    @objc private func openClawPartyWeb() {
        guard let url = URL(string: "https://localhost") else { return }
        NSWorkspace.shared.open(url)
    }

    @objc private func quitApp() {
        processManager.stopClawParty()
        NSApp.terminate(nil)
    }

    private func showNotification(title: String, body: String) {
        let notification = UNMutableNotificationContent()
        notification.title = title
        notification.body = body
        notification.sound = .default

        let request = UNNotificationRequest(
            identifier: UUID().uuidString,
            content: notification,
            trigger: nil
        )

        UNUserNotificationCenter.current().add(request)
    }
}

extension StatusBarController: NSMenuDelegate {
    func menuWillOpen(_ menu: NSMenu) {
        updateMenuState()
    }
}
