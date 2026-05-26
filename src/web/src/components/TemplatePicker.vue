<script setup>
import { ref, computed, watch, inject } from 'vue'
import { templateService } from '../services/templateService'

const openclawAgents = inject('openclawAgents', ref([]))

const props = defineProps({
  show: Boolean,
  source: { type: String, default: 'local' },
  installedAgentIds: { type: Array, default: () => [] },
  mode: { type: String, default: 'openclaw' }, // 'openclaw' | 'zagent'
})

const emit = defineEmits(['close', 'installed', 'created', 'open-main-chat', 'send-messages'])

const industries = ref([])
const selectedIndustry = ref(null)
const loading = ref(false)
const installing = ref({})
const installAllLoading = ref(false)
const error = ref('')
const editingAgent = ref(null)
const editorContent = ref('')
const agentName = ref('')
const showDuplicateConfirm = ref(false)
const showModelConfig = ref(false)
const tplProvider = ref('')
const tplApiEndpoint = ref('')
const tplApiKey = ref('')
const tplModel = ref('')

watch(() => props.show, async (val) => {
  if (val) {
    selectedIndustry.value = null
    error.value = ''
    await loadTemplates()
  }
})

const isZAgentMode = computed(() => props.mode === 'zagent')

const modalTitle = computed(() => {
  if (isZAgentMode.value) return '从模板创建 zAgent'
  return props.source === 'local' ? '添加本地 Agent' : '添加共享 Agent'
})

const loadTemplates = async () => {
  loading.value = true
  error.value = ''
  try {
    const response = props.source === 'local'
      ? await templateService.getLocalTemplates()
      : await templateService.getSharedTemplates()
    const data = response.data
    industries.value = Array.isArray(data.industries) ? data.industries : []
    if (industries.value.length > 0) {
      selectedIndustry.value = industries.value[0]
    }
  } catch (e) {
    console.error('[TemplatePicker] Failed to load templates:', e)
    error.value = 'Failed to load templates'
  } finally {
    loading.value = false
  }
}

const agentsWithStatus = computed(() => {
  if (!selectedIndustry.value) return []
  return selectedIndustry.value.agents.map(agent => ({
    ...agent,
    installed: props.installedAgentIds.includes(agent.slug),
  }))
})

const notInstalledCount = computed(() => {
  if (isZAgentMode.value) return agentsWithStatus.value.length
  return agentsWithStatus.value.filter(a => !a.installed).length
})

const handleInstallAll = async () => {
  if (installAllLoading.value || notInstalledCount.value === 0) return
  installAllLoading.value = true
  error.value = ''

  if (isZAgentMode.value) {
    // zAgent mode: create all from templates using default config
    const { zagentService: zs } = await import('../services/chatService')
    let gcRes = null
    try { gcRes = await zs.getGlobalConfig() } catch {}
    const llm = gcRes?.data?.llm || {}
    const agents = agentsWithStatus.value
    for (let idx = 0; idx < agents.length; idx++) {
      const ag = agents[idx]
      try {
        const res = await zs.createAgent({
          agent_name: ag.slug,
          display_name: ag.name,
          soul_content: ag.systemPrompt || '',
          template_source: {
            industry: ag.industrySlug,
            slug: ag.slug,
            shared: props.source === 'shared'
          },
          provider: llm.provider || null,
          api_endpoint: llm.api_endpoint || null,
          api_key: llm.api_key || null,
          model: llm.model || null
        })
        const actualName = res.data?.agent_name || ag.slug
        try { await zs.startAgent(actualName) } catch {}
      } catch (e) {
        console.error('[ZAgentTemplate] Failed to create', ag.slug, e)
      }
    }
    emit('installed')
    emit('close')
    installAllLoading.value = false
    return
  }

  // openclaw mode
  const notInstalledAgents = agentsWithStatus.value.filter(a => !a.installed)
  const messageTexts = notInstalledAgents.map(agent =>
    `帮我创建一个agent，agent-id使用拼音'${agent.slug}'，agent-name使用中文名'${agent.name}'，创建完成后，把它加入到 main 的 subagents.allowAgents，他的soul.md是：\n\n${agent.systemPrompt || ''}`
  )
  emit('send-messages', messageTexts)
  emit('open-main-chat')
  emit('close')
  setTimeout(async () => {
    try {
      const { openclawService } = await import('../services/chatService')
      await openclawService.getAgents()
      setTimeout(async () => {
        const response = await openclawService.getAgents()
        openclawAgents.value = response.data || []
      }, 500)
    } catch (e) {
      console.error('[TemplatePicker] Failed to refresh agents:', e)
    }
  }, 3000)
  installAllLoading.value = false
}

