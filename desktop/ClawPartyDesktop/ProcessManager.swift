import Foundation
import Combine

struct LogEntry: Identifiable {
    let id = UUID()
    let timestamp: Date
    let level: LogLevel
    let message: String

    var formattedTime: String {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss"
        return formatter.string(from: timestamp)
    }
}

enum LogLevel: String {
    case info = "INFO"
    case error = "ERROR"
    case warn = "WARN"
    case debug = "DEBUG"
}

class ProcessManager: ObservableObject {
    static let shared = ProcessManager()

    @Published private var clawPartyProcess: Process?
    @Published var logs: [LogEntry] = []
    @Published var isClawPartyRunning: Bool = false

    private let clawPartyBinary: String
    private let maxLogCount = 1000
    private var checkTimer: Timer?

    private init() {
        let repoRoot = "/Users/caishu/github/clawparty"
        self.clawPartyBinary = "\(repoRoot)/bin/clawparty"

        // 启动时先检测是否已有运行中的 clawparty
        checkExistingProcess()

        // 定期检查进程状态
        startStatusCheckTimer()
    }

    // MARK: - 进程/端口检测

    private func startStatusCheckTimer() {
        checkTimer = Timer.scheduledTimer(withTimeInterval: 3.0, repeats: true) { [weak self] _ in
            self?.checkExistingProcess()
        }
    }

    private func checkExistingProcess() {
        let isRunning = isClawPartyProcessRunning() || isPortInUse(port: 7778)

        DispatchQueue.main.async { [weak self] in
            if self?.isClawPartyRunning != isRunning {
                self?.isClawPartyRunning = isRunning
                if isRunning {
                    self?.addLog(level: .info, message: "检测到 ClawParty 正在运行")
                } else {
                    self?.addLog(level: .warn, message: "ClawParty 未运行")
                }
            }
        }
    }

    /// 检查是否有 clawparty 进程在运行（排除本应用自身）
    private func isClawPartyProcessRunning() -> Bool {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/bin/sh")
        // 使用 pgrep -f 匹配完整路径中的 bin/clawparty，避免匹配到 ClawPartyDesktop
        task.arguments = ["-c", "pgrep -f 'bin/clawparty' > /dev/null 2>&1 && echo 'found'"]

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()

        do {
            try task.run()
            task.waitUntilExit()

            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            guard let output = String(data: data, encoding: .utf8) else { return false }

            return output.trimmingCharacters(in: .whitespacesAndNewlines) == "found"
        } catch {
            return false
        }
    }

    /// 检查端口是否被占用
    private func isPortInUse(port: Int) -> Bool {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/bin/sh")
        task.arguments = ["-c", "lsof -i :\(port) | grep LISTEN"]

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()

        do {
            try task.run()
            task.waitUntilExit()

            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            guard let output = String(data: data, encoding: .utf8) else { return false }

            return !output.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        } catch {
            return false
        }
    }

    // MARK: - 日志管理

    func addLog(level: LogLevel, message: String) {
        DispatchQueue.main.async { [weak self] in
            guard let self = self else { return }
            let entry = LogEntry(timestamp: Date(), level: level, message: message)
            self.logs.append(entry)
            if self.logs.count > self.maxLogCount {
                self.logs.removeFirst(self.logs.count - self.maxLogCount)
            }
        }
    }

    func clearLogs() {
        DispatchQueue.main.async { [weak self] in
            self?.logs.removeAll()
        }
    }

    // MARK: - 启动/停止

