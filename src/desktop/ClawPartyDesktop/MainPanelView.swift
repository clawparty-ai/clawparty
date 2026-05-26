import SwiftUI

enum RightPanelMode {
    case log
    case editor
    case llmConfig
    case terminal(AgentInfo)
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

    // First-run setup state
    @State private var showFirstRunSetup = false
    @State private var firstRunPassword = ""
    @State private var firstRunApiKey = ""
    @State private var isStarting = false

    // Right panel mode
    @State private var rightPanelMode: RightPanelMode = .log

    // Check & Repair
    @State private var showCheckRepairSheet = false

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
                    rightPanelMode = .log
                } label: {
                    Label("查看日志", systemImage: "doc.text")
                        .frame(width: 90)
                }
                .buttonStyle(BorderedButtonStyle())

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
                    runCheckAndFix()
                } label: {
                    Label("检查与修复", systemImage: "wrench.and.screwdriver")
                        .frame(width: 110)
                }
                .buttonStyle(BorderedButtonStyle())

                Button {
                    if ProcessManager.isFirstRun() {
                        showFirstRunSetup = true
                    } else {
                        ProcessManager.shared.startClawParty { _, _ in }
                    }
                } label: {
                    Label("启动", systemImage: "play.fill")
                        .frame(width: 90)
                }
                .disabled(processManager.isClawPartyRunning || isStarting)
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
                                rightPanelMode = .terminal(agent)
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
                            onCancel: cancelEditing,
                            onTerm: {
                                rightPanelMode = .terminal(editingAgent)
                            }
                        )
                    } else {
                        LogPanelView(autoScroll: $logAutoScroll)
                    }
                case .llmConfig:
                    LLMConfigPanelView(
                        agents: agents,
                        onClose: { rightPanelMode = .log }
                    )
                case .terminal:
                    TerminalPanelView(
                        selectedAgent: $selectedAgent,
                        onConfig: {
                            if let agent = selectedAgent {
                                startEditing(agent: agent)
                            }
                        },
                        onClose: {
                            rightPanelMode = .log
                        }
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
        .sheet(isPresented: $showCheckRepairSheet, onDismiss: {
            ProcessManager.shared.resetCheckState()
        }) {
            CheckRepairSheetView(agents: agents)
        }
        .sheet(isPresented: $showFirstRunSetup) {
            FirstRunSetupView(
                password: $firstRunPassword,
                apiKey: $firstRunApiKey,
                isStarting: $isStarting,
                onSetup: {
                    showFirstRunSetup = false
                    ProcessManager.shared.startClawParty(
                        adminPassword: firstRunPassword,
                        apiKey: firstRunApiKey
                    ) { _, _ in }
                },
                onCancel: {
                    showFirstRunSetup = false
                    firstRunPassword = ""
                    firstRunApiKey = ""
                }
            )
        }
    }

    private func startEditing(agent: AgentInfo) {
        if editingAgent?.id != agent.id {
            editingAgent = agent
            tomlContent = ConfigManager.shared.loadAgentConfigRaw(agent: agent)
        }
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

    private func runCheckAndFix() {
        showCheckRepairSheet = true
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
    let onTerm: () -> Void

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
                    onTerm()
                } label: {
                    Label("终端", systemImage: "terminal")
                }
                .buttonStyle(BorderedButtonStyle())
                .controlSize(.small)
                .help("切换到终端")

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

// MARK: - First-Run Setup Sheet

struct FirstRunSetupView: View {
    @Binding var password: String
    @Binding var apiKey: String
    @Binding var isStarting: Bool
    let onSetup: () -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "lobster")
                .font(.system(size: 40))
                .foregroundColor(.red)

            Text("首次运行设置")
                .font(.title2)
                .fontWeight(.bold)

            Text("检测到 ClawParty 初次运行，请设置管理员密码和 API Key。")
                .font(.subheadline)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)

            VStack(alignment: .leading, spacing: 8) {
                Text("管理员密码")
                    .font(.caption)
                    .foregroundColor(.secondary)
                SecureField("输入管理员密码", text: $password)
                    .textFieldStyle(RoundedBorderTextFieldStyle())

                Text("API Key")
                    .font(.caption)
                    .foregroundColor(.secondary)
                SecureField("输入 API Key", text: $apiKey)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
            }
            .padding(.horizontal)

            HStack(spacing: 12) {
                Button("取消") {
                    onCancel()
                }
                .buttonStyle(BorderedButtonStyle())
                .disabled(isStarting)

                Button {
                    isStarting = true
                    onSetup()
                } label: {
                    HStack(spacing: 6) {
                        if isStarting {
                            ProgressView()
                                .scaleEffect(0.7)
                        }
                        Text("启动")
                    }
                    .frame(width: 80)
                }
                .buttonStyle(BorderedProminentButtonStyle())
                .tint(.green)
                .disabled(password.isEmpty || apiKey.isEmpty || isStarting)
            }
        }
        .padding(30)
        .frame(width: 420)
    }
}