const handleInstall = async (agent) => {
  if (!isZAgentMode.value && (agent.installed || installing.value[agent.slug])) return
  if (isZAgentMode.value && installing.value[agent.slug]) return

  if (isZAgentMode.value) {
    installing.value[agent.slug] = true
    error.value = ''
    try {
      const { zagentService: zs } = await import('../services/chatService')
      let gcRes = null
      try { gcRes = await zs.getGlobalConfig() } catch {}
      const llm = gcRes?.data?.llm || {}

      // Use user-provided model config if any field is filled, otherwise use global config
      const hasCustomConfig = tplProvider.value || tplApiEndpoint.value || tplApiKey.value || tplModel.value
      const res = await zs.createAgent({
        agent_name: agent.slug,
        display_name: agentName.value || agent.name,
        soul_content: editorContent.value,
        template_source: {
          industry: agent.industrySlug,
          slug: agent.slug,
          shared: props.source === 'shared'
        },
        provider: hasCustomConfig ? (tplProvider.value || null) : (llm.provider || null),
        api_endpoint: hasCustomConfig ? (tplApiEndpoint.value || null) : (llm.api_endpoint || null),
        api_key: hasCustomConfig ? (tplApiKey.value || null) : (llm.api_key || null),
        model: hasCustomConfig ? (tplModel.value || null) : (llm.model || null)
      })
      const actualName = res.data?.agent_name || agent.slug
      try { await zs.startAgent(actualName) } catch (e) {
        console.warn('[ZAgentTemplate] Start failed, may need manual start:', e)
      }
      emit('created', {
        agentName: actualName,
        industrySlug: agent.industrySlug,
        agentSlug: agent.slug,
        source: props.source,
        displayName: agentName.value || agent.name
      })
      emit('close')
    } catch (e) {
      console.error('[TemplatePicker] Failed to create zAgent from template:', e)
      error.value = '创建失败: ' + (e.message || e)
    } finally {
      installing.value[agent.slug] = false
    }
    return
  }

  const message = `帮我创建一个agent，agent-id使用拼音，agent-name使用中文名'${agentName.value}'，把它加入到 main 的 subagents.allowAgents 中，他的soul.md是：\n\n${editorContent.value}`
  emit('send-messages', [message])
  emit('open-main-chat')
  emit('close')
}

const handleSelect = (agent) => {
  editingAgent.value = agent
  editorContent.value = agent.systemPrompt || ''
  agentName.value = agent.name || ''
}

const handleCancelEdit = () => {
  editingAgent.value = null
  editorContent.value = ''
  agentName.value = ''
  showModelConfig.value = false
  tplProvider.value = ''
  tplApiEndpoint.value = ''
  tplApiKey.value = ''
  tplModel.value = ''
}

const handleClose = () => {
  editingAgent.value = null
  editorContent.value = ''
  agentName.value = ''
  error.value = ''
  showModelConfig.value = false
  tplProvider.value = ''
  tplApiEndpoint.value = ''
  tplApiKey.value = ''
  tplModel.value = ''
  emit('close')
}

const handleCancelDuplicate = () => {
  showDuplicateConfirm.value = false
  editingAgent.value = null
  editorContent.value = ''
  agentName.value = ''
}

