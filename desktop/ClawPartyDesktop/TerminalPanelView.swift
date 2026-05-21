import SwiftUI
import SwiftTerm

/// 终端面板：每个 Agent 有独立的 terminal view，切换时只显示/隐藏，不销毁
struct TerminalPanelView: View {
    @Binding var selectedAgent: AgentInfo?
    let onConfig: () -> Void
    let onClose: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // 标题栏
            HStack {
                HStack(spacing: 6) {
                    Image(systemName: "terminal")
                        .foregroundColor(.accentColor)
                    if let agent = selectedAgent {
                        VStack(alignment: .leading, spacing: 1) {
                            Text(agent.displayName)
                                .font(.headline)
                            Text(workspacePath(for: agent))
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                        }
                    } else {
                        Text("终端")
                            .font(.headline)
                    }
                }

                Spacer()

                Button {
                    onConfig()
                } label: {
                    Label("配置", systemImage: "gearshape")
                        .font(.caption)
                }
                .buttonStyle(BorderedButtonStyle())
                .controlSize(.small)
                .help("编辑 Agent 配置")

                Button {
                    onClose()
                } label: {
                    Image(systemName: "xmark.circle")
                        .font(.title3)
                        .foregroundColor(.secondary)
                }
                .buttonStyle(BorderlessButtonStyle())
                .help("关闭终端，返回日志")
            }
            .padding(.horizontal)
            .padding(.vertical, 8)

            Divider()

            // 终端视图容器
            TerminalContainerView(selectedAgent: $selectedAgent)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private func workspacePath(for agent: AgentInfo) -> String {
        let configDir = (agent.configPath as NSString).deletingLastPathComponent
        return "\(configDir)/workspace"
    }
}

// MARK: - Terminal Container ( manages visibility of multiple terminals )

struct TerminalContainerView: NSViewRepresentable {
    @Binding var selectedAgent: AgentInfo?

    func makeNSView(context: Context) -> NSView {
        let container = NSView()
        container.wantsLayer = true
        return container
    }

    func updateNSView(_ container: NSView, context: Context) {
        guard let agent = selectedAgent else { return }

        let manager = TerminalManager.shared
        let termView = manager.terminal(for: agent)

        // 如果当前 container 的子视图已经是这个 term，不需要做任何事
        if container.subviews.first === termView {
            return
        }

        // 移除之前显示的其他 terminal
        for subview in container.subviews {
            subview.removeFromSuperview()
        }

        // 添加新的 terminal，使用 Auto Layout 填满容器
        termView.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(termView)

        NSLayoutConstraint.activate([
            termView.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            termView.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            termView.topAnchor.constraint(equalTo: container.topAnchor),
            termView.bottomAnchor.constraint(equalTo: container.bottomAnchor)
        ])

        // 确保新显示的 terminal 能正确响应焦点和重绘
        DispatchQueue.main.async {
            termView.needsDisplay = true
            container.window?.makeFirstResponder(termView)
        }
    }
}
