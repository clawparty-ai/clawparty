import SwiftUI

struct LLMConfigView: View {
    @State private var agents: [AgentInfo] = []
    @State private var selectedAgent: AgentInfo?
    @State private var agentConfig = AgentLLMConfig()
    @State private var showSavedAlert = false
    @State private var saveError = ""
    @State private var showError = false
    @State private var isLoading = true

    let initialAgent: AgentInfo?

    init(initialAgent: AgentInfo? = nil) {
        self.initialAgent = initialAgent
    }

    let providers = [
        "openai",
        "openrouter",
        "anthropic",
        "gemini",
        "ollama",
        "deepseek",
        "groq",
        "zai",
        "glm",
        "xai",
        "moonshot",
        "opencode-go"
    ]

    let models = [
        "gpt-4o-mini",
        "gpt-4o",
        "gpt-4-turbo",
        "claude-3-5-sonnet",
        "claude-3-opus",
        "gemini-pro",
        "deepseek-chat",
        "deepseek-v4-flash",
        "llama3",
        "glm-4"
    ]

    var body: some View {
        VStack(spacing: 0) {
            if isLoading {
                ProgressView("正在加载 Agents...")
                    .padding()
            } else if agents.isEmpty {
                VStack(spacing: 16) {
                    Image(systemName: "person.3")
                        .font(.system(size: 48))
                        .foregroundColor(.secondary)
                    Text("没有找到 Agent")
                        .font(.headline)
                        .foregroundColor(.secondary)
                    Text("请先启动 ClawParty 创建 Agent")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding()
            } else {
                // Agent 选择器
                HStack {
                    Text("选择 Agent:")
                        .font(.headline)
                    Picker("", selection: $selectedAgent) {
                        Text("请选择").tag(nil as AgentInfo?)
                        ForEach(agents) { agent in
                            Text(agent.displayName)
                                .tag(agent as AgentInfo?)
                        }
                    }
                    .pickerStyle(MenuPickerStyle())
                    .frame(width: 250)
                    .onChange(of: selectedAgent) { newAgent in
                        if let agent = newAgent {
                            agentConfig = ConfigManager.shared.loadAgentConfig(agent: agent)
                        }
                    }

                    Spacer()
                }
                .padding()

                Divider()

                if selectedAgent != nil {
                    configForm
                } else {
                    Spacer()
                    Text("请选择一个 Agent 进行配置")
                        .foregroundColor(.secondary)
                    Spacer()
                }
            }

            Spacer()

            HStack {
                Button("刷新列表") {
                    loadAgents()
                }
                .buttonStyle(BorderlessButtonStyle())

                Spacer()

                Button("保存配置") {
                    saveConfig()
                }
                .buttonStyle(BorderedProminentButtonStyle())
                .controlSize(.large)
                .disabled(selectedAgent == nil)
            }
            .padding()
        }
        .frame(width: 520, height: 500)
        .onAppear {
            loadAgents()
        }
        .alert("配置已保存", isPresented: $showSavedAlert) {
            Button("确定", role: .cancel) {}
        }
        .alert("保存失败", isPresented: $showError) {
            Button("确定", role: .cancel) {}
        } message: {
            Text(saveError)
        }
    }

    private var configForm: some View {
        Form {
            Section(header: Text("提供商设置").font(.headline)) {
                Picker("Provider", selection: $agentConfig.provider) {
                    Text("请选择").tag("")
                    ForEach(providers, id: \.self) { provider in
                        Text(provider).tag(provider)
                    }
                }

                HStack {
                    Picker("Model", selection: $agentConfig.model) {
                        Text("请选择").tag("")
                        ForEach(models, id: \.self) { model in
                            Text(model).tag(model)
                        }
                    }

                    TextField("或自定义", text: $agentConfig.model)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                        .frame(width: 180)
                }
            }

            Section(header: Text("API 设置").font(.headline)) {
                SecureField("API Key", text: $agentConfig.apiKey)
                    .textFieldStyle(RoundedBorderTextFieldStyle())

                TextField("API URL (可选)", text: $agentConfig.apiUrl)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .placeholder(when: agentConfig.apiUrl.isEmpty) {
                        Text("例如: https://api.openai.com/v1").foregroundColor(.gray)
                    }
            }

            Section(header: Text("参数设置").font(.headline)) {
                HStack {
                    Text("Temperature: \(String(format: "%.1f", agentConfig.temperature))")
                    Spacer()
                    Slider(value: $agentConfig.temperature, in: 0.0...2.0, step: 0.1)
                        .frame(width: 200)
                }

                HStack {
                    Text("Timeout")
                    Spacer()
                    TextField("秒", value: $agentConfig.timeoutSecs, format: .number)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                        .frame(width: 80)
                }
            }
        }
        .padding(.horizontal)
    }

    private func loadAgents() {
        isLoading = true
        DispatchQueue.global().async {
            let loadedAgents = ConfigManager.shared.loadAgents()
            DispatchQueue.main.async {
                self.agents = loadedAgents
                self.isLoading = false

                // 如果有初始 agent，尝试选中
                if let initial = self.initialAgent {
                    if let match = loadedAgents.first(where: { $0.agentName == initial.agentName }) {
                        self.selectedAgent = match
                        self.agentConfig = ConfigManager.shared.loadAgentConfig(agent: match)
                    } else if let first = loadedAgents.first {
                        self.selectedAgent = first
                        self.agentConfig = ConfigManager.shared.loadAgentConfig(agent: first)
                    }
                } else if let first = loadedAgents.first {
                    self.selectedAgent = first
                    self.agentConfig = ConfigManager.shared.loadAgentConfig(agent: first)
                }
            }
        }
    }

    private func saveConfig() {
        guard let agent = selectedAgent else { return }

        if ConfigManager.shared.saveAgentConfig(agent: agent, config: agentConfig) {
            showSavedAlert = true
        } else {
            saveError = "无法保存到 \(agent.configPath)"
            showError = true
        }
    }
}

// Placeholder 扩展
extension View {
    func placeholder<Content: View>(
        when shouldShow: Bool,
        alignment: Alignment = .leading,
        @ViewBuilder placeholder: () -> Content
    ) -> some View {
        ZStack(alignment: alignment) {
            placeholder().opacity(shouldShow ? 1 : 0)
            self
        }
    }
}