const handleBackToEdit = () => {
  showDuplicateConfirm.value = false
}
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="modal-backdrop" @click.self="handleClose">
      <div class="modal-dialog template-picker">
        <div class="modal-header">
          <span class="modal-title">{{ modalTitle }}</span>
          <button class="modal-close" @click="handleClose">✕</button>
        </div>

        <div v-if="loading" class="template-loading">Loading templates...</div>
        <div v-else-if="error" class="template-error">{{ error }}</div>
        <template v-else>
          <!-- Industry cards -->
          <div v-if="industries.length > 0" class="template-cards">
            <div
              v-for="ind in industries"
              :key="ind.slug"
              class="template-card"
              :class="{ active: selectedIndustry?.slug === ind.slug }"
              @click="selectedIndustry = ind"
            >
              <span class="card-name">{{ ind.name }}</span>
              <span class="card-count">{{ ind.agents.length }} 个助手</span>
            </div>
          </div>

          <!-- Agent list -->
          <div v-if="selectedIndustry && !editingAgent" class="template-agents">
            <div class="agents-header">
              <span class="agents-title">{{ selectedIndustry.name }}</span>
              <span class="agents-count">({{ agentsWithStatus.length }})</span>
              <button
                class="install-all-btn"
                :disabled="installAllLoading || notInstalledCount === 0"
                @click="handleInstallAll"
              >
                {{ installAllLoading ? '安装中...' : '一键全安装' }}
              </button>
            </div>
            <div
              v-for="agent in agentsWithStatus"
              :key="agent.slug"
              class="agent-item"
            >
              <div class="agent-info">
                <span class="agent-emoji">{{ agent.emoji }}</span>
                <div class="agent-text">
                  <span class="agent-name">{{ agent.name }}</span>
                  <span class="agent-desc">{{ agent.description }}</span>
                </div>
              </div>
              <button
                class="install-btn"
                :class="{ installed: !isZAgentMode && agent.installed, installing: installing[agent.slug] }"
                :disabled="(!isZAgentMode && agent.installed) || installing[agent.slug]"
                @click="handleSelect(agent)"
              >
                <span v-if="!isZAgentMode && agent.installed">已安装</span>
                <span v-else>选择</span>
              </button>
            </div>
          </div>

          <!-- Editor -->
          <div v-if="editingAgent" class="template-editor">
            <div class="editor-header">
              <span class="editor-title">{{ editingAgent.name }} - SOUL.md</span>
              <button class="editor-close" @click="handleCancelEdit">✕</button>
            </div>
            <div class="agent-name-input">
              <label>Agent Name</label>
              <input
                v-model="agentName"
                type="text"
                placeholder="Enter agent name..."
              />
            </div>
            <textarea
              v-model="editorContent"
              class="editor-textarea"
              placeholder="Edit SOUL.md content..."
            ></textarea>

            <!-- Collapsible model config -->
            <div class="model-config-section">
              <button class="model-config-toggle" @click="showModelConfig = !showModelConfig">
                <span class="toggle-icon">{{ showModelConfig ? '▾' : '▸' }}</span>
                模型配置（可选，留空使用全局配置）
              </button>
              <div v-if="showModelConfig" class="model-config-fields">
                <div class="tpl-form-row">
                  <label class="tpl-label">Provider</label>
                  <select v-model="tplProvider" class="tpl-input tpl-select">
                    <option value="">-- 使用全局配置 --</option>
                    <option value="custom">OpenAI 兼容（自定义端点）</option>
                    <option value="openai">OpenAI</option>
                    <option value="anthropic">Anthropic</option>
                    <option value="qwen">阿里云通义</option>
                    <option value="moonshot">月之暗面 Kimi</option>
                    <option value="doubao">字节豆包</option>
                    <option value="deepseek">DeepSeek</option>
                    <option value="ollama">本地 Ollama</option>
                  </select>
                </div>
                <div v-if="tplProvider === 'custom' || tplProvider === 'ollama'" class="tpl-form-row">
                  <label class="tpl-label">API 端点</label>
                  <input v-model="tplApiEndpoint" class="tpl-input" :placeholder="tplProvider === 'ollama' ? 'http://localhost:11434' : 'https://your-endpoint/v1'" />
                </div>
                <div class="tpl-form-row">
                  <label class="tpl-label">API Key</label>
                  <input v-model="tplApiKey" type="password" class="tpl-input" placeholder="sk-..." autocomplete="new-password" />
                </div>
                <div class="tpl-form-row">
                  <label class="tpl-label">模型名称</label>
                  <input v-model="tplModel" class="tpl-input" placeholder="gpt-4o-mini" />
                </div>
              </div>
            </div>

            <div class="editor-actions">
              <button class="cancel-btn" @click="handleCancelEdit">取消</button>
              <button
                class="confirm-btn"
                :class="{ installing: installing[editingAgent.slug] }"
                :disabled="installing[editingAgent.slug]"
                @click="handleInstall(editingAgent)"
              >
                {{ installing[editingAgent.slug] ? '安装中...' : '安装' }}
              </button>
            </div>
          </div>

          <!-- Duplicate Agent Confirm Dialog -->
          <div v-if="showDuplicateConfirm" class="confirm-dialog">
            <div class="confirm-content">
              <p>已经有同名 agent 安装，是否用新的名字安装新的 agent？</p>
              <div class="confirm-actions">
                <button class="cancel-btn" @click="handleCancelDuplicate">不再安装</button>
                <button class="confirm-btn" @click="handleBackToEdit">返回并修改名字</button>
              </div>
            </div>
          </div>

          <div v-if="industries.length === 0" class="template-empty">
            <div class="template-empty-title">No templates found</div>
            <div class="template-empty-hint">
              {{ source === 'local' ? 'Place templates in ~/.clawparty/.agent-template/' : 'No shared templates available on the mesh' }}
            </div>
          </div>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-dialog {
  background: #ffffff;
  border-radius: 12px;
  width: 75vw;
  max-width: 75vw;
  height: 75vh;
  max-height: 75vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  padding: 18px 20px 14px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
  flex-shrink: 0;
}

