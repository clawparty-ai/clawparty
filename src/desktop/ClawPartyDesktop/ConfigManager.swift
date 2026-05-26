import Foundation

struct AgentInfo: Identifiable, Hashable, Equatable {
    var id: String { agentName }
    let agentName: String
    let displayName: String
    let configPath: String
    let status: String
    let port: Int

    func hash(into hasher: inout Hasher) {
        hasher.combine(agentName)
    }

    static func == (lhs: AgentInfo, rhs: AgentInfo) -> Bool {
        lhs.agentName == rhs.agentName
    }
}

struct AgentLLMConfig {
    var apiKey: String = ""
    var provider: String = ""
    var model: String = ""
    var temperature: Double = 0.7
    var timeoutSecs: Int = 120
    var apiUrl: String = ""

    static let `default` = AgentLLMConfig()
}

class ConfigManager {
    static let shared = ConfigManager()

    private let dbPath: String

    private init() {
        let home = ProcessInfo.processInfo.environment["HOME"] ?? NSHomeDirectory()
        self.dbPath = "\(home)/.clawparty/clawparty.db"
    }

    // MARK: - 读取 Agent 列表

    func loadAgents() -> [AgentInfo] {
        guard FileManager.default.fileExists(atPath: dbPath) else {
            print("数据库不存在: \(dbPath)")
            return []
        }

        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/sqlite3")
        task.arguments = [dbPath, "SELECT agent_name, display_name, config_path, status, port FROM agents WHERE deleted = 0;"]

        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe()

        do {
            try task.run()
            task.waitUntilExit()

            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            guard let output = String(data: data, encoding: .utf8) else { return [] }

            var agents: [AgentInfo] = []
            let lines = output.components(separatedBy: .newlines)

            for line in lines {
                let parts = line.components(separatedBy: "|")
                guard parts.count >= 5 else { continue }

                let agentName = parts[0].trimmingCharacters(in: .whitespaces)
                let displayName = parts[1].trimmingCharacters(in: .whitespaces)
                let configPath = parts[2].trimmingCharacters(in: .whitespaces)
                let status = parts[3].trimmingCharacters(in: .whitespaces)
                let port = Int(parts[4].trimmingCharacters(in: .whitespaces)) ?? 0

                guard !agentName.isEmpty else { continue }

                agents.append(AgentInfo(
                    agentName: agentName,
                    displayName: displayName.isEmpty ? agentName : displayName,
                    configPath: configPath,
                    status: status,
                    port: port
                ))
            }

            return agents
        } catch {
            print("读取数据库失败: \(error)")
            return []
        }
    }

    // MARK: - 读取/写入 Agent LLM 配置

    func loadAgentConfig(agent: AgentInfo) -> AgentLLMConfig {
        guard FileManager.default.fileExists(atPath: agent.configPath),
              let content = try? String(contentsOfFile: agent.configPath, encoding: .utf8) else {
            return .default
        }
        return parseAgentToml(content)
    }

    func saveAgentConfig(agent: AgentInfo, config: AgentLLMConfig) -> Bool {
        guard FileManager.default.fileExists(atPath: agent.configPath),
              let originalContent = try? String(contentsOfFile: agent.configPath, encoding: .utf8) else {
            // 如果文件不存在，创建新文件
            let dir = (agent.configPath as NSString).deletingLastPathComponent
            try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
            let newContent = generateAgentToml(config)
            do {
                try newContent.write(toFile: agent.configPath, atomically: true, encoding: .utf8)
                return true
            } catch {
                print("保存配置失败: \(error)")
                return false
            }
        }

        let updatedContent = updateAgentToml(originalContent, with: config)

        do {
            try updatedContent.write(toFile: agent.configPath, atomically: true, encoding: .utf8)
            return true
        } catch {
            print("保存配置失败: \(error)")
            return false
        }
    }

    // MARK: - 原始 TOML 读写（编辑器用）

    func loadAgentConfigRaw(agent: AgentInfo) -> String {
        guard FileManager.default.fileExists(atPath: agent.configPath),
              let content = try? String(contentsOfFile: agent.configPath, encoding: .utf8) else {
            // 返回默认模板
            return defaultConfigTemplate(agentName: agent.displayName)
        }
        return content
    }

    func saveAgentConfigRaw(agent: AgentInfo, content: String) -> Bool {
        do {
            try content.write(toFile: agent.configPath, atomically: true, encoding: .utf8)
            return true
        } catch {
            print("保存配置失败: \(error)")
            return false
        }
    }

