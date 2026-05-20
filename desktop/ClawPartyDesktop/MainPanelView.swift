import SwiftUI

enum RightPanelMode {
    case log
    case editor
    case llmConfig
}

struct MainPanelView: View {
    @StateObject private var processManager = ProcessManager.shared
    @State private var agents: [AgentInfo] = []
    @State private var isLoading = true
    @State private var selectedAgent: AgentInfo?
    @State private var timer: Timer?
    @State private var logAutoScroll = true

    // Config editor state
    @State private var editingAgent: AgentInfo?
    @State private var tomlContent: String = ""
    @State private var showSaveAlert = false
    @State private var saveError = ""
    @State private var showSaveError = false

    // Right panel mode
    @State private var rightPanelMode: RightPanelMode = .log

    var body: some View {
        VStack(spacing: 0) {
            // 顶部状态栏
            HStack {
                Image(systemName: "lobster")
                    .font(.title2)
                    .foregroundColor(.red)

                Text("ClawParty")
                    .font(.title2)
                    .fontWeight(.bold)

                Spacer()

                HStack(spacing: 8) {
                    Circle()
                        .fill(processManager.isClawPartyRunning ? Color.green : Color.gray)
                        .frame(width: 10, height: 10)

                    Text(processManager.isClawPartyRunning ? "运行中" : "已停止")
                        .font(.subheadline)
                        .foregroundColor(processManager.isClawPartyRunning ? .green : .secondary)
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 6)
                .background(Color(NSColor.controlBackgroundColor))
                .cornerRadius(16)
            }
            .padding()

            Divider()

            // 控制按钮
            HStack(spacing: 12) {
                Button {
                    rightPanelMode = .llmConfig
                } label: {
                    Label("配置LLM", systemImage: "gearshape.2")
                        .frame(width: 90)
                }
                .buttonStyle(BorderedButtonStyle())

                Button {
                    openDownloadPage()
                } label: {
                    Label("下载", systemImage: "arrow.down.circle")
                        .frame(width: 90)
                }
                .buttonStyle(BorderedButtonStyle())

                Button {
                    ProcessManager.shared.startClawParty { _, _ in }
                } label: {
                    Label("启动", systemImage: "play.fill")
                        .frame(width: 90)
                }
                .disabled(processManager.isClawPartyRunning)
                .buttonStyle(BorderedProminentButtonStyle())
                .tint(.green)

                Button {
                    ProcessManager.shared.stopClawParty()
                } label: {
                    Label("停止", systemImage: "stop.fill")
                        .frame(width: 90)
                }
                .disabled(!processManager.isClawPartyRunning)
                .buttonStyle(BorderedProminentButtonStyle())
                .tint(.red)

                Button {
                    openClawPartyWeb()
                } label: {
                    Label("访问", systemImage: "globe")
                        .frame(width: 90)
                }
                .disabled(!processManager.isClawPartyRunning)
                .buttonStyle(BorderedButtonStyle())

                Spacer()
            }
            .padding()

            Divider()

            // 主内容区 - 左右分栏
            HStack(spacing: 0) {
                // 左侧：Agent 列表
                VStack(alignment: .leading, spacing: 0) {
                    HStack {
                        Text("Agents")
                            .font(.headline)

                        Spacer()

                        Button {
                            loadAgents()
                        } label: {
                            Image(systemName: "arrow.clockwise")
                        }
                        .buttonStyle(BorderlessButtonStyle())
                    }
                    .padding()

                    if isLoading {
                        ProgressView("正在加载...")
                            .padding()
                            .frame(maxWidth: .infinity, alignment: .center)
                    } else if agents.isEmpty {
                        VStack(spacing: 12) {
                            Image(systemName: "person.3")
                                .font(.system(size: 40))
                                .foregroundColor(.secondary)
                            Text("没有找到 Agent")
                                .foregroundColor(.secondary)
                            Text("请先启动 ClawParty 创建 Agent")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        .padding()
                        .frame(maxWidth: .infinity, alignment: .center)
                    } else {
                        List(agents, selection: $selectedAgent) { agent in
                            AgentRowView(
                                agent: agent,
                                isSelected: selectedAgent?.id == agent.id,
                                isEditing: editingAgent?.id == agent.id,
                                onConfig: {
                                    startEditing(agent: agent)
                                }
                            )
                            .contentShape(Rectangle())
                            .onTapGesture {
                                selectedAgent = agent
                            }
                        }
                        .listStyle(PlainListStyle())
                    }
                }
                .frame(width: 320)

                Divider()

                // 右侧：根据模式显示不同内容
                switch rightPanelMode {
                case .log:
                    LogPanelView(autoScroll: $logAutoScroll)
                case .editor:
                    if let editingAgent = editingAgent {
                        ConfigEditorView(
                            agent: editingAgent,
                            content: $tomlContent,
                            onSave: saveCurrentConfig,
                            onCancel: cancelEditing
                        )
                    } else {
                        LogPanelView(autoScroll: $logAutoScroll)
                    }
                case .llmConfig:
                    LLMConfigPanelView(
                        agents: agents,
                        onClose: { rightPanelMode = .log }
                    )
                }
            }
        }
        .frame(minWidth: 900, minHeight: 500)
        .onAppear {
            loadAgents()
            startTimer()
        }
        .onDisappear {
            timer?.invalidate()
        }
        .alert("配置已保存", isPresented: $showSaveAlert) {
            Button("确定", role: .cancel) {}
        }
        .alert("保存失败", isPresented: $showSaveError) {
            Button("确定", role: .cancel) {}
        } message: {
            Text(saveError)
        }
    }

    private func startEditing(agent: AgentInfo) {
        editingAgent = agent
        tomlContent = ConfigManager.shared.loadAgentConfigRaw(agent: agent)
        rightPanelMode = .editor
    }

    private func cancelEditing() {
        rightPanelMode = .log
        editingAgent = nil
        tomlContent = ""
    }

    private func saveCurrentConfig() {
        guard let agent = editingAgent else { return }
        if ConfigManager.shared.saveAgentConfigRaw(agent: agent, content: tomlContent) {
            showSaveAlert = true
        } else {
            saveError = "无法保存到 \(agent.configPath)"
            showSaveError = true
        }
    }

    private func openDownloadPage() {
        guard let url = URL(string: "https://github.com/clawparty-ai/clawparty/releases") else { return }
        NSWorkspace.shared.open(url)
    }

    private func openClawPartyWeb() {
        guard let url = URL(string: "https://localhost") else { return }
        NSWorkspace.shared.open(url)
    }

    private func startTimer() {
        timer = Timer.scheduledTimer(withTimeInterval: 3.0, repeats: true) { _ in
            loadAgents()
        }
    }

    private func loadAgents() {
        DispatchQueue.global().async {
            let loadedAgents = ConfigManager.shared.loadAgents()
            DispatchQueue.main.async {
                self.agents = loadedAgents
                self.isLoading = false
            }
        }
    }
}

// MARK: - LLM Config Panel

struct LLMConfigPanelView: View {
    let agents: [AgentInfo]
    let onClose: () -> Void