.modal-title {
  flex: 1;
  color: #333333;
  font-size: 17px;
  font-weight: 700;
}

.modal-close {
  width: 28px;
  height: 28px;
  background: transparent;
  border: none;
  color: rgba(0, 0, 0, 0.5);
  font-size: 16px;
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.1s, color 0.1s;
}

.modal-close:hover {
  background: rgba(0, 0, 0, 0.1);
  color: #333333;
}



.template-loading {
  padding: 40px 20px;
  text-align: center;
  color: rgba(0, 0, 0, 0.5);
  font-size: 14px;
}

.template-error {
  padding: 16px 20px;
  color: #e01e5a;
  font-size: 13px;
  text-align: center;
}

.template-cards {
  display: flex;
  gap: 10px;
  padding: 16px;
  overflow-x: auto;
  flex-shrink: 0;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
}

.template-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  min-width: 90px;
  padding: 12px 10px;
  background: rgba(0, 0, 0, 0.04);
  border-radius: 10px;
  cursor: pointer;
  transition: background 0.15s, box-shadow 0.15s;
  border: 2px solid transparent;
}

.template-card:hover {
  background: rgba(0, 0, 0, 0.08);
}

.template-card.active {
  background: rgba(64, 149, 254, 0.1);
  border-color: #4095fe;
}

.card-emoji {
  font-size: 24px;
  margin-bottom: 6px;
}

.card-name {
  font-size: 13px;
  font-weight: 600;
  color: #333;
  text-align: center;
}

.card-count {
  font-size: 11px;
  color: rgba(0, 0, 0, 0.4);
  margin-top: 2px;
}

.template-agents {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.agents-header {
  display: flex;
  align-items: center;
  padding: 10px 16px;
  background: rgba(0, 0, 0, 0.06);
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
  position: sticky;
  top: 0;
  z-index: 1;
}

.agents-title {
  font-size: 14px;
  font-weight: 700;
  color: #333;
}

.agents-count {
  font-size: 12px;
  color: rgba(0, 0, 0, 0.4);
  margin-left: 6px;
  flex: 1;
}

.install-all-btn {
  padding: 5px 12px;
  background: var(--slack-green);
  border: none;
  border-radius: 6px;
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s;
  flex-shrink: 0;
}

.install-all-btn:hover:not(:disabled) {
  opacity: 0.85;
}

.install-all-btn:disabled {
  background: rgba(0, 0, 0, 0.15);
  color: rgba(0, 0, 0, 0.4);
  cursor: not-allowed;
}

.agent-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  margin: 0 8px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
  transition: background 0.1s;
}

.agent-item:last-child {
  border-bottom: none;
}

.agent-item:hover {
  background: rgba(0, 0, 0, 0.04);
}

.agent-info {
  display: flex;
  align-items: flex-start;
  flex: 1;
  min-width: 0;
}

.agent-emoji {
  font-size: 20px;
  margin-right: 10px;
  flex-shrink: 0;
  margin-top: 2px;
}

.agent-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.agent-name {
  font-size: 14px;
  font-weight: 600;
  color: #333;
}