    func startClawParty(completion: @escaping (Bool, String?) -> Void) {
        // 先检查是否已经有运行中的实例
        if isClawPartyRunning {
            addLog(level: .warn, message: "ClawParty 已经在运行中（由其他进程启动）")
            completion(false, "ClawParty 已经在运行中")
            return
        }

        guard FileManager.default.fileExists(atPath: clawPartyBinary) else {
            addLog(level: .error, message: "找不到 ClawParty 可执行文件: \(clawPartyBinary)")
            completion(false, "找不到 ClawParty 可执行文件: \(clawPartyBinary)")
            return
        }

        addLog(level: .info, message: "正在启动 ClawParty...")

        let process = Process()
        process.executableURL = URL(fileURLWithPath: clawPartyBinary)
        process.currentDirectoryURL = URL(fileURLWithPath: "/Users/caishu/github/clawparty")
        // -s = service mode (非 TUI，适合后台运行)
        process.arguments = ["-s"]

        var environment = ProcessInfo.processInfo.environment
        environment["PATH"] = "/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin"
        process.environment = environment

        let outputPipe = Pipe()
        let errorPipe = Pipe()
        process.standardOutput = outputPipe
        process.standardError = errorPipe

        outputPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            if let line = String(data: handle.availableData, encoding: .utf8), !line.isEmpty {
                let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                if !trimmed.isEmpty {
                    self?.addLog(level: .info, message: trimmed)
                }
            }
        }

        errorPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            if let line = String(data: handle.availableData, encoding: .utf8), !line.isEmpty {
                let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                if !trimmed.isEmpty {
                    self?.addLog(level: .error, message: trimmed)
                }
            }
        }

        process.terminationHandler = { [weak self] proc in
            self?.addLog(level: .warn, message: "ClawParty 进程已退出，状态码: \(proc.terminationStatus)")
            if self?.clawPartyProcess === proc {
                self?.clawPartyProcess = nil
            }
            // 更新状态
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                self?.checkExistingProcess()
            }
        }

        do {
            try process.run()
            clawPartyProcess = process
            isClawPartyRunning = true
            addLog(level: .info, message: "ClawParty 启动成功，PID: \(process.processIdentifier)")
            completion(true, nil)
        } catch {
            addLog(level: .error, message: "启动失败: \(error.localizedDescription)")
            completion(false, "启动失败: \(error.localizedDescription)")
        }
    }

    func stopClawParty() {
        // 先检查是否由其他进程启动
        if clawPartyProcess == nil && isClawPartyRunning {
            addLog(level: .warn, message: "ClawParty 由其他进程启动，尝试查找并终止...")
            killExternalClawParty()
            return
        }

        guard let process = clawPartyProcess else {
            addLog(level: .warn, message: "ClawParty 未运行")
            return
        }

        addLog(level: .info, message: "正在停止 ClawParty (PID: \(process.processIdentifier))...")
        process.terminate()

        DispatchQueue.global().asyncAfter(deadline: .now() + 3.0) { [weak self] in
            if process.isRunning {
                self?.addLog(level: .warn, message: "强制终止 ClawParty 进程")
                kill(process.processIdentifier, SIGKILL)
            }
            self?.clawPartyProcess = nil
            self?.isClawPartyRunning = false
            self?.addLog(level: .info, message: "ClawParty 已停止")
        }
    }

    /// 终止外部启动的 clawparty 进程（排除本应用自身）
    private func killExternalClawParty() {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/bin/sh")
        // 使用 pgrep -f 匹配完整路径，避免杀死 ClawPartyDesktop
        task.arguments = ["-c", "pgrep -f 'bin/clawparty' | xargs kill -TERM 2>/dev/null"]

        do {
            try task.run()
            task.waitUntilExit()
            addLog(level: .info, message: "已发送终止信号到外部 ClawParty 进程")

            // 3秒后检查是否还在运行，如果还在则强制 kill
            DispatchQueue.global().asyncAfter(deadline: .now() + 3.0) { [weak self] in
                if self?.isClawPartyProcessRunning() == true {
                    let forceKill = Process()
                    forceKill.executableURL = URL(fileURLWithPath: "/bin/sh")
                    forceKill.arguments = ["-c", "pgrep -f 'bin/clawparty' | xargs kill -KILL 2>/dev/null"]
                    try? forceKill.run()
                    forceKill.waitUntilExit()
                    self?.addLog(level: .warn, message: "已强制终止外部 ClawParty 进程")
                }
                self?.checkExistingProcess()
            }
        } catch {
            addLog(level: .error, message: "终止外部进程失败: \(error.localizedDescription)")
        }
    }
}