    @State private var selectedAgent: AgentInfo?
    @State private var agentConfig = AgentLLMConfig()
    @State private var showSavedAlert = false
    @State private var saveError = ""
    @State private var showSaveError = false

    let providers = [
        "openai", "openrouter", "anthropic", "gemini", "ollama",
        "deepseek", "groq", "zai", "glm", "xai", "moonshot", "opencode-go"
    ]

    let models = [
        "gpt-4o-mini", "gpt-4o", "gpt-4-turbo", "claude-3-5-sonnet",
        "claude-3-opus", "gemini-pro", "deepseek-chat", "deepseek-v4-flash",
        "llama3", "glm-4"
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // 标题栏
            HStack {
                Image(systemName: "gearshape.2")
                    .foregroundColor(.accentColor)
                Text("Agent LLM 配置")
                    .font(.headline)

                Spacer()

                Button {
                    onClose()
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .font(.title3)
                        .foregroundColor(.secondary)
                }
                .buttonStyle(BorderlessButtonStyle())
            }
            .padding()

            Divider()

            if agents.isEmpty {
                Spacer()
                VStack(spacing: 12) {
                    Image(systemName: "person.3")
                        .font(.system(size: 40))
                        .foregroundColor(.secondary)
                    Text("没有找到 Agent")
                        .foregroundColor(.secondary)
                    Text("请先启动 ClawParty 创建 Agent")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .center)
                Spacer()
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        // Agent 选择卡片
                        configCard(title: "选择 Agent", icon: "person") {
                            VStack(alignment: .leading, spacing: 8) {
                                Text("当前 Agent")
                                    .font(.caption)
                                    .foregroundColor(.secondary)

                                Picker("", selection: $selectedAgent) {
                                    ForEach(agents) { agent in
                                        HStack {
                                            Text(agent.displayName)
                                            Spacer()
                                            Text(agent.status)
                                                .font(.caption2)
                                                .foregroundColor(.secondary)
                                        }
                                        .tag(agent as AgentInfo?)
                                    }
                                }
                                .pickerStyle(MenuPickerStyle())
                                .onChange(of: selectedAgent) { agent in
                                    if let agent = agent {
                                        agentConfig = ConfigManager.shared.loadAgentConfig(agent: agent)
                                    }
                                }
                            }
                        }

                        if selectedAgent != nil {
                            // 提供商设置卡片
                            configCard(title: "提供商", icon: "cpu") {
                                VStack(alignment: .leading, spacing: 12) {
                                    configRow(label: "Provider") {
                                        Picker("", selection: $agentConfig.provider) {
                                            Text("请选择").tag("")
                                            ForEach(providers, id: \.self) { provider in
                                                Text(provider).tag(provider)
                                            }
                                        }
                                        .pickerStyle(MenuPickerStyle())
                                    }

                                    configRow(label: "Model") {
                                        HStack(spacing: 8) {
                                            Picker("", selection: $agentConfig.model) {
                                                Text("请选择").tag("")
                                                ForEach(models, id: \.self) { model in
                                                    Text(model).tag(model)
                                                }
                                            }
                                            .pickerStyle(MenuPickerStyle())
                                            .frame(minWidth: 120)

                                            TextField("自定义", text: $agentConfig.model)
                                                .textFieldStyle(RoundedBorderTextFieldStyle())
                                                .frame(minWidth: 100)
                                        }
                                    }
                                }
                            }

                            // API 设置卡片
                            configCard(title: "API 设置", icon: "key") {
                                VStack(alignment: .leading, spacing: 12) {
                                    configRow(label: "API Key") {
                                        SecureField("输入 API Key", text: $agentConfig.apiKey)
                                            .textFieldStyle(RoundedBorderTextFieldStyle())
                                    }

                                    configRow(label: "API URL") {
                                        TextField("可选，例如 https://api.openai.com/v1", text: $agentConfig.apiUrl)
                                            .textFieldStyle(RoundedBorderTextFieldStyle())
                                    }
                                }
                            }

                            // 参数设置卡片
                            configCard(title: "参数", icon: "slider.horizontal.3") {
                                VStack(alignment: .leading, spacing: 12) {
                                    configRow(label: "Temperature") {
                                        HStack(spacing: 12) {
                                            Slider(value: $agentConfig.temperature, in: 0.0...2.0, step: 0.1)
                                                .frame(minWidth: 100)
                                            Text("\(String(format: "%.1f", agentConfig.temperature))")
                                                .font(.system(.body, design: .monospaced))
                                                .foregroundColor(.secondary)
                                                .frame(width: 36, alignment: .trailing)
                                        }
                                    }

                                    configRow(label: "Timeout") {
                                        HStack(spacing: 8) {
                                            TextField("120", value: $agentConfig.timeoutSecs, format: .number)
                                                .textFieldStyle(RoundedBorderTextFieldStyle())
                                                .frame(width: 80)
                                            Text("秒")
                                                .foregroundColor(.secondary)
                                        }
                                    }
                                }
                            }

                            // 保存按钮
                            HStack {
                                Spacer()
                                Button {
                                    saveConfig()
                                } label: {
                                    HStack(spacing: 6) {
                                        Image(systemName: "checkmark.circle.fill")
                                        Text("保存配置")
                                    }
                                    .padding(.horizontal, 16)
                                    .padding(.vertical, 8)
                                }
                                .buttonStyle(BorderedProminentButtonStyle())
                                .tint(.green)
                                .disabled(selectedAgent == nil)
                                Spacer()
                            }
                            .padding(.top, 8)
                        } else {
                            Spacer()
                            Text("请选择一个 Agent")
                                .foregroundColor(.secondary)
                                .frame(maxWidth: .infinity, alignment: .center)
                            Spacer()
                        }
                    }
                    .padding()
                }
            }
        }
        .background(Color(NSColor.controlBackgroundColor).opacity(0.3))
        .onAppear {
            if let first = agents.first {
                selectedAgent = first
                agentConfig = ConfigManager.shared.loadAgentConfig(agent: first)
            }
        }
        .alert("配置已保存", isPresented: $showSavedAlert) {
            Button("确定", role: .cancel) {}
        }
        .alert("保存失败", isPresented: $showSaveError) {
            Button("确定", role: .cancel) {}
        } message: {
            Text(saveError)
        }
    }

    @ViewBuilder
    private func configCard<Content: View>(title: String, icon: String, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 6) {
                Image(systemName: icon)
                    .foregroundColor(.accentColor)
                    .font(.caption)
                Text(title)
                    .font(.subheadline)
                    .fontWeight(.semibold)
            }
            .padding(.bottom, 4)

            content()
        }
        .padding()
        .background(Color(NSColor.controlBackgroundColor))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(Color.gray.opacity(0.15), lineWidth: 1)
        )
    }

    @ViewBuilder
    private func configRow<Content: View>(label: String, @ViewBuilder content: () -> Content) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Text(label)
                .font(.subheadline)
                .foregroundColor(.secondary)
                .frame(width: 90, alignment: .trailing)

            content()
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private func saveConfig() {
        guard let agent = selectedAgent else { return }

        if ConfigManager.shared.saveAgentConfig(agent: agent, config: agentConfig) {
            showSavedAlert = true
        } else {
            saveError = "无法保存到 \(agent.configPath)"
            showSaveError = true
        }
    }
}

