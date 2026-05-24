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

// MARK: - 检查与修复系统类型

enum CheckStatus: Equatable {
    case pending
    case checking
    case pass(detail: String)
    case warning(detail: String)
    case fail(detail: String)

    var isProblem: Bool {
        switch self {
        case .warning, .fail: return true
        default: return false
        }
    }

    var icon: String {
        switch self {
        case .pending: return "circle"
        case .checking: return "arrow.triangle.2.circlepath"
        case .pass: return "checkmark.circle.fill"
        case .warning: return "exclamationmark.triangle.fill"
        case .fail: return "xmark.circle.fill"
        }
    }

    var label: String {
        switch self {
        case .pending: return "等待中"
        case .checking: return "检查中"
        case .pass: return "通过"
        case .warning: return "警告"
        case .fail: return "失败"
        }
    }
}

enum CheckCategory: String, CaseIterable, Identifiable {
    case executables
    case zeroAgentConfig
    case agentConfigs
    case llmConnectivity
    case workspaceFiles
    case portConflicts
    case databaseIntegrity

    var id: String { rawValue }

    var label: String {
        switch self {
        case .executables: return "可执行文件"
        case .zeroAgentConfig: return "0#Agent 配置"
        case .agentConfigs: return "Agent 配置文件"
        case .llmConnectivity: return "LLM 连通性"
        case .workspaceFiles: return "工作区文件"
        case .portConflicts: return "端口冲突"
        case .databaseIntegrity: return "数据库完整性"
        }
    }

    var icon: String {
        switch self {
        case .executables: return "terminal"
        case .zeroAgentConfig: return "star.fill"
        case .agentConfigs: return "gearshape.2"
        case .llmConnectivity: return "network"
        case .workspaceFiles: return "doc.on.doc"
        case .portConflicts: return "antenna.radiowaves.left.and.right"
        case .databaseIntegrity: return "cylinder"
        }
    }
}

struct CheckItem: Identifiable, Equatable {
    let id = UUID()
    let category: CheckCategory
    let name: String
    let detail: String
    var status: CheckStatus = .pending
    var selected: Bool = false
    var fixDescription: String = ""

    static func == (lhs: CheckItem, rhs: CheckItem) -> Bool {
        lhs.id == rhs.id
    }
}

enum CheckRepairPhase: Equatable {
    case idle
    case checking
    case report
    case plan
    case repairing
    case done(successCount: Int, failCount: Int)
}

class ProcessManager: ObservableObject {
    static let shared = ProcessManager()

    @Published private var clawPartyProcess: Process?
    @Published var logs: [LogEntry] = []
    @Published var isClawPartyRunning: Bool = false

    private var clawPartyBinary: String {
        findBinarySource("clawparty") ?? ""
    }
    private let maxLogCount = 1000
    private var checkTimer: Timer?

    @Published var checkItems: [CheckItem] = []
    @Published var checkPhase: CheckRepairPhase = .idle
    @Published var repairLog: [String] = []