struct CheckRepairSheetView: View {
    let agents: [AgentInfo]
    @Environment(\.dismiss) private var dismiss
    @StateObject private var pm = ProcessManager.shared

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Image(systemName: "wrench.and.screwdriver")
                    .foregroundColor(.accentColor)
                Text("检查与修复")
                    .font(.title3)
                    .fontWeight(.semibold)
                Spacer()
                if pm.checkPhase == .report || pm.checkPhase == .plan || {
                    if case .done = pm.checkPhase { return true }; return false
                }() {
                    Button("关闭") { dismiss() }
                        .buttonStyle(BorderedButtonStyle())
                        .controlSize(.small)
                }
            }
            .padding()
            Divider()

            switch pm.checkPhase {
            case .idle:
                idleView
            case .checking:
                checkingView
            case .report:
                reportView
            case .plan:
                planView
            case .repairing:
                repairingView
            case .done(let successCount, let failCount):
                doneView(successCount: successCount, failCount: failCount)
            }
        }
        .frame(minWidth: 600, minHeight: 450)
        .onAppear {
            if pm.checkPhase == .idle {
                pm.runAllChecks(agents: agents)
            }
        }
    }

    private var idleView: some View {
        VStack(spacing: 20) {
            ProgressView()
                .scaleEffect(1.5)
            Text("准备检查...")
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var checkingView: some View {
        VStack(spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(CheckCategory.allCases) { category in
                        let items = pm.checkItems.filter { $0.category == category }
                        if !items.isEmpty {
                            CategoryCheckRow(category: category, items: items)
                            Divider().padding(.leading, 44)
                        }
                    }
                }
                .padding()
            }
            HStack {
                ProgressView()
                    .scaleEffect(0.7)
                let done = pm.checkItems.filter { $0.status != .pending && $0.status != .checking }.count
                Text("检查中... \(done)/\(pm.checkItems.count)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding(.horizontal)
            .padding(.bottom, 12)
        }
    }

    private var reportView: some View {
        let problemItems = pm.checkItems.filter { $0.status.isProblem }
        let fixableCount = problemItems.filter { !$0.fixDescription.isEmpty }.count

        return VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("检查完成")
                        .font(.headline)
                    if problemItems.isEmpty {
                        Text("所有检查项均通过 ✅")
                            .font(.subheadline)
                            .foregroundColor(.green)
                    } else {
                        Text("发现 \(problemItems.count) 个问题，其中 \(fixableCount) 个可自动修复")
                            .font(.subheadline)
                            .foregroundColor(.orange)
                    }
                }
                Spacer()
                if !problemItems.isEmpty {
                    Button("全选") {
                        for i in 0..<pm.checkItems.count where pm.checkItems[i].status.isProblem && !pm.checkItems[i].fixDescription.isEmpty {
                            pm.checkItems[i].selected = true
                        }
                    }
                    .buttonStyle(BorderedButtonStyle())
                    .controlSize(.small)
                }
            }
            .padding()

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(CheckCategory.allCases) { category in
                        let items = pm.checkItems.filter { $0.category == category }
                        if !items.isEmpty {
                            VStack(alignment: .leading, spacing: 0) {
                                HStack(spacing: 6) {
                                    Image(systemName: category.icon)
                                        .foregroundColor(.accentColor)
                                        .frame(width: 20)
                                    Text(category.label)
                                        .font(.subheadline)
                                        .fontWeight(.semibold)
                                    Spacer()
                                    let catProblems = items.filter { $0.status.isProblem }.count
                                    let catTotal = items.count
                                    Text("\(catTotal - catProblems)/\(catTotal) 通过")
                                        .font(.caption)
                                        .foregroundColor(catProblems == 0 ? .green : .orange)
                                }
                                .padding(.horizontal)
                                .padding(.vertical, 8)
                                .background(Color(NSColor.controlBackgroundColor))

                                ForEach(items) { item in
                                    ReportItemRow(
                                        item: item,
                                        onToggle: {
                                            if let idx = pm.checkItems.firstIndex(where: { $0.id == item.id }) {
                                                pm.checkItems[idx].selected.toggle()
                                            }
                                        }
                                    )
                                }
                            }
                            Divider()
                        }
                    }
                }
                .padding(.bottom, 20)
            }

            if !problemItems.isEmpty {
                Divider()
                HStack {
                    Spacer()
                    Button {
                        dismiss()
                    } label: {
                        Text("取消")
                            .frame(width: 80)
                    }
                    .buttonStyle(BorderedButtonStyle())

                    Button {
                        pm.checkPhase = .plan
                    } label: {
                        Text("开始修复 (\(pm.checkItems.filter { $0.selected }.count) 项)")
                            .frame(width: 160)
                    }
                    .buttonStyle(BorderedProminentButtonStyle())
                    .tint(.blue)
                    .disabled(pm.checkItems.filter { $0.selected }.isEmpty)
                }
                .padding()
            }
        }
    }

    private var planView: some View {
        let selected = pm.checkItems.filter { $0.selected && $0.status.isProblem && !$0.fixDescription.isEmpty }
        let grouped = Dictionary(grouping: selected) { $0.category }

        return VStack(spacing: 0) {
            HStack {
                Image(systemName: "checklist")
                    .foregroundColor(.blue)
                Text("修复计划")
                    .font(.headline)
                Spacer()
            }
            .padding()

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Text("将执行以下 \(selected.count) 项修复：")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                        .padding(.horizontal)
                        .padding(.top, 8)

                    ForEach(CheckCategory.allCases) { category in
                        if let items = grouped[category], !items.isEmpty {
                            VStack(alignment: .leading, spacing: 8) {
                                HStack(spacing: 6) {
                                    Image(systemName: category.icon)
                                        .foregroundColor(.accentColor)
                                    Text(category.label)
                                        .font(.subheadline)
                                        .fontWeight(.semibold)
                                }
                                ForEach(items) { item in
                                    HStack(alignment: .top, spacing: 8) {
                                        Text("•")
                                        VStack(alignment: .leading, spacing: 2) {
                                            Text(item.name)
                                                .font(.callout)
                                            Text(item.fixDescription)
                                                .font(.caption)
                                                .foregroundColor(.secondary)
                                        }
                                    }
                                    .padding(.leading, 12)
                                }
                            }
                            .padding(.horizontal)
                            Divider().padding(.leading, 44)
                        }
                    }
                }
                .padding(.vertical, 8)
            }

            Divider()
            HStack {
                Button {
                    pm.checkPhase = .report
                } label: {
                    Text("返回")
                        .frame(width: 80)
                }
                .buttonStyle(BorderedButtonStyle())

                Spacer()

                Button {
                    pm.executeSelectedFixes()
                } label: {
                    Label("确认并开始修复", systemImage: "play.fill")
                        .frame(width: 160)
                }
                .buttonStyle(BorderedProminentButtonStyle())
                .tint(.green)
            }
            .padding()
        }
    }

    private var repairingView: some View {
        VStack(spacing: 0) {
            HStack {
                ProgressView()
                    .scaleEffect(0.8)
                Text("正在修复...")
                    .font(.headline)
                Spacer()
            }
            .padding()

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(pm.repairLog.indices, id: \.self) { idx in
                        let log = pm.repairLog[idx]
                        HStack(spacing: 6) {
                            Text(log)
                                .font(.system(size: 12, design: .monospaced))
                            Spacer()
                        }
                        .padding(.horizontal)
                        .padding(.vertical, 3)
                    }
                }
                .padding(.vertical, 8)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private func doneView(successCount: Int, failCount: Int) -> some View {
        VStack(spacing: 0) {
            HStack {
                if failCount == 0 {
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundColor(.green)
                        .font(.title2)
                    Text("修复完成")
                        .font(.headline)
                        .foregroundColor(.green)
                } else {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(.orange)
                        .font(.title2)
                    Text("修复完成（部分失败）")
                        .font(.headline)
                        .foregroundColor(.orange)
                }
                Spacer()
            }
            .padding()

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 8) {
                    HStack(spacing: 16) {
                        Label("成功: \(successCount) 项", systemImage: "checkmark.circle.fill")
                            .foregroundColor(.green)
                        Label("失败: \(failCount) 项", systemImage: "xmark.circle.fill")
                            .foregroundColor(failCount > 0 ? .red : .secondary)
                    }
                    .font(.subheadline)
                    .padding(.horizontal)
                    .padding(.top, 12)

                    Divider().padding(.horizontal)

                    ForEach(pm.repairLog.indices, id: \.self) { idx in
                        let log = pm.repairLog[idx]
                        HStack(spacing: 6) {
                            Text(log)
                                .font(.system(size: 12, design: .monospaced))
                            Spacer()
                        }
                        .padding(.horizontal)
                    }
                }
                .padding(.bottom, 20)
            }

            Divider()
            HStack {
                Spacer()
                Button {
                    dismiss()
                } label: {
                    Text("关闭")
                        .frame(width: 80)
                }
                .buttonStyle(BorderedProminentButtonStyle())
                .tint(.blue)
            }
            .padding()
        }
    }
}