.agent-desc {
  font-size: 12px;
  color: rgba(0, 0, 0, 0.5);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.install-btn {
  padding: 5px 14px;
  background: var(--slack-green);
  border: none;
  border-radius: 6px;
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s;
  flex-shrink: 0;
  margin-left: 10px;
}

.install-btn:hover:not(:disabled) {
  opacity: 0.85;
}

.install-btn.installed {
  background: rgba(0, 0, 0, 0.1);
  color: rgba(0, 0, 0, 0.5);
  cursor: default;
}

.install-btn.installing {
  opacity: 0.5;
  cursor: not-allowed;
}

.install-btn:disabled {
  cursor: not-allowed;
}

.template-empty {
  padding: 40px 20px;
  text-align: center;
}

.template-empty-title {
  font-size: 15px;
  font-weight: 600;
  color: rgba(0, 0, 0, 0.5);
  margin-bottom: 6px;
}

.template-empty-hint {
  font-size: 13px;
  color: rgba(0, 0, 0, 0.35);
}

.template-editor {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 16px;
  overflow: hidden;
}

.editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
  flex-shrink: 0;
}

.editor-title {
  font-size: 15px;
  font-weight: 600;
  color: #333;
}

.editor-close {
  width: 28px;
  height: 28px;
  background: transparent;
  border: none;
  color: rgba(0, 0, 0, 0.5);
  font-size: 16px;
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.editor-close:hover {
  background: rgba(0, 0, 0, 0.1);
  color: #333;
}

.agent-name-input {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 8px;
}

.agent-name-input label {
  font-size: 12px;
  font-weight: 600;
  color: #666;
}

.agent-name-input input {
  padding: 8px 12px;
  border: 1px solid rgba(0, 0, 0, 0.15);
  border-radius: 6px;
  font-size: 14px;
}

.agent-name-input input:focus {
  outline: none;
  border-color: #4095fe;
}

.editor-textarea {
  flex: 1;
  min-height: 80px;
  width: 100%;
  padding: 12px;
  border: 1px solid rgba(0, 0, 0, 0.15);
  border-radius: 8px;
  font-size: 13px;
  font-family: monospace;
  line-height: 1.5;
  resize: none;
  background: #fafafa;
  color: #333;
}

.editor-textarea:focus {
  outline: none;
  border-color: #4095fe;
  background: #fff;
}

.editor-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 12px;
  flex-shrink: 0;
}

.cancel-btn {
  padding: 8px 16px;
  background: transparent;
  border: 1px solid rgba(0, 0, 0, 0.2);
  border-radius: 6px;
  font-size: 13px;
  color: #666;
  cursor: pointer;
}

.cancel-btn:hover {
  background: rgba(0, 0, 0, 0.05);
}

.confirm-btn {
  padding: 8px 20px;
  background: var(--slack-green);
  border: none;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 600;
  color: #fff;
  cursor: pointer;
}

.confirm-btn:hover:not(:disabled) {
  opacity: 0.85;
}

.confirm-btn.installing {
  opacity: 0.5;
  cursor: not-allowed;
}

.confirm-dialog {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.confirm-content {
  background: white;
  border-radius: 8px;
  padding: 24px;
  max-width: 400px;
  text-align: center;
}

.confirm-content p {
  margin: 0 0 20px;
  font-size: 14px;
  color: #333;
}

.confirm-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
}

.model-config-section {
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 8px;
  overflow: hidden;
  margin-top: 8px;
  flex-shrink: 0;
}

.model-config-toggle {
  width: 100%;
  background: rgba(102, 126, 234, 0.06);
  border: none;
  padding: 8px 12px;
  text-align: left;
  font-size: 12px;
  font-weight: 600;
  color: #555;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: background 0.15s;
}

.model-config-toggle:hover {
  background: rgba(102, 126, 234, 0.12);
}

.toggle-icon {
  font-size: 10px;
  color: #667eea;
}

.model-config-fields {
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: #fafafa;
}

.tpl-form-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tpl-label {
  font-size: 12px;
  font-weight: 600;
  color: #666;
  width: 72px;
  flex-shrink: 0;
}

.tpl-input {
  flex: 1;
  padding: 6px 10px;
  border: 1px solid rgba(0, 0, 0, 0.15);
  border-radius: 6px;
  font-size: 13px;
  background: #fff;
  color: #333;
}

.tpl-input:focus {
  outline: none;
  border-color: #4095fe;
}

.tpl-select {
  appearance: none;
  cursor: pointer;
}
</style>
