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

        let envPath = "/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin"
        var env = ProcessInfo.processInfo.environment
        env["PATH"] = envPath
        let envStrings = env.map { "\($0.key)=\($0.value)" }

        let opencodeBin = findBinary("opencode")
        let shellCommand: String
        if let bin = opencodeBin {
            shellCommand = """
            export PATH="\(envPath)"
            cd '\(workspaceDir)' || { echo "[ERROR] cd to workspace failed"; exec /bin/zsh; }
            echo "[ClawParty] Starting opencode in \(workspaceDir)..."
            exec '\(bin)'
            """
        } else {
            shellCommand = """
            export PATH="\(envPath)"
            cd '\(workspaceDir)' || { echo "[ERROR] cd to workspace failed"; }
            echo "[ERROR] opencode not found in PATH"
            echo "PATH=\(envPath)"
            exec /bin/zsh
            """
        }
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
        try? FileManager.default.createDirectory(atPath: workspace, withIntermediateDirectories: true)
        return workspace
    }

    private func findBinary(_ name: String) -> String? {
        let envPath = "/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin"
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/bin/sh")
        task.environment = ["PATH": envPath]
        task.arguments = ["-c", "command -v \(name)"]
        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()
        do { try task.run(); task.waitUntilExit() } catch { return nil }
        let output = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return (output?.isEmpty == false) ? output : nil
    }
}