    private init() {
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
        task.arguments = ["-c", "pgrep -x 'clawparty' > /dev/null 2>&1 && echo 'found'"]

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

    /// 检测是否首次运行（~/.clawparty 目录不存在）
    static func isFirstRun() -> Bool {
        let homeDir = FileManager.default.homeDirectoryForCurrentUser
        let clawpartyDir = homeDir.appendingPathComponent(".clawparty")
        return !FileManager.default.fileExists(atPath: clawpartyDir.path)
    }

    func startClawParty(adminPassword: String? = nil, apiKey: String? = nil, completion: @escaping (Bool, String?) -> Void) {
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
        process.currentDirectoryURL = Bundle.main.bundleURL.deletingLastPathComponent()
        // -s = service mode (非 TUI，适合后台运行)
        process.arguments = ["-s"]

        var environment = ProcessInfo.processInfo.environment
        environment["PATH"] = "/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin"
        // 首次运行凭据：通过环境变量传递给 clawparty，避免交互式提示
        if let password = adminPassword, !password.isEmpty {
            environment["CLAWPARTY_ADMIN_PASSWORD"] = password
        }
        if let key = apiKey, !key.isEmpty {
            environment["CLAWPARTY_API_KEY"] = key
        }
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
        task.arguments = ["-c", "pgrep -x 'clawparty' | xargs kill -TERM 2>/dev/null"]

        do {
            try task.run()
            task.waitUntilExit()
            addLog(level: .info, message: "已发送终止信号到外部 ClawParty 进程")

            // 3秒后检查是否还在运行，如果还在则强制 kill
            DispatchQueue.global().asyncAfter(deadline: .now() + 3.0) { [weak self] in
                if self?.isClawPartyProcessRunning() == true {
                    let forceKill = Process()
                    forceKill.executableURL = URL(fileURLWithPath: "/bin/sh")
                    forceKill.arguments = ["-c", "pgrep -x 'clawparty' | xargs kill -KILL 2>/dev/null"]
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

    // MARK: - 检查与修复二进制文件

    struct BinaryCheckResult {
        let found: [String]
        let fixed: [String]
        let failed: [(name: String, reason: String)]
    }

    /// 检查 PATH 中是否存在指定二进制文件，不存在的从当前目录拷贝到 /usr/local/bin
    func checkAndFixBinaries() -> BinaryCheckResult {
        let binaries = ["clawparty", "zeroclaw", "ztm", "opencode"]
        let cwd = FileManager.default.currentDirectoryPath
        let targetDir = "/usr/local/bin"
        var found: [String] = []
        var fixed: [String] = []
        var failed: [(String, String)] = []

        // 收集所有需要修复的文件，避免多次弹密码框
        var tasks: [(binary: String, source: String, target: String)] = []

        for binary in binaries {
            if isBinaryInPath(binary) {
                found.append(binary)
                continue
            }

            let sourcePath = "\(cwd)/\(binary)"
            let targetPath = "\(targetDir)/\(binary)"

            guard FileManager.default.fileExists(atPath: sourcePath) else {
                failed.append((binary, "当前目录找不到 \(binary)"))
                addLog(level: .warn, message: "当前目录找不到 \(binary)")
                continue
            }

            tasks.append((binary, sourcePath, targetPath))
        }

        // 一次性提权执行所有拷贝
        if !tasks.isEmpty {
            var scriptBody = ""
            for t in tasks {
                let safeSource = t.source.replacingOccurrences(of: "'", with: "'\\''")
                let safeTarget = t.target.replacingOccurrences(of: "'", with: "'\\''")
                scriptBody += "cp '\(safeSource)' '\(safeTarget)' && chmod +x '\(safeTarget)' && "
            }
            // 去掉末尾多余的 " && "
            scriptBody = String(scriptBody.dropLast(4))

            let binaryList = tasks.map { $0.binary }.joined(separator: "、")
            let prompt = "拷贝 \(binaryList) 到 \(targetDir) 目录需要管理员权限"
            let script = """
            do shell script "\(scriptBody)" with administrator privileges with prompt "\(prompt)"
            """

            let task = Process()
            task.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
            task.arguments = ["-e", script]

            let pipe = Pipe()
            task.standardOutput = pipe
            task.standardError = pipe

            do {
                try task.run()
                task.waitUntilExit()
                if task.terminationStatus == 0 {
                    for t in tasks {
                        fixed.append(t.binary)
                        addLog(level: .info, message: "已将 \(t.binary) 拷贝到 \(targetDir)")
                    }
                } else {
                    let data = pipe.fileHandleForReading.readDataToEndOfFile()
                    let err = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? "未知错误"
                    for t in tasks {
                        failed.append((t.binary, err))
                        addLog(level: .error, message: "拷贝 \(t.binary) 失败: \(err)")
                    }
                }
            } catch {
                for t in tasks {
                    failed.append((t.binary, error.localizedDescription))
                    addLog(level: .error, message: "拷贝 \(t.binary) 失败: \(error.localizedDescription)")
                }
            }
        }

        return BinaryCheckResult(found: found, fixed: fixed, failed: failed)
    }

    private func isBinaryInPath(_ binary: String) -> Bool {
        let envPath = "/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin"
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/bin/sh")
        task.environment = ["PATH": envPath]
        task.arguments = ["-c", "which \(binary) > /dev/null 2>&1 && echo 'found'"]

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

    // MARK: 综合检查系统

    func runAllChecks(agents: [AgentInfo]) {
        checkPhase = .checking
        let binaries = ["clawparty", "zeroclaw", "ztm", "opencode"]

        let allItems: [CheckItem] = {
            var items: [CheckItem] = []
            for binary in binaries {
                items.append(CheckItem(category: .executables, name: binary, detail: "检查 \(binary) 是否在 PATH 中"))
            }
            items.append(CheckItem(category: .zeroAgentConfig, name: "配置文件存在", detail: "~/.clawparty/.zeroclaw/config.toml"))
            items.append(CheckItem(category: .zeroAgentConfig, name: "api_key", detail: "0#Agent API Key 是否配置"))
            items.append(CheckItem(category: .zeroAgentConfig, name: "default_provider", detail: "0#Agent 默认提供商是否配置"))
            items.append(CheckItem(category: .zeroAgentConfig, name: "default_model", detail: "0#Agent 默认模型是否配置"))
            for agent in agents {
                let label = agent.displayName.isEmpty ? agent.agentName : "\(agent.displayName)(\(agent.agentName))"
                items.append(CheckItem(category: .agentConfigs, name: "\(agent.agentName) 配置文件", detail: agent.configPath))
                items.append(CheckItem(category: .agentConfigs, name: "\(agent.agentName) api_key", detail: "\(label) 的 API Key 是否配置"))
                items.append(CheckItem(category: .agentConfigs, name: "\(agent.agentName) provider", detail: "\(label) 的 default_provider 是否配置"))
                items.append(CheckItem(category: .agentConfigs, name: "\(agent.agentName) model", detail: "\(label) 的 default_model 是否配置"))
                items.append(CheckItem(category: .llmConnectivity, name: "\(agent.agentName) LLM 连通性", detail: "用 curl 测试 \(label) 的 LLM 服务是否可达"))
                items.append(CheckItem(category: .workspaceFiles, name: "\(agent.agentName) 工作区目录", detail: "\(label) 的 workspace 目录是否存在"))
                items.append(CheckItem(category: .workspaceFiles, name: "\(agent.agentName) 工作区可写", detail: "\(label) 的 workspace 目录是否可写"))
                items.append(CheckItem(category: .portConflicts, name: "\(agent.agentName) 端口 \(agent.port)", detail: "\(label) 端口 \(agent.port) 是否被占用"))
            }
            items.append(CheckItem(category: .databaseIntegrity, name: "数据库完整性", detail: "检查 clawparty.db 是否完整"))
            return items
        }()

        checkItems = allItems

        DispatchQueue.global().async { [weak self] in
            guard let self = self else { return }
            let home = ProcessInfo.processInfo.environment["HOME"] ?? NSHomeDirectory()

            for i in 0..<allItems.count {
                let item = allItems[i]
                DispatchQueue.main.async { self.checkItems[i].status = .checking }

                switch item.category {
                case .executables:
                    let info = self.getBinaryInfo(item.name)
                    DispatchQueue.main.async {
                        if info.found {
                            let path = info.path ?? "未知路径"
                            let ver = info.version ?? "未知版本"
                            self.checkItems[i].status = .pass(detail: "\(path) (版本: \(ver))")
                        } else if let sourcePath = self.findBinarySource(item.name) {
                            self.checkItems[i].status = .fail(detail: "未在 PATH 中找到，在 \(sourcePath) 找到")
                            self.checkItems[i].fixDescription = "从 \(sourcePath) 拷贝 \(item.name) 到 /usr/local/bin"
                            self.checkItems[i].selected = true
                        } else {
                            self.checkItems[i].status = .fail(detail: "在 PATH 及各常见目录均未找到")
                            self.checkItems[i].fixDescription = "请下载 \(item.name) 可执行文件"
                        }
                    }

                case .zeroAgentConfig:
                    let configPath = "\(home)/.clawparty/.zeroclaw/config.toml"
                    let fields = self.parseAgentConfigFields(configPath)
                    DispatchQueue.main.async {
                        if item.name == "配置文件存在" {
                            if FileManager.default.fileExists(atPath: configPath) {
                                self.checkItems[i].status = .pass(detail: configPath)
                            } else {
                                self.checkItems[i].status = .fail(detail: "配置文件不存在: \(configPath)")
                                self.checkItems[i].fixDescription = "生成默认 0#Agent 配置文件"
                                self.checkItems[i].selected = true
                            }
                        } else if item.name == "api_key" {
                            if let key = fields["api_key"], !key.isEmpty {
                                self.checkItems[i].status = .pass(detail: "已配置")
                            } else {
                                self.checkItems[i].status = .warning(detail: "api_key 未设置或为空")
                                self.checkItems[i].fixDescription = "请在配置文件中设置 api_key"
                            }
                        } else if item.name == "default_provider" {
                            if let prov = fields["default_provider"], !prov.isEmpty {
                                self.checkItems[i].status = .pass(detail: prov)
                            } else {
                                self.checkItems[i].status = .fail(detail: "default_provider 未设置")
                                self.checkItems[i].fixDescription = "设置 default_provider (如 openai)"
                            }
                        } else if item.name == "default_model" {
                            if let model = fields["default_model"], !model.isEmpty {
                                self.checkItems[i].status = .pass(detail: model)
                            } else {
                                self.checkItems[i].status = .fail(detail: "default_model 未设置")
                                self.checkItems[i].fixDescription = "设置 default_model (如 gpt-4o-mini)"
                            }
                        }
                    }

                case .agentConfigs:
                    let parts = item.name.components(separatedBy: " ")
                    guard let agentName = parts.first else { continue }
                    let configPath: String
                    if let agent = agents.first(where: { $0.agentName == agentName }) {
                        configPath = agent.configPath
                    } else {
                        configPath = "\(home)/.clawparty/agents/\(agentName)/config.toml"
                    }
                    let fields = self.parseAgentConfigFields(configPath)
                    DispatchQueue.main.async {
                        if item.name.hasSuffix("配置文件") {
                            if FileManager.default.fileExists(atPath: configPath) {
                                self.checkItems[i].status = .pass(detail: configPath)
                            } else {
                                self.checkItems[i].status = .fail(detail: "配置文件不存在: \(configPath)")
                                self.checkItems[i].fixDescription = "生成默认 \(agentName) 配置文件"
                                self.checkItems[i].selected = true
                            }
                        } else if item.name.hasSuffix("api_key") {
                            if let key = fields["api_key"], !key.isEmpty {
                                self.checkItems[i].status = .pass(detail: "已配置")
                            } else {
                                self.checkItems[i].status = .warning(detail: "api_key 未设置或为空")
                            }
                        } else if item.name.hasSuffix("provider") {
                            if let prov = fields["default_provider"], !prov.isEmpty {
                                self.checkItems[i].status = .pass(detail: prov)
                            } else {
                                self.checkItems[i].status = .fail(detail: "default_provider 未设置")
                            }
                        } else if item.name.hasSuffix("model") {
                            if let model = fields["default_model"], !model.isEmpty {
                                self.checkItems[i].status = .pass(detail: model)
                            } else {
                                self.checkItems[i].status = .fail(detail: "default_model 未设置")
                            }
                        }
                    }

                case .llmConnectivity:
                    let parts = item.name.components(separatedBy: " ")
                    guard let agentName = parts.first else { continue }
                    guard let agent = agents.first(where: { $0.agentName == agentName }) else {
                        DispatchQueue.main.async {
                            self.checkItems[i].status = .warning(detail: "未在数据库中找到 \(agentName)")
                        }
                        continue
                    }
                    let fields = self.parseAgentConfigFields(agent.configPath)
                    let apiKey = fields["api_key"] ?? ""
                    let apiUrl = fields["api_url"] ?? ""
                    let provider = fields["default_provider"] ?? ""
                    if apiKey.isEmpty {
                        DispatchQueue.main.async {
                            self.checkItems[i].status = .warning(detail: "api_key 未配置，跳过连通性测试")
                        }
                        continue
                    }
                    let reachable = self.testLLMReachability(apiKey: apiKey, apiUrl: apiUrl, provider: provider)
                    DispatchQueue.main.async {
                        if reachable {
                            self.checkItems[i].status = .pass(detail: "LLM 服务可达")
                        } else {
                            self.checkItems[i].status = .fail(detail: "LLM 服务不可达，请检查 API URL 和网络")
                        }
                    }

                case .workspaceFiles:
                    let parts = item.name.components(separatedBy: " ")
                    guard let agentName = parts.first else { continue }
                    let configDir: String
                    if let agent = agents.first(where: { $0.agentName == agentName }) {
                        configDir = (agent.configPath as NSString).deletingLastPathComponent
                    } else {
                        configDir = "\(home)/.clawparty/agents/\(agentName)"
                    }
                    let workspaceDir = "\(configDir)/workspace"
                    DispatchQueue.main.async {
                        if item.name.hasSuffix("工作区目录") {
                            if FileManager.default.fileExists(atPath: workspaceDir) {
                                self.checkItems[i].status = .pass(detail: workspaceDir)
                            } else {
                                self.checkItems[i].status = .fail(detail: "工作区目录不存在: \(workspaceDir)")
                                self.checkItems[i].fixDescription = "创建 \(agentName) 工作区目录"
                                self.checkItems[i].selected = true
                            }
                        } else if item.name.hasSuffix("工作区可写") {
                            let testFile = "\(workspaceDir)/.clawparty_write_test"
                            let writable = FileManager.default.createFile(atPath: testFile, contents: "test".data(using: .utf8))
                            if writable {
                                try? FileManager.default.removeItem(atPath: testFile)
                                self.checkItems[i].status = .pass(detail: "可写")
                            } else {
                                self.checkItems[i].status = .fail(detail: "工作区不可写: \(workspaceDir)")
                                self.checkItems[i].fixDescription = "修复 \(workspaceDir) 目录权限"
                            }
                        }
                    }

                case .portConflicts:
                    let parts = item.name.components(separatedBy: " ")
                    guard let agentName = parts.first else { continue }
                    guard let agent = agents.first(where: { $0.agentName == agentName }), agent.port > 0 else {
                        DispatchQueue.main.async {
                            self.checkItems[i].status = .warning(detail: "端口未分配")
                        }
                        continue
                    }
                    let portInfo = self.checkPortUsage(port: agent.port)
                    DispatchQueue.main.async {
                        if let info = portInfo {
                            self.checkItems[i].status = .fail(detail: "端口 \(agent.port) 被占用: \(info)")
                        } else {
                            self.checkItems[i].status = .pass(detail: "端口 \(agent.port) 空闲")
                        }
                    }

                case .databaseIntegrity:
                    let dbPath = "\(home)/.clawparty/clawparty.db"
                    guard FileManager.default.fileExists(atPath: dbPath) else {
                        DispatchQueue.main.async {
                            self.checkItems[i].status = .pass(detail: "数据库文件不存在（首次运行前正常）")
                        }
                        continue
                    }
                    let integriryResult = self.checkDatabaseIntegrity(dbPath: dbPath)
                    DispatchQueue.main.async {
                        if integriryResult == "ok" {
                            self.checkItems[i].status = .pass(detail: "数据库完整")
                        } else {
                            self.checkItems[i].status = .fail(detail: "数据库可能损坏: \(integriryResult)")
                        }
                    }
                }

                Thread.sleep(forTimeInterval: 0.05)
            }

            DispatchQueue.main.async {
                self.checkPhase = .report
            }
        }
    }

    func executeSelectedFixes() {
        let selected = checkItems.filter { $0.selected && $0.status.isProblem }
        repairLog = []
        checkPhase = .repairing

        let copyTargets = selected.filter { $0.category == .executables }.map { $0.name }
        let configTargets = selected.filter { $0.fixDescription.contains("生成") }
        let directoryTargets = selected.filter { $0.category == .workspaceFiles && $0.fixDescription.contains("创建") }

        DispatchQueue.global().async { [weak self] in
            guard let self = self else { return }
            var successCount = 0
            var failCount = 0

            if !copyTargets.isEmpty {
                self.fixCopyBinaries(copyTargets) { name, ok in
                    if ok {
                        successCount += 1
                        self.appendRepairLog("✅ 已拷贝 \(name) 到 /usr/local/bin")
                    } else {
                        failCount += 1
                        self.appendRepairLog("❌ 拷贝 \(name) 失败")
                    }
                }
            }

            for item in configTargets {
                let parts = item.name.components(separatedBy: " ")
                let agentName = parts.first ?? "unknown"
                let home = ProcessInfo.processInfo.environment["HOME"] ?? NSHomeDirectory()
                let configPath = agentName == "0" || item.category == .zeroAgentConfig
                    ? "\(home)/.clawparty/.zeroclaw/config.toml"
                    : "\(home)/.clawparty/agents/\(agentName)/config.toml"
                if self.fixGenerateConfig(for: configPath, agentName: agentName) {
                    successCount += 1
                    self.appendRepairLog("✅ 已生成 \(agentName) 配置文件")
                } else {
                    failCount += 1
                    self.appendRepairLog("❌ 生成 \(agentName) 配置文件失败")
                }
            }

            for item in directoryTargets {
                let parts = item.name.components(separatedBy: " ")
                let agentName = parts.first ?? "unknown"
                let home = ProcessInfo.processInfo.environment["HOME"] ?? NSHomeDirectory()
                let workspaceDir = "\(home)/.clawparty/agents/\(agentName)/workspace"
                do {
                    try FileManager.default.createDirectory(atPath: workspaceDir, withIntermediateDirectories: true)
                    successCount += 1
                    self.appendRepairLog("✅ 已创建 \(agentName) 工作区目录")
                } catch {
                    failCount += 1
                    self.appendRepairLog("❌ 创建 \(agentName) 工作区目录失败: \(error.localizedDescription)")
                }
            }

            if copyTargets.isEmpty && configTargets.isEmpty && directoryTargets.isEmpty {
                self.appendRepairLog("⚠️ 没有选中可自动修复的项目")
            }

            DispatchQueue.main.async {
                self.checkPhase = .done(successCount: successCount, failCount: failCount)
            }
        }
    }

    private func appendRepairLog(_ message: String) {
        DispatchQueue.main.async {
            self.repairLog.append(message)
        }
    }

    func resetCheckState() {
        checkItems = []
        checkPhase = .idle
        repairLog = []
    }

    private func getBinaryInfo(_ name: String) -> (found: Bool, path: String?, version: String?) {
        let envPath = "/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin"
        let pathTask = Process()
        pathTask.executableURL = URL(fileURLWithPath: "/bin/sh")
        pathTask.environment = ["PATH": envPath]
        pathTask.arguments = ["-c", "command -v \(name)"]
        let pathPipe = Pipe()
        pathTask.standardOutput = pathPipe
        pathTask.standardError = Pipe()
        do {
            try pathTask.run()
            pathTask.waitUntilExit()
        } catch {
            return (false, nil, nil)
        }
        let pathData = pathPipe.fileHandleForReading.readDataToEndOfFile()
        guard let pathOutput = String(data: pathData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
              !pathOutput.isEmpty else {
            return (false, nil, nil)
        }

        let verTask = Process()
        verTask.executableURL = URL(fileURLWithPath: "/bin/sh")
        verTask.arguments = ["-c", "\(name) --version 2>&1 | head -1"]
        let verPipe = Pipe()
        verTask.standardOutput = verPipe
        verTask.standardError = Pipe()
        do {
            try verTask.run()
            verTask.waitUntilExit()
        } catch {
            return (true, pathOutput, nil)
        }
        let verData = verPipe.fileHandleForReading.readDataToEndOfFile()
        let version = String(data: verData, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
        return (true, pathOutput, version?.isEmpty == false ? version : nil)
    }

    private func findBinarySource(_ name: String) -> String? {
        let cwd = FileManager.default.currentDirectoryPath

        let candidates: [String] = {
            var paths: [String] = []
            var dir = cwd
            for _ in 0..<6 {
                paths.append("\(dir)/bin/\(name)")
                paths.append("\(dir)/\(name)")
                let parent = (dir as NSString).deletingLastPathComponent
                if parent == dir { break }
                dir = parent
            }
            // Also search from the binary's own location
            if let execPath = Bundle.main.executableURL?.deletingLastPathComponent().path {
                var binDir = execPath
                for _ in 0..<8 {
                    paths.append("\(binDir)/bin/\(name)")
                    paths.append("\(binDir)/\(name)")
                    let parent = (binDir as NSString).deletingLastPathComponent
                    if parent == binDir { break }
                    binDir = parent
                }
            }
            return paths
        }()

        for path in candidates {
            if FileManager.default.fileExists(atPath: path) {
                return path
            }
        }
        return nil
    }

    private func parseAgentConfigFields(_ configPath: String) -> [String: String] {
        guard FileManager.default.fileExists(atPath: configPath),
              let content = try? String(contentsOfFile: configPath, encoding: .utf8) else {
            return [:]
        }
        var fields: [String: String] = [:]
        let lines = content.components(separatedBy: .newlines)
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            guard !trimmed.isEmpty, !trimmed.hasPrefix("#"), !trimmed.hasPrefix("[") else { continue }
            let parts = trimmed.split(separator: "=", maxSplits: 1).map { String($0).trimmingCharacters(in: .whitespaces) }
            guard parts.count == 2 else { continue }
            var value = parts[1]
            if value.hasPrefix("\"") && value.hasSuffix("\"") {
                value = String(value.dropFirst().dropLast())
            }
            if fields[parts[0]] == nil {
                fields[parts[0]] = value
            }
        }
        return fields
    }

    private func testLLMReachability(apiKey: String, apiUrl: String, provider: String) -> Bool {
        let targetUrl: String
        if !apiUrl.isEmpty {
            targetUrl = apiUrl.hasSuffix("/") ? String(apiUrl.dropLast()) : apiUrl
        } else {
            targetUrl = defaultApiUrl(for: provider)
        }
        guard !targetUrl.isEmpty else { return false }

        let safeKey = apiKey.replacingOccurrences(of: "'", with: "'\\''")
        let safeUrl = targetUrl.replacingOccurrences(of: "'", with: "'\\''")
        let script = "curl -s -o /dev/null -w '%{http_code}' --connect-timeout 5 --max-time 10 -H 'Authorization: Bearer \(safeKey)' '\(safeUrl)'"

        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/bin/sh")
        task.arguments = ["-c", script]
        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()
        do {
            try task.run()
            task.waitUntilExit()
        } catch {
            return false
        }
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        guard let output = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
              let statusCode = Int(output) else {
            return false
        }
        return statusCode >= 200 && statusCode < 500
    }

    private func defaultApiUrl(for provider: String) -> String {
        switch provider.lowercased() {
        case "openai": return "https://api.openai.com/v1/models"
        case "openrouter": return "https://openrouter.ai/api/v1/models"
        case "anthropic": return "https://api.anthropic.com/v1/messages"
        case "deepseek": return "https://api.deepseek.com/v1/models"
        case "gemini": return "https://generativelanguage.googleapis.com/v1/models"
        case "groq": return "https://api.groq.com/openai/v1/models"
        case "ollama": return "http://localhost:11434/api/tags"
        case "xai": return "https://api.x.ai/v1/models"
        case "moonshot": return "https://api.moonshot.cn/v1/models"
        case "zai", "glm": return ""
        case "opencode-go": return ""
        default: return ""
        }
    }

    private func fixCopyBinaries(_ names: [String], completion: @escaping (String, Bool) -> Void) {
        let targetDir = "/usr/local/bin"
        var tasks: [(name: String, source: String, target: String)] = []

        for name in names {
            guard let sourcePath = findBinarySource(name) else {
                completion(name, false)
                continue
            }
            let targetPath = "\(targetDir)/\(name)"
            tasks.append((name, sourcePath, targetPath))
        }

        guard !tasks.isEmpty else { return }

        var scriptBody = ""
        for t in tasks {
            let safeSource = t.source.replacingOccurrences(of: "'", with: "'\\''")
            let safeTarget = t.target.replacingOccurrences(of: "'", with: "'\\''")
            scriptBody += "cp '\(safeSource)' '\(safeTarget)' && chmod +x '\(safeTarget)' && "
        }
        scriptBody = String(scriptBody.dropLast(4))

        let binaryList = tasks.map { $0.name }.joined(separator: "、")
        let prompt = "拷贝 \(binaryList) 到 \(targetDir) 目录需要管理员权限"
        let script = """
        do shell script "\(scriptBody)" with administrator privileges with prompt "\(prompt)"
        """

        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        task.arguments = ["-e", script]
        do {
            try task.run()
            task.waitUntilExit()
            let ok = task.terminationStatus == 0
            for t in tasks { completion(t.name, ok) }
        } catch {
            for t in tasks { completion(t.name, false) }
        }
    }

    private func fixGenerateConfig(for configPath: String, agentName: String) -> Bool {
        let dir = (configPath as NSString).deletingLastPathComponent
        do {
            try FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        } catch {
            return false
        }

        let template = """
        # Agent: \(agentName)
        # Generated by ClawParty Desktop Check & Repair

        api_key = ""
        default_provider = "openai"
        default_model = "gpt-4o-mini"
        default_temperature = 0.7
        provider_timeout_secs = 120

        """
        let workspaceDir = "\(dir)/workspace"
        try? FileManager.default.createDirectory(atPath: workspaceDir, withIntermediateDirectories: true)

        do {
            try template.write(toFile: configPath, atomically: true, encoding: .utf8)
            return true
        } catch {
            return false
        }
    }

    private func checkPortUsage(port: Int) -> String? {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/bin/sh")
        task.arguments = ["-c", "lsof -i :\(port) -P -n 2>/dev/null | grep LISTEN"]
        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()
        do {
            try task.run()
            task.waitUntilExit()
        } catch {
            return nil
        }
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        guard let output = String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
              !output.isEmpty else {
            return nil
        }
        let lines = output.components(separatedBy: .newlines)
        var processes: [String] = []
        for line in lines {
            let cols = line.components(separatedBy: .whitespaces).filter { !$0.isEmpty }
            if cols.count >= 2 {
                let procName = cols[0]
                let pid = cols[1]
                processes.append("\(procName)(PID:\(pid))")
            }
        }
        return processes.isEmpty ? nil : processes.joined(separator: ", ")
    }

    private func checkDatabaseIntegrity(dbPath: String) -> String {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/sqlite3")
        task.arguments = [dbPath, "PRAGMA integrity_check;"]
        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()
        do {
            try task.run()
            task.waitUntilExit()
        } catch {
            return "无法执行检查: \(error.localizedDescription)"
        }
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        return String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? "未知"
    }

}