private struct CategoryCheckRow: View {
    let category: CheckCategory
    let items: [CheckItem]

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 6) {
                Image(systemName: category.icon)
                    .foregroundColor(.accentColor)
                    .frame(width: 20)
                Text(category.label)
                    .font(.subheadline)
                    .fontWeight(.medium)
                Spacer()
                let done = items.filter { $0.status != .pending && $0.status != .checking }.count
                Text("\(done)/\(items.count)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            ForEach(items) { item in
                HStack(spacing: 6) {
                    Image(systemName: item.status.icon)
                        .font(.caption)
                        .foregroundColor(statusColor(item.status))
                        .frame(width: 16)
                    Text(item.name)
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(item.status.label)
                        .font(.caption2)
                        .foregroundColor(statusColor(item.status))
                }
                .padding(.leading, 26)
                .padding(.vertical, 1)
            }
        }
    }

    private func statusColor(_ status: CheckStatus) -> Color {
        switch status {
        case .pending: return .secondary
        case .checking: return .blue
        case .pass: return .green
        case .warning: return .orange
        case .fail: return .red
        }
    }
}

private struct ReportItemRow: View {
    let item: CheckItem
    let onToggle: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            if item.status.isProblem && !item.fixDescription.isEmpty {
                Button {
                    onToggle()
                } label: {
                    Image(systemName: item.selected ? "checkmark.square.fill" : "square")
                        .foregroundColor(item.selected ? .blue : .secondary)
                        .font(.system(size: 14))
                }
                .buttonStyle(BorderlessButtonStyle())
            } else {
                Rectangle()
                    .fill(Color.clear)
                    .frame(width: 14)
            }

