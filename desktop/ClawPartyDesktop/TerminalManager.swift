import SwiftTerm
import Foundation

/// 管理每个 Agent 对应的终端实例，支持多终端同时运行
class TerminalManager {
    static let shared = TerminalManager()

    private var terminals: [String: LocalProcessTerminalView] = [:]
    private let queue = DispatchQueue(label: "com.clawparty.terminal-manager", attributes: .concurrent)

    private init() {}

    /// 获取或创建指定 Agent 的终端视图
    func terminal(for agent: AgentInfo) -> LocalProcessTerminalView {
        let existing = queue.sync { terminals[agent.id] }
        if let existing = existing {
            return existing
        }

        let term = LocalProcessTerminalView(frame: .zero)
        let workspaceDir = workspaceDirectory(for: agent)

        // 构建环境变量，确保 PATH 包含 opencode 所在路径
        var env = ProcessInfo.processInfo.environment
        let currentPath = env["PATH"] ?? "/usr/bin:/bin"
        env["PATH"] = currentPath + ":/usr/local/bin:/opt/homebrew/bin"
        let envStrings = env.map { "\($0.key)=\($0.value)" }

        // 启动 zsh，cd 到 agent workspace，然后执行 opencode -c
        let shellCommand = "cd '\(workspaceDir)' && opencode -c"
        term.startProcess(
            executable: "/bin/zsh",
            args: ["-c", shellCommand],
            environment: envStrings
        )

        queue.async(flags: .barrier) {
            self.terminals[agent.id] = term
        }

        return term
    }

    /// 获取已存在的终端视图（如果已创建）
    func existingTerminal(for agent: AgentInfo) -> LocalProcessTerminalView? {
        queue.sync {
            terminals[agent.id]
        }
    }

    /// 终止指定 Agent 的终端进程
    func terminateTerminal(for agent: AgentInfo) {
        queue.async(flags: .barrier) {
            guard let term = self.terminals.removeValue(forKey: agent.id) else { return }
            DispatchQueue.main.async {
                term.terminate()
            }
        }
    }

    /// 终止所有终端进程（应用退出时调用）
    func terminateAll() {
        let allTerms = queue.sync { Array(terminals.values) }
        queue.async(flags: .barrier) {
            self.terminals.removeAll()
        }
        DispatchQueue.main.async {
            for term in allTerms {
                term.terminate()
            }
        }
    }

    // MARK: - Helpers

    private func workspaceDirectory(for agent: AgentInfo) -> String {
        let configDir = (agent.configPath as NSString).deletingLastPathComponent
        let workspace = "\(configDir)/workspace"
        // 确保 workspace 目录存在
        try? FileManager.default.createDirectory(atPath: workspace, withIntermediateDirectories: true)
        return workspace
    }
}