// MARK: - Config Editor

struct ConfigEditorView: View {
    let agent: AgentInfo
    @Binding var content: String
    let onSave: () -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // 编辑器标题栏
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("编辑配置")
                        .font(.headline)
                    Text(agent.configPath)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                }

                Spacer()

                Button {
                    onSave()
                } label: {
                    Label("保存", systemImage: "checkmark.circle.fill")
                }
                .buttonStyle(BorderedProminentButtonStyle())
                .controlSize(.small)
                .tint(.green)

                Button {
                    onCancel()
                } label: {
                    Label("取消", systemImage: "xmark.circle")
                }
                .buttonStyle(BorderedButtonStyle())
                .controlSize(.small)
            }
            .padding(.horizontal)
            .padding(.vertical, 8)

            Divider()

            // TOML 编辑器
            TextEditor(text: $content)
                .font(.system(size: 12, design: .monospaced))
                .padding(4)
                .background(Color(NSColor.textBackgroundColor))
        }
    }
}

// MARK: - Log Panel

struct LogPanelView: View {
    @Binding var autoScroll: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // 日志标题栏
            HStack {
                Text("日志")
                    .font(.headline)

                Spacer()

                Toggle("自动滚动", isOn: $autoScroll)
                    .toggleStyle(CheckboxToggleStyle())
                    .font(.caption)

                Button {
                    ProcessManager.shared.clearLogs()
                } label: {
                    Image(systemName: "trash")
                        .font(.caption)
                }
                .buttonStyle(BorderlessButtonStyle())
                .help("清空日志")
            }
            .padding(.horizontal)
            .padding(.vertical, 8)