            Image(systemName: item.status.icon)
                .font(.caption)
                .foregroundColor(statusColor)
                .frame(width: 16)

            VStack(alignment: .leading, spacing: 2) {
                Text(item.name)
                    .font(.callout)
                if case .pass(let detail) = item.status {
                    Text(detail)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                } else if !item.status.label.isEmpty {
                    HStack(spacing: 4) {
                        Text(item.status.label)
                            .font(.caption2)
                            .fontWeight(.medium)
                            .foregroundColor(statusColor)
                        if case .fail(let detail) = item.status {
                            Text(detail)
                                .font(.caption2)
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                        } else if case .warning(let detail) = item.status {
                            Text(detail)
                                .font(.caption2)
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                        }
                    }
                }
                if item.selected, !item.fixDescription.isEmpty {
                    Text("修复方案: \(item.fixDescription)")
                        .font(.caption2)
                        .foregroundColor(.blue)
                        .padding(.top, 2)
                }
            }

            Spacer()
        }
        .padding(.horizontal)
        .padding(.vertical, 6)
        .background(Color(NSColor.controlBackgroundColor).opacity(0.3))
    }

    private var statusColor: Color {
        switch item.status {
        case .pass: return .green
        case .warning: return .orange
        case .fail: return .red
        case .checking: return .blue
        case .pending: return .secondary
        }
    }
}

#Preview {
    MainPanelView()
}