    private func defaultConfigTemplate(agentName: String) -> String {
        return """
        # Agent: \(agentName)
        # Generated by ClawParty Desktop

        api_key = ""
        default_provider = "openai"
        default_model = "gpt-4o-mini"
        default_temperature = 0.7
        provider_timeout_secs = 120
        """
    }

    // MARK: - TOML 解析

    private func parseAgentToml(_ content: String) -> AgentLLMConfig {
        var config = AgentLLMConfig()

        let lines = content.components(separatedBy: .newlines)
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            guard !trimmed.isEmpty, !trimmed.hasPrefix("#") else { continue }
            guard !trimmed.hasPrefix("[") else { continue }

            let parts = trimmed.split(separator: "=", maxSplits: 1).map { String($0).trimmingCharacters(in: .whitespaces) }
            guard parts.count == 2 else { continue }

            let key = parts[0]
            var value = parts[1]

            if value.hasPrefix("\"") && value.hasSuffix("\"") {
                value = String(value.dropFirst().dropLast())
            }

            switch key {
            case "api_key":
                config.apiKey = value
            case "default_provider":
                config.provider = value
            case "default_model":
                config.model = value
            case "default_temperature":
                config.temperature = Double(value) ?? 0.7
            case "provider_timeout_secs":
                config.timeoutSecs = Int(value) ?? 120
            case "api_url":
                config.apiUrl = value
            default:
                break
            }
        }

        return config
    }

    private func generateAgentToml(_ config: AgentLLMConfig) -> String {
        var lines: [String] = []

        lines.append("api_key = \"\(config.apiKey)\"")
        lines.append("default_provider = \"\(config.provider)\"")
        lines.append("default_model = \"\(config.model)\"")
        lines.append("default_temperature = \(config.temperature)")
        lines.append("provider_timeout_secs = \(config.timeoutSecs)")
        if !config.apiUrl.isEmpty {
            lines.append("api_url = \"\(config.apiUrl)\"")
        }

        return lines.joined(separator: "\n")
    }

    private func updateAgentToml(_ originalContent: String, with config: AgentLLMConfig) -> String {
        var lines = originalContent.components(separatedBy: .newlines)
        var updatedKeys = Set<String>()

        for i in 0..<lines.count {
            let trimmed = lines[i].trimmingCharacters(in: .whitespaces)
            guard !trimmed.isEmpty, !trimmed.hasPrefix("#") else { continue }
            guard !trimmed.hasPrefix("[") else { continue }

            let parts = trimmed.split(separator: "=", maxSplits: 1).map { String($0).trimmingCharacters(in: .whitespaces) }
            guard parts.count == 2 else { continue }

            let key = parts[0]

            switch key {
            case "api_key":
                lines[i] = "api_key = \"\(config.apiKey)\""
                updatedKeys.insert(key)
            case "default_provider":
                lines[i] = "default_provider = \"\(config.provider)\""
                updatedKeys.insert(key)
            case "default_model":
                lines[i] = "default_model = \"\(config.model)\""
                updatedKeys.insert(key)
            case "default_temperature":
                lines[i] = "default_temperature = \(config.temperature)"
                updatedKeys.insert(key)
            case "provider_timeout_secs":
                lines[i] = "provider_timeout_secs = \(config.timeoutSecs)"
                updatedKeys.insert(key)
            case "api_url":
                if !config.apiUrl.isEmpty {
                    lines[i] = "api_url = \"\(config.apiUrl)\""
                }
                updatedKeys.insert(key)
            default:
                break
            }
        }

        // 如果有未更新的键，添加到文件末尾
        if !updatedKeys.contains("api_key") {
            lines.append("api_key = \"\(config.apiKey)\"")
        }
        if !updatedKeys.contains("default_provider") {
            lines.append("default_provider = \"\(config.provider)\"")
        }
        if !updatedKeys.contains("default_model") {
            lines.append("default_model = \"\(config.model)\"")
        }
        if !updatedKeys.contains("default_temperature") {
            lines.append("default_temperature = \(config.temperature)")
        }
        if !updatedKeys.contains("provider_timeout_secs") {
            lines.append("provider_timeout_secs = \(config.timeoutSecs)")
        }
        if !updatedKeys.contains("api_url"), !config.apiUrl.isEmpty {
            lines.append("api_url = \"\(config.apiUrl)\"")
        }

        return lines.joined(separator: "\n")
    }
}