            // 日志内容
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 2) {
                        ForEach(ProcessManager.shared.logs) { log in
                            LogRowView(log: log)
                                .id(log.id)
                        }
                    }
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                }
                .background(Color.black.opacity(0.05))
                .onChange(of: ProcessManager.shared.logs.count) { _ in
                    if autoScroll, let last = ProcessManager.shared.logs.last {
                        withAnimation {
                            proxy.scrollTo(last.id, anchor: .bottom)
                        }
                    }
                }
            }
        }
    }
}

// MARK: - Agent Row

struct AgentRowView: View {
    let agent: AgentInfo
    let isSelected: Bool
    let isEditing: Bool
    let onConfig: () -> Void

    var body: some View {
        HStack(spacing: 10) {
            Circle()
                .fill(statusColor)
                .frame(width: 8, height: 8)

            Image(systemName: "person.fill")
                .font(.title3)
                .foregroundColor(.accentColor)
                .frame(width: 28, height: 28)
                .background(Color.accentColor.opacity(0.1))
                .clipShape(Circle())

            VStack(alignment: .leading, spacing: 2) {
                Text(agent.displayName)
                    .font(.system(size: 13, weight: .medium))

                Text(agent.agentName)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Spacer()

            // 状态标签
            Text(agent.status)
                .font(.caption2)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(statusColor.opacity(0.15))
                .foregroundColor(statusColor)
                .cornerRadius(4)

            // 配置按钮
            Button {
                onConfig()
            } label: {
                Image(systemName: isEditing ? "gearshape.fill" : "gearshape")
                    .font(.system(size: 12))
            }
            .buttonStyle(BorderlessButtonStyle())
            .help("编辑配置")
        }
        .padding(.vertical, 4)
        .padding(.horizontal, 8)
        .background(isSelected ? Color.accentColor.opacity(0.1) : Color.clear)
        .cornerRadius(6)
    }

    private var statusColor: Color {
        switch agent.status.lowercased() {
        case "running", "active", "online":
            return .green
        case "stopped", "offline":
            return .gray
        case "error", "failed":
            return .red
        default:
            return .orange
        }
    }
}

// MARK: - Log Row

struct LogRowView: View {
    let log: LogEntry

    var body: some View {
        HStack(alignment: .top, spacing: 6) {
            Text(log.formattedTime)
                .font(.system(size: 10, design: .monospaced))
                .foregroundColor(.secondary)
                .frame(width: 60, alignment: .leading)

            Text(log.level.rawValue)
                .font(.system(size: 10, weight: .bold, design: .monospaced))
                .foregroundColor(levelColor)
                .frame(width: 40, alignment: .leading)

            Text(log.message)
                .font(.system(size: 11, design: .monospaced))
                .foregroundColor(.primary)
                .lineLimit(nil)
                .fixedSize(horizontal: false, vertical: true)

            Spacer()
        }
        .padding(.vertical, 1)
    }

    private var levelColor: Color {
        switch log.level {
        case .info: return .green
        case .error: return .red
        case .warn: return .orange
        case .debug: return .blue
        }
    }
}

#Preview {
    MainPanelView()
}
