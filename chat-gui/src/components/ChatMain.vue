<template>
  <main ref="chatMainEl" class="chat-main">
    <ChatHeader
      :chat="chat"
      :openclawSessions="openclawSessions"
      :currentUserName="currentUserName"
      :showBackButton="showBackButton"
      :showTaskButton="chat.isZeroClaw || chat.isGroupChat"
      :taskPanelVisible="showTaskPanel"
      :showWebShareButton="chat.isZeroClaw || chat.isGroupChat"
      :webSharePanelVisible="showWebSharePanel"
      :showWikiButton="chat.isZeroClaw"
      :wikiPanelVisible="showWikiPanel"
      :showRadarButton="chat.isZeroClaw || chat.isGroupChat"
      :radarPanelVisible="showRadarPanel"
      @switchSession="$emit('switchSession', $event)"
      @deleteGroup="$emit('deleteGroup', $event)"
      @leaveGroup="$emit('leaveGroup', $event)"
      @back="$emit('back')"
      @download="handleDownload"
      @download-md="handleDownloadMd"
      @download-pdf="handleDownloadPdf"
      @reload="fetchMessages"
      @toggleTaskPanel="showTaskPanel = !showTaskPanel"
      @toggleWebSharePanel="showWebSharePanel = !showWebSharePanel"
      @toggleWikiPanel="showWikiPanel = !showWikiPanel"
      @toggleRadarPanel="showRadarPanel = !showRadarPanel"
      @showMembers="showMembersPanel = !showMembersPanel"
    />
    <div class="chat-body-wrapper">
      <div class="chat-content" :class="{ 'with-members-panel': showMembersPanel }">
    <WebSharePanel
      v-if="(chat.isZeroClaw || chat.isGroupChat) && showWebSharePanel"
      :agentName="agentName || chat.display_name || chat.agent_name || chat.ownerAgent"
      :files="webShareFiles"
      :expanded="webSharePanelExpanded"
      :initialHeight="taskPanelInitialHeight"
      :refreshing="isWebShareRefreshing"
      @toggle="webSharePanelExpanded = !webSharePanelExpanded"
      @refresh="loadWebShareFiles"
      @uploaded="handleWebShareUploaded"
    />
    <TaskPanel
      v-if="(chat.isZeroClaw || chat.isGroupChat) && showTaskPanel"
      :agentName="agentName || chat.display_name || chat.agent_name"
      :tasks="tasks"
      :expanded="taskPanelBodyExpanded"
      :initialHeight="taskPanelInitialHeight"
      :refreshing="isTaskRefreshing"
      :pendingChanges="pendingTaskChanges"
      :refreshLogs="refreshLogs"
      :kanbanConfig="kanbanConfig"
      @toggle="taskPanelBodyExpanded = !taskPanelBodyExpanded"
      @refresh="handleTaskRefresh"
      @confirmChange="handleConfirmTaskChange"
      @refreshTimeoutChange="refreshTimeout = $event"
      @reuse="handleTaskReuse"
      @generatePrompt="handleGeneratePrompt"
      @createPipelineTask="handleCreatePipelineTask"
      @savePipeline="handleSavePipeline"
      @togglePipelinePanel="handleTogglePipelinePanel"
      @updateKanbanConfig="handleUpdateKanbanConfig"
      @generateChart="handleGenerateChart"
    />
    <WikiPanel
      v-if="chat.isZeroClaw && showWikiPanel"
      :agentName="agentName || chat.display_name || chat.agent_name"
      :expanded="wikiPanelBodyExpanded"
      :initialHeight="taskPanelInitialHeight"
      :refreshing="isWikiRefreshing"
      @toggle="wikiPanelBodyExpanded = !wikiPanelBodyExpanded"
      @refresh="handleWikiRefresh"
    />
    <RadarPanel
      v-if="(chat.isZeroClaw || chat.isGroupChat) && showRadarPanel"
      :agentName="agentName || chat.display_name || chat.agent_name || chat.ownerAgent"
      :expanded="radarPanelExpanded"
      :initialHeight="taskPanelInitialHeight"
      :refreshing="isRadarRefreshing"
      @toggle="radarPanelExpanded = !radarPanelExpanded"
      @refresh="handleRadarRefresh"
      @createTarget="handleRadarCreate"
      @createProbe="handleRadarCreate"
    />
    <div class="messages" ref="messagesContainer" @click="handleMessagesClick">
      <div class="date-divider">
        <span>{{ currentDate }}</span>
      </div>
      <div 
        v-for="(msg, index) in filteredMessages" 
        :key="index"
        class="message"
        :class="{ sent: isMessageSent(msg), typing: msg.isTyping }"
      >
          <div class="message-avatar">
          <div v-if="(chat.isOpenclaw || chat.isZeroClaw) && !isMessageSent(msg) && !msg.isTyping" class="avatar-emoji">
            {{ chat.emoji || '🀄' }}
          </div>
          <div v-else-if="msg.isTyping && (chat.isOpenclaw || chat.isZeroClaw)" class="avatar-emoji">
            {{ chat.emoji || '🀄' }}
          </div>
          <div v-else-if="msg.agentName === 'user' && !isMessageSent(msg)" class="avatar-placeholder group-owner-avatar" :style="{ background: '#611f69' }">
            👑
          </div>
          <div v-else-if="!msg.isTyping" class="avatar-placeholder" :class="{ 'group-agent-avatar': chat.isGroupChat && msg.agentName && msg.agentName !== 'user' }" :style="{ background: getAvatarColor(isMessageSent(msg) ? (currentUserName || 'You') : (msg.agentName || msg.sender || chat.name)) }">
            {{ (isMessageSent(msg) ? (currentUserName || 'You') : (msg.agentName || msg.sender || chat.name))[0].toUpperCase() }}
          </div>
        </div>
        <div class="message-body">
          <div v-if="msg.isTyping && !chat.isGroupChat" class="typing-indicator">
            <span class="typing-dot"></span>
            <span class="typing-dot"></span>
            <span class="typing-dot"></span>
          </div>
          <template v-else>
            <div class="message-header">
              <span class="message-author">
                <span v-if="chat.isGroupChat && msg.agentName && msg.agentName !== 'user'" class="agent-tag" :style="{ background: getAvatarColor(msg.agentName) }">{{ msg.agentName }}</span>
                <span v-else-if="chat.isGroupChat && msg.agentName === 'user'" class="agent-tag owner-tag">👑 owner</span>
                <template v-else>{{ isMessageSent(msg) ? myDisplayNameWithAgent : (msg.sender || chat.name) }}</template>
              </span>
              <span class="message-time">{{ msg.time }}</span>
            </div>
            <div class="message-bubble" :class="{ 'system-hint': msg.isSystemHint }">
              <ConfigTable v-if="msg.isConfigTable" :rows="msg.configRows" :meshName="msg.meshName || meshName" />
              <template v-else>
<div v-if="msg.files && msg.files.length > 0" class="message-images">
  <div v-for="file in msg.files" :key="file.hash" class="attachment-item">
    <img v-if="file.type && file.type.startsWith('image/')"
         :src="file.url"
         :alt="file.name || 'image'"
         class="chat-image"
         loading="lazy"
         @click="openImagePreview(file.url)"
    />
    <span v-else class="file-text">
      文件上传成功，保存在：{{ file.path }}
    </span>
  </div>
</div>
                <div v-if="msg.text" class="message-content" v-html="msg.isHtml ? msg.text : renderMarkdown(msg.text)"></div>
                <!-- Quote preview -->
                <div v-if="msg.quote" class="message-quote">
                  <div class="message-quote-header">
                    <span class="message-quote-author">{{ msg.quote.sender }}</span>
                    <span class="message-quote-time">{{ msg.quote.time }}</span>
                  </div>
                  <div class="message-quote-preview">{{ msg.quote.preview }}</div>
                </div>
              </template>
              <div v-if="msg.isGroupRequest || msg.isPeerRequest" class="group-request-actions">
                <template v-if="(msg.isPeerRequest || msg.isGroupEpRequest) && msg.availableAgents && msg.availableAgents.length > 0">
                  <select v-model="msg.selectedAgent" class="agent-select">
                    <option v-for="a in msg.availableAgents" :key="a" :value="a">
                      {{ a.startsWith('zeroclaw:') ? '🀄 ' + a.substring(9) : a }}
                    </option>
                  </select>
                </template>
                <button class="approve-btn" @click="approveGroupRequest(msg)">Approve</button>
              </div>
            </div>
            <div class="message-actions">
              <button v-if="!msg.isGroupRequest && !msg.isPeerRequest" class="quote-btn" @click="quoteMessage(msg)" title="引用此消息">↩ 引用</button>
              <button v-if="!msg.isGroupRequest && !msg.isPeerRequest && msg.text" class="copy-btn" @click="copyMessage(msg)" title="拷贝此消息">📋 拷贝</button>
            </div>
          </template>
        </div>
      </div>
    </div>
    <HalfAutomationInput
      v-if="peerMode === 'half' && !chat.isOpenclaw"
      ref="halfInputRef"
      :meshName="meshName"
      :peerName="chat.name"
      :sessionId="currentSessionId"
      :initialDraft="halfDraftText"
      :currentMode="peerMode"
      @send="handleHalfSend"
      @draft-updated="handleDraftUpdated"
      @update:peerMode="handlePeerModeChange"
    />
    <MessageInput
      v-else
      :chatName="chat.name"
      :loading="sending"
      :modelValue="modelValue"
      :isOpenclaw="chat.isOpenclaw"
      :agentId="chat.agentId"
      :autoFocus="autoFocus"
      :members="chat.isGroupChat ? [chat.ownerAgent, ...(chat.members || [])] : []"
      :agentGroups="agentGroupChats"
      :peerMode="peerMode"
      :showPeerMode="!chat.isOpenclaw && !!chat.name"
      :quote="quotedMessage"
      :agentStatus="agentStatus || (chat.isZeroClaw ? (chat.status || 'created') : null)"
      :agentErrorMsg="agentErrorMsg || chat.error_msg"
      :agentName="agentName || (chat.isZeroClaw ? (chat.display_name || chat.agent_name) : null)"
      @update:modelValue="$emit('update:modelValue', $event)"
      @send="handleSendWithQuote"
      @send-images="$emit('send-images', $event)"
      @send-files="$emit('send-files', $event)"
      @hash-command="handleHashCommand"
      @update:peerMode="handlePeerModeChange"
      @clear-quote="handleClearQuote"
      @start-agent="$emit('start-agent', $event)"
    />
      </div>
      <!-- Members Panel for group chats -->
      <div class="members-panel" v-if="chat.isGroupChat && showMembersPanel">
        <div class="members-panel-header">
          <span class="members-panel-title">成员管理</span>
          <button class="members-panel-close" @click="showMembersPanel = false">✕</button>
        </div>
        <div class="members-owner">
          <div class="members-label">群主</div>
          <div class="members-item">
            <div class="members-avatar" :style="{ background: getAvatarColor(chat.ownerAgent) }">
              {{ chat.ownerAgent?.[0]?.toUpperCase() || '👑' }}
            </div>
            <span class="members-name">{{ chat.ownerAgent }}</span>
          </div>
        </div>
        <div class="members-list">
          <div class="members-label">群成员</div>
          <div
            v-for="member in (chat.members || [])"
            :key="member"
            class="members-item"
          >
            <div class="members-avatar" :style="{ background: getAvatarColor(member) }">
              {{ member[0].toUpperCase() }}
            </div>
            <span class="members-name">{{ member }}</span>
            <button class="members-remove-btn" title="移除" @click="handleRemoveMember(member)">✕</button>
          </div>
          <div v-if="!(chat.members || []).length" class="members-empty">暂无其他成员</div>
        </div>
        <div class="members-add">
          <div class="members-label">添加成员</div>
          <div class="members-add-list">
            <div
              v-for="agent in availableZAgents"
              :key="agent.agent_name"
              class="members-add-item"
              @click="handleAddMember(agent.agent_name)"
            >
              <div class="members-avatar" :style="{ background: getAvatarColor(agent.agent_name) }">
                {{ agent.agent_name[0].toUpperCase() }}
              </div>
              <span class="members-name">{{ agent.agent_name }}</span>
              <span class="members-add-icon">+</span>
            </div>
            <div v-if="!availableZAgents.length" class="members-empty">无可添加的agent</div>
          </div>
        </div>
      </div>
    </div>
  </main>
</template>

<script setup>
import { ref, watch, nextTick, computed, onUnmounted, onMounted, onBeforeUnmount, inject } from 'vue'
import { marked } from 'marked'
import ChatHeader from './ChatHeader.vue'
import MessageInput from './MessageInput.vue'
import HalfAutomationInput from './HalfAutomationInput.vue'
import ConfigTable from './ConfigTable.vue'
import TaskPanel from './TaskPanel.vue'
import WebSharePanel from './WebSharePanel.vue'
import WikiPanel from './WikiPanel.vue'
import RadarPanel from './RadarPanel.vue'
import { chatService, taskService, kanbanService, ZeroClawWS, zagentService, groupChatService, webshareService, wikiService } from '../services/chatService'
import { radarService } from '../services/radarService'
import { getAvatarColor } from '../utils/avatar'

marked.setOptions({
  breaks: true,
  gfm: true
})

const props = defineProps({
  chat: {
    type: Object,
    required: true
  },
  meshName: {
    type: String,
    default: ''
  },
  currentUserName: {
    type: String,
    default: ''
  },
  sending: {
    type: Boolean,
    default: false
  },
  openclawSessions: {
    type: Array,
    default: () => []
  },
  agentStatus: {
    type: String,
    default: null
  },
  agentErrorMsg: {
    type: String,
    default: null
  },
  agentName: {
    type: String,
    default: null
  },

  modelValue: String,
  isActive: {
    type: Boolean,
    default: true
  },
  showBackButton: {
    type: Boolean,
    default: false
  },
  autoFocus: {
    type: Boolean,
    default: true
  },
  isGroupChat: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['send', 'update:modelValue', 'switchSession', 'deleteGroup', 'leaveGroup', 'back', 'send-images', 'send-files', 'clear-quote'])

const chatMainEl = ref(null)
const messagesContainer = ref(null)
let pollTimer = null
const openclawAgents = inject('openclawAgents', ref([]))
const allGroupChats = inject('groupChats', ref([]))
const resolveEpDisplayName = inject('resolveEpDisplayName', (u) => u)
const peerMode = ref('')  // 'blocked' | 'muted' | 'manual' | 'auto' | 'half'
const currentPeerConfig = ref(null)  // 存储当前 peer 的配置数据（含 autoReplyAgent）
const allPeerConfigs = ref([])  // 存储所有 peer 的配置数据
const halfInputRef = ref(null)
const halfDraftText = ref('')
const currentSessionId = ref('')
const showMembersPanel = ref(false)

// zAgents list is needed for adding members to group chats
const zAgents = inject('zAgents', ref([]))

// Quote feature
const quotedMessage = ref(null)

// Task management (zAgent / Group Chat)
const tasks = ref([])
const kanbanConfig = ref({
  name: '默认看板',
  prompt: '',
  config: {
    charts: [
      { id: 'status', type: 'doughnut', title: '状态分布', enabled: true, prompt: '' },
      { id: 'trend', type: 'line', title: '近7天趋势', enabled: true, prompt: '' },
      { id: 'agent', type: 'bar', title: 'Agent分布', enabled: true, prompt: '' },
      { id: 'duration', type: 'bar', title: '耗时统计', enabled: true, prompt: '' }
    ]
  }
})
const showTaskPanel = ref(false)
const taskPanelBodyExpanded = ref(true)
const taskPanelInitialHeight = ref(180)
const isTaskRefreshing = ref(false)
const pendingTaskChanges = ref([])
const refreshLogs = ref([])
const refreshTimeout = ref(120)
const addRefreshLog = (level, msg) => {
  const t = new Date()
  const hh = t.getHours().toString().padStart(2, '0')
  const mm = t.getMinutes().toString().padStart(2, '0')
  const ss = t.getSeconds().toString().padStart(2, '0')
  refreshLogs.value.push({ time: hh + ':' + mm + ':' + ss, level, msg })
  if (refreshLogs.value.length > 80) refreshLogs.value.shift()
}

// Wiki panel state
const showWikiPanel = ref(false)
const wikiPanelBodyExpanded = ref(true)
const wikiPanelInitialHeight = ref(180)
const isWikiRefreshing = ref(false)

const handleWikiRefresh = async () => {
  const agentName = props.agentName || props.chat.agent_name || props.chat.ownerAgent
  if (!agentName) return
  isWikiRefreshing.value = true
  try {
    await wikiService.refresh(agentName)
  } catch (e) {
    console.error('[Wiki] Refresh failed:', e)
  } finally {
    isWikiRefreshing.value = false
  }
}

// Pipeline management
const showPipelinePanel = ref(false)

const handleTogglePipelinePanel = (visible) => {
  showPipelinePanel.value = visible
}

const handleCreatePipelineTask = async (data) => {
  const agentName = props.agentName || props.chat.agent_name || props.chat.ownerAgent
  if (!agentName) {
    console.error('[Pipeline] No agent name available')
    return
  }

  try {
    const taskId = 'PIPELINE-' + Date.now()
    const pipelineDesc = data.pipeline.map((t, i) => `步骤${i + 1}: ${t.title}`).join(' → ')

    await taskService.createTask({
      task_id: taskId,
      agent_name: agentName,
      group_id: props.chat.isGroupChat ? props.chat.groupId : null,
      title: '流水线任务: ' + pipelineDesc,
      description: data.prompt,
      status: 'pending',
      progress: 0,
      priority: 'normal',
      is_pipeline: true,
      pipeline_definition: data.pipeline.map(t => t.task_id)
    })

    console.log('[Pipeline] Task created:', taskId)
    addRefreshLog('info', '流水线任务已创建: ' + taskId)

    // Send prompt to 0#Agent via chat
    emit('send', data.prompt)

    // Reload tasks
    await loadTasks()
  } catch (err) {
    console.error('[Pipeline] Failed to create task:', err)
    addRefreshLog('error', '创建流水线任务失败: ' + (err.message || err))
  }
}

const handleSavePipeline = async (data) => {
  try {
    await taskService.updateTask(data.taskId, {
      pipeline_definition: data.pipeline_definition,
      description: data.prompt
    })
    addRefreshLog('info', '流水线已更新: ' + data.taskId)
    await loadTasks()
  } catch (err) {
    console.error('[Pipeline] Failed to save pipeline:', err)
    addRefreshLog('error', '保存流水线失败: ' + (err.message || err))
  }
}

// Web Share management
const showWebSharePanel = ref(false)
const webSharePanelExpanded = ref(true)
const webSharePanelHeight = ref(180)
const webShareFiles = ref([])
const isWebShareRefreshing = ref(false)

const loadWebShareFiles = async (path) => {
  if (!props.chat.isZeroClaw && !props.chat.isGroupChat) return
  const agentName = props.agentName || props.chat.agent_name || props.chat.ownerAgent
  if (!agentName) return
  try {
    isWebShareRefreshing.value = true
    const res = await webshareService.getAgentWebshareList(agentName, path)
    if (res.data && res.data.files) {
      webShareFiles.value = res.data.files
    }
  } catch (e) {
    console.error('[WebShare] Failed to load files:', e)
  } finally {
    isWebShareRefreshing.value = false
  }
}

watch(showWebSharePanel, (visible) => {
  if (visible) {
    showTaskPanel.value = false
    showWikiPanel.value = false
    showRadarPanel.value = false
    loadWebShareFiles()
  }
})

watch(showTaskPanel, (visible) => {
  if (visible) {
    showWebSharePanel.value = false
    showWikiPanel.value = false
  }
})

watch(showWikiPanel, (visible) => {
  if (visible) {
    showTaskPanel.value = false
    showWebSharePanel.value = false
    showRadarPanel.value = false
  }
})

// Radar panel state
const showRadarPanel = ref(false)
const radarPanelExpanded = ref(true)
const isRadarRefreshing = ref(false)

const handleRadarRefresh = async () => {
  const agentName = props.agentName || props.chat.agent_name || props.chat.ownerAgent
  if (!agentName) return
  isRadarRefreshing.value = true
  try {
    await radarService.initRadar(agentName)
  } catch (e) {
    console.error('[Radar] Refresh failed:', e)
  } finally {
    isRadarRefreshing.value = false
  }
}

const handleRadarCreate = (draft) => {
  emit('update:modelValue', draft)
}

watch(showRadarPanel, (visible) => {
  if (visible) {
    showTaskPanel.value = false
    showWebSharePanel.value = false
    showWikiPanel.value = false
  }
})

const handleWebShareUploaded = () => {
  // Refresh the file list after successful upload
  setTimeout(() => {
    loadWebShareFiles()
  }, 300)
}

const calcTaskPanelHeight = async () => {
  if (!chatMainEl.value) return
  await nextTick()
  await nextTick()
  const totalH = chatMainEl.value.clientHeight
  if (totalH === 0) {
    setTimeout(calcTaskPanelHeight, 100)
    return
  }
  const headerH = chatMainEl.value.querySelector('.chat-header')?.clientHeight || 56
  const inputH = chatMainEl.value.querySelector('.input-area')?.clientHeight || 0
  const msgAreaH = Math.max(totalH - headerH - inputH, 120)
  taskPanelInitialHeight.value = Math.round(msgAreaH * 0.5)
}

const loadTasks = async () => {
  if (!props.chat.isZeroClaw && !props.chat.isGroupChat) return
  const agentName = props.agentName || props.chat.agent_name || props.chat.ownerAgent
  const groupId = props.chat.isGroupChat ? props.chat.groupId : null
  if (!agentName) return
  try {
    addRefreshLog('info', 'Loading tasks for ' + agentName + (groupId ? ' (group=' + groupId + ')' : ''))
    const res = await taskService.getAgentTasks(agentName, groupId)
    if (res.data && res.data.tasks) {
      tasks.value = res.data.tasks
      addRefreshLog('info', 'Loaded ' + res.data.tasks.length + ' task(s)')
    }
  } catch (e) {
    addRefreshLog('error', 'Failed to load tasks: ' + (e.message || e))
  }
}

const loadKanbanConfig = async () => {
  if (!props.chat.isZeroClaw && !props.chat.isGroupChat) return
  const agentName = props.agentName || props.chat.agent_name || props.chat.ownerAgent
  const groupId = props.chat.isGroupChat ? props.chat.groupId : null
  if (!agentName) return
  try {
    const res = await kanbanService.getKanbanConfig(agentName, groupId)
    if (res.data) {
      kanbanConfig.value = {
        name: res.data.name || '默认看板',
        prompt: res.data.prompt || '',
        config: res.data.config || {
          charts: [
            { id: 'status', type: 'doughnut', title: '状态分布', enabled: true, prompt: '' },
            { id: 'trend', type: 'line', title: '近7天趋势', enabled: true, prompt: '' },
            { id: 'agent', type: 'bar', title: 'Agent分布', enabled: true, prompt: '' },
            { id: 'duration', type: 'bar', title: '耗时统计', enabled: true, prompt: '' }
          ]
        }
      }
    }
  } catch (e) {
    console.error('[ChatMain] Failed to load kanban config:', e)
  }
}

const handleUpdateKanbanConfig = async (config) => {
  const agentName = props.agentName || props.chat.agent_name || props.chat.ownerAgent
  const groupId = props.chat.isGroupChat ? props.chat.groupId : null
  if (!agentName) return
  try {
    const res = await kanbanService.setKanbanConfig(
      agentName,
      groupId,
      config.name,
      config.prompt,
      config.config
    )
    if (res.data) {
      kanbanConfig.value = {
        name: res.data.name,
        prompt: res.data.prompt,
        config: res.data.config
      }
    }
  } catch (e) {
    console.error('[ChatMain] Failed to save kanban config:', e)
  }
}

const handleGenerateChart = async (chartInfo) => {
  // TODO: Implement AI chart generation
  // This would send the chart prompt to 0#Agent and update the chart data
  console.log('[ChatMain] Generate chart:', chartInfo)
}

const handleTaskRefresh = async () => {
  if (isTaskRefreshing.value) {
    addRefreshLog('warn', 'Already in progress, skipping')
    return
  }
  isTaskRefreshing.value = true
  pendingTaskChanges.value = []
  refreshLogs.value = [{ time: '', level: 'info', msg: '==== Task Refresh Started ====' }]

  addRefreshLog('info', 'Loading existing tasks...')
  await loadTasks()

  const msgs = props.chat.messages || []
  const agentName = props.agentName || props.chat.agent_name || props.chat.ownerAgent
  const groupId = props.chat.isGroupChat ? props.chat.groupId : null

  var lastAnalyzed = 0
  try {
    if (agentName) {
      const logRes = await taskService.getAnalysisLog(agentName, groupId)
      if (logRes.data) {
        lastAnalyzed = logRes.data.last_analyzed_at || 0
        addRefreshLog('info', 'Loaded analysis log, lastAnalyzed=' + lastAnalyzed)
      }
    }
  } catch (e) {
    addRefreshLog('warn', 'Failed to load analysis log: ' + (e.message || e))
  }
  if (!lastAnalyzed && props.chat.lastTaskAnalyzedAt) {
    lastAnalyzed = props.chat.lastTaskAnalyzedAt
  }

  let newMessages = msgs.filter(m => m.timestamp != null && m.timestamp >= lastAnalyzed)
  addRefreshLog('info', 'Messages total: ' + msgs.length + ' | new since last analysis: ' + newMessages.length)

  if (newMessages.length === 0) {
    if (msgs.length === 0) {
      addRefreshLog('warn', 'Messages not loaded yet, skip cursor persistence')
      isTaskRefreshing.value = false
      return
    }
    const latestTs = msgs[msgs.length - 1].timestamp
    if (latestTs != null) {
      const newLast = Math.max(lastAnalyzed, latestTs)
      props.chat.lastTaskAnalyzedAt = newLast
      try {
        await taskService.setAnalysisLog(agentName, groupId, newLast)
        addRefreshLog('info', 'Persisted analysis log (no new messages)')
      } catch (e) {}
    }
    addRefreshLog('info', 'No new messages. Done.')
    isTaskRefreshing.value = false
    return
  }

  if (!agentName) {
    addRefreshLog('warn', 'No agentName available. Cannot send to Zerus.')
    props.chat.lastTaskAnalyzedAt = newMessages[newMessages.length - 1].timestamp
    isTaskRefreshing.value = false
    return
  }
  addRefreshLog('info', 'Agent: ' + agentName + (groupId ? ' | group=' + groupId : ''))

  const persistLastAnalyzed = function(ts) {
    props.chat.lastTaskAnalyzedAt = ts
    taskService.setAnalysisLog(agentName, groupId, ts).catch(function(e) {
      addRefreshLog('warn', 'Failed to persist analysis log: ' + (e.message || e))
    })
  }

  try {
    addRefreshLog('info', 'Fetching 0#Agent info...')
    const agentsRes = await zagentService.getAgents()
    const zeroAgent = agentsRes.data.find(function(a) { return a.agent_name === '0#Agent' })
    if (!zeroAgent || !zeroAgent.port) {
      addRefreshLog('error', '0#Agent not found or port missing. Agents count: ' + (agentsRes.data?.length || 0))
      persistLastAnalyzed(newMessages[newMessages.length - 1].timestamp)
      await loadTasks()
      isTaskRefreshing.value = false
      return
    }
    addRefreshLog('info', '0#Agent found on port ' + zeroAgent.port)

    const MAX_MSG_COUNT = 10
    const MAX_MSG_LEN = 300
    const MAX_TASKS_IN_PROMPT = 15

    const recentMsgs = newMessages.slice(-MAX_MSG_COUNT)
    const msgText = recentMsgs.map(m => {
      const role = m.isSent ? 'user' : 'assistant'
      const text = (m.text || '').substring(0, MAX_MSG_LEN)
      return `[${role}] ${m.sender || 'unknown'}: ${text}`
    }).join('\n\n')

    const existingTasks = tasks.value.slice(0, MAX_TASKS_IN_PROMPT)

    const prompt = `[系统指令] 你是 Task Analyst，请分析以下聊天记录，识别新任务和已有任务的状态变更，并为每个任务生成结果概要。\n\n## 当前已有任务${tasks.value.length > MAX_TASKS_IN_PROMPT ? '（仅显示部分）' : ''}\n${existingTasks.map(t => '- ID: ' + t.task_id + ' | 编号: #' + (t.task_number || '?') + ' | 标题: ' + t.title + ' | 状态: ' + t.status + (t.description ? ' | 目标: ' + t.description.substring(0, 80) : '')).join('\n') || '无'}\n\n## 新聊天记录（最近 ${recentMsgs.length} 条，user=用户，assistant=AI）\n${msgText}\n\n## 分析要求\n1. **新任务**：用户或 AI 提到的新待办事项、计划、工作目标\n2. **状态变更**：现有任务在聊天中被提到已完成、失败、取消或进度变化\n3. **结果概要**：为每个已有任务生成一段简短的结果概要——如果任务已完成，写完成总结；如果任务进行中，写当前进展；如果任务待办，写"待开始"\n4. **重用提示词**：为每个新任务生成一个精简的提示词（prompt），该提示词应保留任务的核心意图和关键信息，去除冗余描述，长度控制在 50-150 字，可直接作为新任务的指令使用\n\n## 输出格式（纯 JSON，不要 markdown 代码块，不要解释）\n{\n  "newTasks": [{"title": "...", "description": "...", "status": "pending|running|completed|failed", "progress": 0, "summary": "新任务的目标概要", "prompt": "精简的重用提示词，保留核心意图，50-150字"}],\n  "statusChanges": [{"taskId": "现有任务ID", "newStatus": "...", "newProgress": 100, "reason": "变化原因", "summary": "该任务的结果概要或完成总结"}],\n  "summaries": [{"taskId": "现有任务ID", "summary": "即使状态未变也为每个任务生成结果概要"}]\n}`

    addRefreshLog('info', 'Building prompt... recentMsgs: ' + recentMsgs.length + ' | existingTasks: ' + existingTasks.length + ' | promptLen: ' + prompt.length)

    const sessionId = 'sys-task-refresh-' + Date.now()
    let fullResponse = ''
    let hasResponded = false
    let isFinished = false
    addRefreshLog('info', 'Opening WebSocket session...')

    const finishRefresh = async (label) => {
      if (isFinished) return
      isFinished = true
      addRefreshLog('info', 'Finishing (' + label + ')...')
      await loadTasks()
      isTaskRefreshing.value = false
      addRefreshLog('info', '==== Done (' + label + ') ====')
    }

    const ws = new ZeroClawWS(
      '0#Agent',
      sessionId,
      async function(data) {
        if (data.type === 'chunk') {
          fullResponse += data.content
        } else if (data.type === 'done') {
          hasResponded = true
          addRefreshLog('info', 'AI response received, length: ' + fullResponse.length)
          ws.close()
          try {
            let jsonText = fullResponse.trim()
            if (jsonText.indexOf('```json') >= 0) {
              const start = jsonText.indexOf('```json') + 7
              const end = jsonText.lastIndexOf('```')
              jsonText = jsonText.slice(start, end > start ? end : undefined).trim()
            } else if (jsonText.indexOf('```') >= 0) {
              const start = jsonText.indexOf('```') + 3
              const end = jsonText.lastIndexOf('```')
              jsonText = jsonText.slice(start, end > start ? end : undefined).trim()
            }
            const result = JSON.parse(jsonText)
            const changes = []
            if (result.newTasks && result.newTasks.length > 0) {
              addRefreshLog('info', 'AI detected ' + result.newTasks.length + ' new task(s)')
              for (const t of result.newTasks) {
                changes.push({
                  type: 'create',
                  data: {
                    title: t.title,
                    description: t.description || t.title,
                    status: t.status || 'pending',
                    progress: t.progress !== undefined ? t.progress : 0,
                    prompt: t.prompt || null
                  }
                })
              }
            }
            if (result.statusChanges && result.statusChanges.length > 0) {
              addRefreshLog('info', 'AI detected ' + result.statusChanges.length + ' status change(s)')
              for (const c of result.statusChanges) {
                changes.push({
                  type: 'update',
                  task_id: c.taskId,
                  new_status: c.newStatus,
                  new_progress: c.newProgress,
                  result_summary: c.summary || null
                })
              }
            }
            if (result.summaries && result.summaries.length > 0) {
              addRefreshLog('info', 'AI generated ' + result.summaries.length + ' summary/summaries')
              for (const s of result.summaries) {
                if (s.taskId && s.summary) {
                  changes.push({
                    type: 'summary',
                    task_id: s.taskId,
                    result_summary: s.summary
                  })
                }
              }
            }
            pendingTaskChanges.value = changes
            if (changes.length === 0) {
              addRefreshLog('info', 'No changes detected by AI.')
            } else {
              addRefreshLog('info', 'Batch refreshing ' + changes.length + ' change(s) via tui...')
              const batchRes = await taskService.batchRefresh(
                agentName, groupId,
                newMessages[newMessages.length - 1].timestamp,
                changes
              )
              addRefreshLog('info', 'Batch refresh done: created=' + (batchRes.data?.created || 0) +
                ' updated=' + (batchRes.data?.updated || 0) +
                ' saved=' + (batchRes.data?.tasks_saved || false))
              // Remove "create" changes from pending since batchRefresh already persisted them.
              // Keeping them would show duplicate entries AND trigger extra creates on Confirm.
              pendingTaskChanges.value = changes.filter(function (c) { return c.type !== 'create' })
              if (pendingTaskChanges.value.length === 0) {
                addRefreshLog('info', 'All changes committed, no pending confirmations.')
              } else {
                addRefreshLog('info', pendingTaskChanges.value.length + ' change(s) remain pending confirmation.')
              }
            }
          } catch (e) {
            addRefreshLog('error', 'Failed to parse AI response: ' + (e.message || e))
          }
          persistLastAnalyzed(newMessages[newMessages.length - 1].timestamp)
          finishRefresh('ws-done')
        }
      },
      function() {
        addRefreshLog('info', 'WebSocket connected, sending prompt...')
        ws.sendMessage(prompt)
      },
      function() {
        addRefreshLog('info', 'WebSocket closed.')
      },
      function(error) {
        addRefreshLog('error', 'WebSocket error: ' + (error.message || error))
        persistLastAnalyzed(newMessages[newMessages.length - 1].timestamp)
        finishRefresh('ws-error')
      },
      zeroAgent.port
    )
    ws.connect()

    setTimeout(function() {
      if (!hasResponded) {
        ws.close()
        addRefreshLog('warn', 'Timeout (' + refreshTimeout.value + 's) waiting for 0#Agent response')
        persistLastAnalyzed(newMessages[newMessages.length - 1].timestamp)
        finishRefresh('timeout')
      }
    }, refreshTimeout.value * 1000)
  } catch (e) {
    addRefreshLog('error', 'Unexpected error: ' + (e.message || e))
    persistLastAnalyzed(newMessages[newMessages.length - 1].timestamp)
    await loadTasks()
    isTaskRefreshing.value = false
  }
}

const handleConfirmTaskChange = async (change) => {
  if (!change) return
  const agentName = props.agentName || props.chat.agent_name || props.chat.ownerAgent
  const groupId = props.chat.isGroupChat ? props.chat.groupId : null
  addRefreshLog('info', 'Confirming ' + change.type + ': ' + (change.taskId || change.task_id))
  try {
    // Ensure the change is actually persisted before removing from pending
    if (change.type === 'create' && change.data) {
      const payload = {
        agent_name: agentName,
        group_id: groupId || null,
        title: change.data.title,
        description: change.data.description || '',
        status: change.data.status || 'pending',
        progress: change.data.progress !== undefined ? change.data.progress : 0,
        priority: 'normal',
        prompt: change.data.prompt || null
      }
      await taskService.createTask(payload)
      addRefreshLog('info', 'Created task: ' + change.data.title)
    } else if (change.type === 'update' && change.task_id) {
      const updates = {}
      if (change.new_status) updates.status = change.new_status
      if (change.new_progress !== undefined) updates.progress = change.new_progress
      if (change.result_summary) updates.result_summary = change.result_summary
      await taskService.updateTask(change.task_id, updates)
      addRefreshLog('info', 'Updated task: ' + change.task_id)
    } else if (change.type === 'summary' && change.task_id) {
      await taskService.updateTask(change.task_id, { result_summary: change.result_summary })
      addRefreshLog('info', 'Updated summary: ' + change.task_id)
    }
    await loadTasks()
    pendingTaskChanges.value = pendingTaskChanges.value.filter(c =>
      !(c.type === change.type && (c.taskId === change.taskId || c.task_id === change.task_id))
    )
    addRefreshLog('info', 'Remaining pending: ' + pendingTaskChanges.value.length)
  } catch (e) {
    addRefreshLog('error', 'Confirm failed: ' + (e.message || e))
  }
}

const handleTaskReuse = (prompt) => {
  emit('update:modelValue', prompt)
}

const handleGeneratePrompt = async (task) => {
  if (!task || !task.task_id) return
  task._generatingPrompt = true
  addRefreshLog('info', 'Generating prompt for task: ' + task.task_id)

  try {
    // Get 0#Agent info
    const agentsRes = await zagentService.getAgents()
    const zeroAgent = agentsRes.data.find(function(a) { return a.agent_name === '0#Agent' })
    if (!zeroAgent || !zeroAgent.port) {
      addRefreshLog('error', '0#Agent not found or port missing')
      return
    }

    // Get messages before task creation time
    const msgs = props.chat.messages || []
    const taskCreatedAt = task.created_at ? task.created_at * 1000 : Date.now()
    const relevantMsgs = msgs.filter(m => m.timestamp && m.timestamp <= taskCreatedAt).slice(-10)

    const MAX_MSG_LEN = 300
    const msgText = relevantMsgs.map(m => {
      const role = m.isSent ? 'user' : 'assistant'
      const text = (m.text || '').substring(0, MAX_MSG_LEN)
      return `[${role}] ${m.sender || 'unknown'}: ${text}`
    }).join('\n\n')

    const prompt = `[系统指令] 请将以下任务信息和聊天记录整理成一个简洁、清晰的重用提示词。

## 任务信息
- 标题：${task.title}
- 描述：${task.description || '无'}
- 结果概要：${task.result_summary || '无'}

## 相关聊天记录
${msgText || '无'}

## 要求
1. 保留任务的核心意图和关键信息
2. 去除冗余的对话和确认过程
3. 输出格式适合直接作为新任务的指令
4. 长度控制在 50-150 字
5. 只输出提示词内容，不要任何解释或格式标记`

    addRefreshLog('info', 'Sending prompt to 0#Agent...')

    const sessionId = 'sys-gen-prompt-' + Date.now()
    let fullResponse = ''
    let hasResponded = false

    const ws = new ZeroClawWS(
      '0#Agent',
      sessionId,
      function(data) {
        if (data.type === 'chunk') {
          fullResponse += data.content
        } else if (data.type === 'done') {
          hasResponded = true
          ws.close()
          const generatedPrompt = fullResponse.trim()
          addRefreshLog('info', 'Generated prompt, length: ' + generatedPrompt.length)

          // Update task with generated prompt
          taskService.updateTask(task.task_id, { prompt: generatedPrompt }).then(() => {
            task.prompt = generatedPrompt
            emit('update:modelValue', generatedPrompt)
            addRefreshLog('info', 'Prompt saved and filled into input')
          }).catch(e => {
            addRefreshLog('error', 'Failed to save prompt: ' + (e.message || e))
          })
        }
      },
      function() {
        addRefreshLog('info', 'WebSocket connected, sending prompt...')
        ws.sendMessage(prompt)
      },
      function() {
        addRefreshLog('info', 'WebSocket closed.')
        task._generatingPrompt = false
      },
      function(error) {
        addRefreshLog('error', 'WebSocket error: ' + (error.message || error))
        task._generatingPrompt = false
      },
      zeroAgent.port
    )
    ws.connect()

    setTimeout(function() {
      if (!hasResponded) {
        ws.close()
        addRefreshLog('warn', 'Timeout waiting for 0#Agent response')
        task._generatingPrompt = false
      }
    }, 60000)
  } catch (e) {
    addRefreshLog('error', 'Failed to generate prompt: ' + (e.message || e))
    task._generatingPrompt = false
  }
}

// Copy function
const copyMessage = async (msg) => {
  try {
    await navigator.clipboard.writeText(msg.text || '')
  } catch (e) {
    console.error('[ChatMain] 拷贝到剪切板失败:', e)
  }
}

// Quote function
const quoteMessage = (msg) => {
  quotedMessage.value = {
    messageId: msg.id || Date.now().toString(),
    sender: msg.sender || msg.name,
    preview: msg.text || '',
    time: msg.time
  }
}

// Handle send - always emit send event
const handleSendWithQuote = (text) => {
  quotedMessage.value = null
  emit('send', text)
}

// Handle clear quote
const handleClearQuote = () => {
  quotedMessage.value = null
}

// Group chat member management
const availableZAgents = computed(() => {
  if (!props.chat.isGroupChat || !zAgents.value) return []
  const currentMembers = [props.chat.ownerAgent, ...(props.chat.members || [])]
  return zAgents.value.filter(a => currentMembers.indexOf(a.agent_name) === -1)
})

const fetchZAgents = inject('fetchZAgents')

const handleAddMember = async (agentName) => {
  if (!props.chat.isGroupChat) return
  try {
    await groupChatService.addMembers(props.chat.groupId, [agentName])
    if (!props.chat.members) props.chat.members = []
    props.chat.members.push(agentName)

    // Auto-start the agent if it's not running
    const agent = zAgents.value.find(a => a.agent_name === agentName)
    if (agent && agent.status !== 'running') {
      try {
        await zagentService.startAgent(agentName)
      } catch (err) {
        console.warn('[GroupChat] Failed to auto-start agent:', agentName, err)
      }
      if (fetchZAgents) await fetchZAgents()
    }
  } catch (e) {
    console.error('[ChatMain] Failed to add member:', e)
    alert('添加成员失败: ' + (e.message || e))
  }
}

const handleRemoveMember = async (agentName) => {
  if (!props.chat.isGroupChat) return
  if (!confirm('确定移除成员 ' + agentName + ' 吗？')) return
  try {
    await groupChatService.removeMember(props.chat.groupId, agentName)
    const idx = props.chat.members.indexOf(agentName)
    if (idx !== -1) props.chat.members.splice(idx, 1)
  } catch (e) {
    console.error('[ChatMain] Failed to remove member:', e)
    alert('移除成员失败: ' + (e.message || e))
  }
}

// 我的显示名，带 autoReplyAgent 名称后缀（如 "lord-argyll-8384/龙闺蜜"）
const myDisplayNameWithAgent = computed(() => {
  const myName = props.currentUserName || 'Me'
  if (!currentPeerConfig.value?.autoReplyAgent || !openclawAgents.value?.length) {
    return myName
  }
  const agent = openclawAgents.value.find(a => a.id === currentPeerConfig.value.autoReplyAgent)
  const agentName = agent ? (agent.identityName || agent.name) : currentPeerConfig.value.autoReplyAgent
  return myName + '/' + agentName
})

defineExpose({})

// ── # command handlers ───────────────────────────────────────────────────────────

const FIELD_MAP = {
  auto_reply: 'autoReply',
  auto_reply_agent: 'autoReplyAgent',
  peer_agent_name: 'peerAgentName',
  credit: 'credit',
  filter_chain: 'filterChain',
  send_filter_chain: 'sendFilterChain',
  is_blocked: 'isBlocked',
  run: 'run',
  muted: 'muted',
  thinking_time: 'thinkingTime',
  peer_profile: 'peerProfile',
  short_context: 'shortContext',
  long_context: 'longContext',
  peer_name: 'peerName',
}

const PEER_CONFIG_TABLE_FIELDS = [
  { key: 'peer',           header: 'peer' },
  { key: 'peerName',       header: 'peer_name' },
  { key: 'autoReply',      header: 'auto_reply' },
  { key: 'autoReplyAgent', header: 'auto_reply_agent' },
  { key: 'peerAgentName',  header: 'peer_agent_name' },
  { key: 'credit',         header: 'credit' },
  { key: 'isBlocked',      header: 'is_blocked' },
  { key: 'run',            header: 'run' },
  { key: 'muted',          header: 'muted' },
  { key: 'thinkingTime',   header: 'thinking_time' },
  { key: 'peerProfile',    header: 'peer_profile' },
  { key: 'shortContext',   header: 'short_context' },
  { key: 'longContext',    header: 'long_context' },
]

const LONG_TEXT_FIELDS = new Set(['peerProfile', 'shortContext', 'longContext'])

function formatPeerConfigsTable(configs) {
  if (!configs || configs.length === 0) return 'No peer configs found.'
  const fields = PEER_CONFIG_TABLE_FIELDS
  let html = '<table class="peer-config-table"><thead><tr>'
  fields.forEach(f => { html += `<th>${f.header}</th>` })
  html += '</tr></thead><tbody>'
  configs.forEach(cfg => {
    html += '<tr>'
    fields.forEach(f => {
      let val = cfg[f.key]
      if (val === undefined || val === null) val = ''
      if (typeof val === 'boolean') val = val ? 1 : 0
      val = String(val)
      if (LONG_TEXT_FIELDS.has(f.key) && val.length > 100) {
        val = val.slice(0, 100) + '...'
      }
      const style = LONG_TEXT_FIELDS.has(f.key) ? ' style="max-width:20ch;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;"' : ''
      html += `<td${style}>${val}</td>`
    })
    html += '</tr>'
  })
  html += '</tbody></table>'
  return html
}

function parseConfigValue(str) {
  if (str === 'true') return true
  if (str === 'false') return false
  const n = Number(str)
  if (!isNaN(n) && str !== '') return n
  return str
}

function insertSystemMessage(text, options = {}) {
  const now = new Date()
  const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
  if (!props.chat.messages) props.chat.messages = []
  props.chat.messages.push({
    text,
    time,
    sender: 'system',
    timestamp: now.getTime(),
    isSystemHint: true,
    isHtml: options.isHtml || false,
    isTemp: false,
  })
}

function insertConfigTableMessage(rows) {
  const now = new Date()
  const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
  if (!props.chat.messages) props.chat.messages = []
  props.chat.messages.push({
    text: '',
    time,
    sender: 'system',
    timestamp: now.getTime(),
    isSystemHint: true,
    isConfigTable: true,
    configRows: rows,
    meshName: props.meshName,
    isTemp: false,
  })
}

const handleHashCommand = async (cmdString) => {
  const trimmed = cmdString.trim()
  
  // Insert command echo
  insertSystemMessage('> ' + trimmed)
  
  try {
    // #list-all
    if (trimmed === '#list') {
      const res = await chatService.getAllPeerConfigs(props.meshName)
      const data = Array.isArray(res.data) ? res.data : []
      insertSystemMessage(formatPeerConfigsTable(data), { isHtml: true })
      return
    }
    
    // #config [agent] [peer]  — two args (second arg is not 'set')
    const configTwoArgs = trimmed.match(/^#config\s+(\S+)\s+(\S+)$/)
    if (configTwoArgs && configTwoArgs[2] !== 'set') {
      const agentName = configTwoArgs[1]
      const peerName = configTwoArgs[2]
      const res = await chatService.getAllPeerConfigs(props.meshName)
      const data = (Array.isArray(res.data) ? res.data : [])
        .filter(c => c.autoReplyAgent === agentName && c.peer === peerName)
      if (data.length) {
        insertConfigTableMessage(data)
      } else {
        insertSystemMessage(`No config found for agent: ${agentName}, peer: ${peerName}`)
      }
      return
    }

    // #config [agent-name] — one arg: list all peers for this agent
    const agentMatch = trimmed.match(/^#config\s+(\S+)$/)
    if (agentMatch) {
      const agentName = agentMatch[1]
      const res = await chatService.getAllPeerConfigs(props.meshName)
      const data = (Array.isArray(res.data) ? res.data : [])
        .filter(c => c.autoReplyAgent === agentName)
      if (data.length) {
        insertConfigTableMessage(data)
      } else {
        insertSystemMessage(`No peers found for agent: ${agentName}`)
      }
      return
    }

    // #config [agent] set [peer] key=value
    const configMatch = trimmed.match(/^#config\s+(\S+)\s+set\s+(\S+)\s+(\S+)=(.+)$/)
    if (configMatch) {
      const agentName = configMatch[1]
      const peerName = configMatch[2]
      const key = configMatch[3]
      const rawValue = configMatch[4]
      
      const mappedKey = FIELD_MAP[key]
      if (!mappedKey) {
        insertSystemMessage(`Unknown field: ${key}`)
        return
      }
      
      const value = parseConfigValue(rawValue)
      
      // First update auto_reply_agent, then the target key
      await chatService.updatePeerConfig(props.meshName, peerName, { autoReplyAgent: agentName })
      await chatService.updatePeerConfig(props.meshName, peerName, { [mappedKey]: value })
      
      insertSystemMessage(`✓ Updated **${peerName}**: ${key} = ${rawValue}`)
      return
    }
    
    insertSystemMessage(`Unknown command. Available: \`#list\`, \`#config [agent]\`, \`#config [agent] [peer]\`, \`#config [agent] set [peer] key=value\``)
  } catch (e) {
    insertSystemMessage(`Error: ${e?.message || String(e)}`)
  }
}

// ── end # command handlers ───────────────────────────────────────────────────────

const availableAgents = computed(() => {
  const currentAgentId = props.chat.agentId
  return (openclawAgents.value || []).filter(agent => agent.id !== currentAgentId)
})

const agentGroupChats = computed(() => {
  if (!props.chat.isOpenclaw) return []
  const agentId = props.chat.agentId
  return allGroupChats.value.filter(c =>
    c.members && c.members.indexOf(agentId) !== -1
  )
})

const currentDate = computed(() => {
  const now = new Date()
  const options = { year: 'numeric', month: 'long', day: 'numeric', weekday: 'long' }
  return now.toLocaleDateString('zh-CN', options)
})



const buildChatHtml = () => {
  const messages = props.chat.messages || []
  const chatName = props.chat.name || 'chat'
  const exportTime = new Date().toLocaleString('zh-CN')

  // Inline avatar color logic (mirrors avatar.js)
  const avatarColors = [
    '#e01e5a', '#2eb67d', '#ecb22e', '#1d9bd1', '#611f69',
    '#36c5f0', '#f2c744', '#ff6b6b', '#4ecdc4', '#9b59b6',
    '#e67e22', '#1abc9c',
  ]
  const getColor = (name) => {
    if (!name) return avatarColors[0]
    let hash = 0
    for (let i = 0; i < name.length; i++) {
      hash = name.charCodeAt(i) + ((hash << 5) - hash)
    }
    return avatarColors[Math.abs(hash) % avatarColors.length]
  }

  const escapeHtml = (str) => {
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
  }

  const msgRows = messages
    .filter(msg => !msg.isTyping)
    .map(msg => {
      const isSent = isMessageSent(msg)
      const senderName = isSent
        ? myDisplayNameWithAgent.value
        : (msg.sender || chatName)
      const time = escapeHtml(msg.time || '')
      const isOpenclawOrZeroClaw = (props.chat.isOpenclaw || props.chat.isZeroClaw) && !isSent
      const emoji = props.chat.emoji || (props.chat.isZeroClaw ? '🀄' : '')

      // Avatar HTML
      let avatarHtml
      if (isOpenclawOrZeroClaw) {
        avatarHtml = `<div class="avatar-emoji">${escapeHtml(emoji)}</div>`
      } else {
        const color = getColor(senderName)
        const initial = escapeHtml(senderName[0].toUpperCase())
        avatarHtml = `<div class="avatar-placeholder" style="background:${color}">${initial}</div>`
      }

      // Message bubble content
      const bubbleClass = msg.isSystemHint ? 'message-bubble system-hint' : 'message-bubble'
      const renderedText = marked.parse(msg.text || '')

      return `
    <div class="message${isSent ? ' sent' : ''}">
      <div class="message-avatar">${avatarHtml}</div>
      <div class="message-body">
        <div class="message-header">
          <span class="message-author">${escapeHtml(senderName)}</span>
          <span class="message-time">${time}</span>
        </div>
        <div class="${bubbleClass}">
          <div class="message-content">${renderedText}</div>
        </div>
      </div>
    </div>`
    })
    .join('\n')

  const html = `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${escapeHtml(chatName)} - 聊天记录</title>
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
        Oxygen, Ubuntu, Cantarell, 'Helvetica Neue', sans-serif;
      font-size: 15px;
      background: #f8f8f8;
      color: #1d1c1d;
    }
    .chat-header {
      position: sticky;
      top: 0;
      background: #ffffff;
      border-bottom: 1px solid rgba(0,0,0,0.07);
      padding: 14px 20px;
      display: flex;
      align-items: center;
      gap: 10px;
      z-index: 10;
    }
    .chat-header h1 {
      font-size: 16px;
      font-weight: 700;
      color: #1d1c1d;
    }
    .chat-header .export-time {
      font-size: 12px;
      color: #616061;
      margin-left: auto;
    }
    .messages {
      max-width: 860px;
      margin: 0 auto;
      padding: 20px 16px 40px;
    }
    .date-divider {
      display: flex;
      align-items: center;
      gap: 10px;
      margin: 16px 0;
      color: #616061;
      font-size: 12px;
    }
    .date-divider::before,
    .date-divider::after {
      content: '';
      flex: 1;
      height: 1px;
      background: rgba(0,0,0,0.1);
    }
    .message {
      display: flex;
      align-items: flex-start;
      gap: 10px;
      padding: 4px 0;
      margin-bottom: 4px;
    }
    .message.sent {
      flex-direction: row-reverse;
    }
    .message-avatar {
      flex-shrink: 0;
      width: 40px;
      height: 40px;
    }
    .avatar-placeholder {
      width: 40px;
      height: 40px;
      border-radius: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      color: #ffffff;
      font-size: 16px;
      font-weight: 700;
    }
    .avatar-emoji {
      width: 40px;
      height: 40px;
      border-radius: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 24px;
      background: rgba(0,0,0,0.04);
    }
    .message-body {
      display: flex;
      flex-direction: column;
      max-width: 80%;
    }
    .message.sent .message-body {
      align-items: flex-end;
    }
    .message-header {
      display: flex;
      align-items: baseline;
      gap: 8px;
      margin-bottom: 4px;
    }
    .message.sent .message-header {
      flex-direction: row-reverse;
    }
    .message-author {
      font-size: 15px;
      font-weight: 700;
      color: #1d1c1d;
    }
    .message-time {
      font-size: 11px;
      color: #616061;
    }
    .message-bubble {
      position: relative;
      background: #f2f0f0;
      border-radius: 12px;
      padding: 11px 15px;
      line-height: 1.4667;
      word-break: break-word;
    }
    .message-bubble::before {
      content: '';
      position: absolute;
      top: 12px;
      left: -6px;
      border: 6px solid transparent;
      border-right-color: #f2f0f0;
      border-left: none;
    }
    .message.sent .message-bubble {
      background: #0A2E6F;
      color: #ffffff;
    }
    .message.sent .message-bubble::before {
      left: auto;
      right: -6px;
      border-right: none;
      border-left: 6px solid #0A2E6F;
      border-right-color: transparent;
    }
    .message-bubble.system-hint {
      background: transparent;
      border: none;
      color: #333;
      box-shadow: none;
    }
    .message-bubble.system-hint::before {
      display: none;
    }
    /* Markdown content styles */
    .message-content p { margin: 0 0 4px; }
    .message-content p:last-child { margin-bottom: 0; }
    .message-content code {
      background: rgba(0,0,0,0.08);
      border-radius: 3px;
      padding: 1px 4px;
      font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
      font-size: 13px;
    }
    .message.sent .message-content code {
      background: rgba(255,255,255,0.15);
    }
    .message-content pre {
      background: rgba(0,0,0,0.05);
      border-radius: 6px;
      padding: 10px 12px;
      overflow-x: auto;
      margin: 6px 0;
    }
    .message-content pre code {
      background: none;
      padding: 0;
    }
    .message.sent .message-content pre {
      background: rgba(255,255,255,0.1);
    }
    .message-content blockquote {
      border-left: 3px solid #616061;
      padding-left: 10px;
      margin: 6px 0;
      color: #616061;
    }
    .message.sent .message-content blockquote {
      border-left-color: rgba(255,255,255,0.5);
      color: rgba(255,255,255,0.75);
    }
    /* Quote menu */
    .quote-menu {
      position: fixed;
      background: #fff;
      border-radius: 6px;
      box-shadow: 0 2px 12px rgba(0,0,0,0.15);
      z-index: 1000;
      overflow: hidden;
    }
    .quote-menu-item {
      padding: 8px 16px;
      cursor: pointer;
      font-size: 14px;
      color: #333;
    }
    .quote-menu-item:hover {
      background: #f0f0f0;
    }
    /* Quote button on message bubble */
    .quote-btn {
      background: none;
      border: none;
      color: #999;
      font-size: 12px;
      cursor: pointer;
      padding: 2px 8px;
      margin-top: 4px;
      border-radius: 4px;
      opacity: 0;
      transition: opacity 0.2s;
    }
    .message:hover .quote-btn {
      opacity: 1;
    }
    .quote-btn:hover {
      background: rgba(0,0,0,0.1);
      color: #666;
    }
    .copy-btn {
      background: none;
      border: none;
      color: #999;
      font-size: 12px;
      cursor: pointer;
      padding: 2px 8px;
      margin-top: 4px;
      border-radius: 4px;
      opacity: 0;
      transition: opacity 0.2s;
    }
    .message:hover .copy-btn {
      opacity: 1;
    }
    .copy-btn:hover {
      background: rgba(0,0,0,0.1);
      color: #666;
    }
    .message.sent .copy-btn {
      color: #aaa;
    }
    .message.sent .copy-btn:hover {
      background: rgba(255,255,255,0.1);
      color: #fff;
    }
    .message.sent .quote-btn {
      color: #aaa;
    }
    .message.sent .quote-btn:hover {
      background: rgba(255,255,255,0.1);
      color: #fff;
    }
    /* Quote content from 「 marks */
    }
    /* Quote preview in message */
    .message-quote {
      margin-top: 8px;
      padding: 8px 12px;
      background: #0A2E6F;
      border-radius: 4px;
      color: #fff;
    }
    .message.sent .message-quote {
      background: #0A2E6F;
    }
    .message-quote-header {
      display: flex;
      justify-content: space-between;
      margin-bottom: 4px;
      font-size: 12px;
    }
    .message-quote-author {
      font-weight: 500;
      color: #fff;
    }
    .message.sent .message-quote-author {
      color: #fff;
    }
    .message-quote-time {
      color: #ccc;
    }
    .message-quote-preview {
      font-size: 13px;
      color: #fff;
      overflow: hidden;
      text-overflow: ellipsis;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
    }
    .message.sent .message-quote-preview {
      color: #ccc;
    }
    .message-content a {
      color: #1d9bd1;
    }
    .message.sent .message-content a {
      color: #a8d8ea;
    }
    .message-content ul, .message-content ol {
      padding-left: 20px;
      margin: 4px 0;
    }
    .message-content h1, .message-content h2, .message-content h3 {
      margin: 6px 0 4px;
    }
    .footer {
      text-align: center;
      font-size: 12px;
      color: #616061;
      padding: 20px;
      border-top: 1px solid rgba(0,0,0,0.07);
      margin-top: 20px;
    }
  </style>
</head>
<body>
  <div class="chat-header">
    <h1>${escapeHtml(chatName)}</h1>
    <span class="export-time">导出于 ${escapeHtml(exportTime)}</span>
  </div>
  <div class="messages">
    <div class="date-divider"><span>${escapeHtml(exportTime)}</span></div>
    ${msgRows}
  </div>
  <div class="footer">— 聊天记录导出自 ClawParty —</div>
</body>
</html>`

  return { html, chatName }
}

const handleDownload = () => {
  const { html, chatName } = buildChatHtml()
  const blob = new Blob([html], { type: 'text/html;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${chatName}-chat-history.html`
  a.click()
  URL.revokeObjectURL(url)
}

const handleDownloadMd = () => {
  const messages = props.chat.messages || []
  const chatName = props.chat.name || 'chat'
  const exportTime = new Date().toLocaleString('zh-CN')

  const mdLines = [
    `# ${chatName}`,
    `> 导出于 ${exportTime}`,
    '',
    '---',
    ''
  ]

  messages
    .filter(msg => !msg.isTyping)
    .forEach(msg => {
      const isSent = isMessageSent(msg)
      const senderName = isSent
        ? myDisplayNameWithAgent.value
        : (msg.sender || chatName)
      const time = msg.time || ''
      const text = msg.text || ''

      const rendered = marked.parse(text)
      const plainText = rendered
        .replace(/<[^>]+>/g, '')
        .replace(/&nbsp;/g, ' ')
        .replace(/&amp;/g, '&')
        .replace(/&lt;/g, '<')
        .replace(/&gt;/g, '>')

      mdLines.push(`**${senderName}** <${time}>`)
      mdLines.push(plainText)
      mdLines.push('')
    })

  mdLines.push('---')
  mdLines.push(`*聊天记录导出自 ClawParty*`)

  const md = mdLines.join('\n')
  const blob = new Blob([md], { type: 'text/markdown;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${chatName}-chat-history.md`
  a.click()
  URL.revokeObjectURL(url)
}

const handleDownloadPdf = () => {
  const { html } = buildChatHtml()
  // Inject @media print styles for better PDF output
  const printHtml = html.replace('</style>', `
    @media print {
      body { background: #ffffff; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
      .chat-header { position: static; }
      .message-bubble { break-inside: avoid; }
      .message { break-inside: avoid; }
    }
  </style>`)
  const iframe = document.createElement('iframe')
  iframe.style.position = 'fixed'
  iframe.style.left = '-9999px'
  iframe.style.top = '-9999px'
  iframe.style.width = '0'
  iframe.style.height = '0'
  document.body.appendChild(iframe)
  iframe.contentDocument.open()
  iframe.contentDocument.write(printHtml)
  iframe.contentDocument.close()
  iframe.contentWindow.onafterprint = () => {
    document.body.removeChild(iframe)
  }
  setTimeout(() => {
    iframe.contentWindow.print()
  }, 300)
}

const filteredMessages = computed(() => {
  const msgs = props.chat.messages || []
  const result = []
  let i
  for (i = 0; i < msgs.length; i++) {
    const m = msgs[i]
    if (m.isHalfDraft) continue
    // Filter out NO_REPLY messages from LLM in group chats
    if (props.chat.isGroupChat && !!m.text && (m.text === 'NO_REPLY' || m.text.includes('NO_REPLY'))) continue
    if (!!m.text && m.text.indexOf(' GMT') >= 0) {
      m.text = m.text.split(/[^[]*] /).slice(1)[0]
    }
    result.push(m)
  }
  return result
})

const formatTime = (timestamp) => {
  if (!timestamp) return ''
  const date = new Date(timestamp)
  const now = new Date()
  const diff = now - date
  
  if (diff < 86400000) {
    return date.getHours().toString().padStart(2, '0') + ':' + 
           date.getMinutes().toString().padStart(2, '0')
  } else if (diff < 172800000) {
    return '昨天'
  } else {
    return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
  }
}

const renderMarkdown = (text) => {
  if (!text) return ''
  // Replace 「sender: preview」 with HTML quote bubble
  const processedText = text.replace(/「([^:]+): ([^」]+)」/g, (match, sender, preview) => {
    return `<div class="quote-content" style="background:#0A2E6F;color:#fff;padding:8px 12px;border-radius:4px;margin-top:8px;"><div style="font-weight:500;font-size:12px;margin-bottom:4px;">${sender}</div><div style="font-size:13px;">${preview}</div></div>`
  })
  return marked.parse(processedText)
}

const isMessageSent = (msg) => {
  // Rely on isSent flag set by parseMessages or API
  return msg.isSent === true
}

const openImagePreview = (url) => {
  window.open(url, '_blank')
}

const parseMessages = (data) => {
  return data.map(item => {
    // sender may be "gcid/username" for group chat messages; strip the gcid prefix for display
    const rawSender = item.sender || ''
    const displaySender = rawSender.indexOf('/') !== -1 ? rawSender.split('/')[1] : rawSender
    const isSent = displaySender === props.currentUserName
    
    // Determine sender display name
    let senderDisplay
    if (isSent) {
      // 我的消息：显示 ep-name/autoReplyAgentName
      senderDisplay = myDisplayNameWithAgent.value
    } else if (props.chat?.isGroup) {
      senderDisplay = resolveEpDisplayName(displaySender)
    } else {
      // 对方的消息：显示 ep-name/agentName（如果消息中包含 agentName）
      const peerAgentId = item.message?.agentName
      if (peerAgentId) {
        const peerAgent = (openclawAgents.value || []).find(a => a.id === peerAgentId)
        const peerAgentDisplayName = peerAgent ? (peerAgent.identityName || peerAgent.name) : peerAgentId
        senderDisplay = displaySender + '/' + peerAgentDisplayName
      } else {
        senderDisplay = resolveEpDisplayName(displaySender)
      }
    }
    // Resolve file URLs for image messages
    const rawFiles = item.message?.files || null
    const resolvedFiles = rawFiles && rawFiles.length > 0 && props.meshName
      ? rawFiles.map(f => {
          var url = ''
          var owner = f.owner || ''
          var hash = f.hash || ''
          if (owner && owner.indexOf('~') !== -1 && hash) {
            url = chatService.getFileFromSessionUrl(props.meshName, owner, hash)
          } else if (owner && hash) {
            url = chatService.getFileUrl(props.meshName, owner, hash)
          }
return {
  hash: hash,
  name: f.name || '',
  path: f.path || '',
  type: f.type || '',
  size: f.size || 0,
  owner: owner,
  url: url
}
        })
      : null
    return {
      text: item.message?.text || '',
      files: resolvedFiles,
      time: formatTime(item.time),
      sender: senderDisplay,
      isSent,
      timestamp: item.time,
      isSystemHint: item.isSystemHint || false,
      isGroupRequest: item.isGroupRequest || false,
      isGroupEpRequest: item.isGroupEpRequest || false,
      isPeerRequest: item.isPeerRequest || false,
      isHalfDraft: item.isHalfDraft || false,
      gcid: item.gcid || '',
      peer: item.peer || '',
      agentName: item.agentName || '',
      groupName: item.groupName || '',
      availableAgents: item.availableAgents || [],
      selectedAgent: (item.availableAgents?.find(a => a.startsWith('zeroclaw:'))) 
                     || item.availableAgents?.[0] 
                     || 'main',
    }
  })
}

const approveGroupRequest = async (msg) => {
  if (!props.meshName) return
  try {
    if (msg.isPeerRequest) {
      // Peer chat: enable auto-reply with selected agent
      const agentId = msg.selectedAgent || 'main'
      const agent = (openclawAgents.value || []).find(a => a.id === agentId)
      const peerAgentName = agent ? (agent.identityName || agent.name) : agentId
      await chatService.approvePeerAutoReply(props.meshName, msg.peer, agentId, peerAgentName)
      msg.isPeerRequest = false
      msg.isSystemHint = false
      msg.text = `Auto-reply enabled for "${msg.peer}" via agent "${peerAgentName}".`
    } else if (msg.isGroupEpRequest) {
      // ZTM EP member: enable auto-reply for this group with the selected agent
      const agentName = msg.selectedAgent || 'main'
      await chatService.approveGroupEpAutoReply(props.meshName, msg.gcid, agentName)
      msg.isGroupRequest = false
      msg.isGroupEpRequest = false
      msg.text = `Auto-reply enabled for group "${msg.groupName || msg.gcid}" via agent "${agentName}".`
    } else {
      // Local openclaw agent in group
      if (!msg.agentName) return
      await chatService.approveGroupAgentAutoReply(props.meshName, msg.gcid, msg.agentName)
      msg.isGroupRequest = false
      msg.text = `Auto-reply approved for agent "${msg.agentName}" in group "${msg.groupName || msg.gcid}".`
    }
  } catch (err) {
    console.error('[ChatMain] Failed to approve auto-reply:', err)
  }
}

const fetchMessages = async () => {
  if (props.chat.isOpenclaw) return
  if (!props.meshName || !props.chat.name) return
  
  try {
    const lastTimestamp = props.chat.messages?.length > 0
      ? Math.max(...props.chat.messages.filter(m => m.timestamp).map(m => m.timestamp))
      : Date.now() - (24 * 60 * 60 * 1000)
    
    let response
    if (props.chat.isGroup) {
      response = await chatService.getGroupMessagesSince(props.meshName, props.chat.creator, props.chat.groupId, lastTimestamp)
    } else {
      response = await chatService.getMessagesSince(props.meshName, props.chat.name, lastTimestamp)
    }
    const messages = parseMessages(response.data || [])
    
    messages.forEach(newMsg => {
      const existingIndex = props.chat.messages.findIndex(m => 
        !m.isTemp && m.timestamp === newMsg.timestamp
      )
      if (existingIndex === -1) {
        props.chat.messages.push(newMsg)
      }
    })
    
    props.chat.messages.sort((a, b) => a.timestamp - b.timestamp)
    scrollToBottom()
  } catch (error) {
    if (error.response?.status === 404) {
      // No new messages, keep existing
    } else {
      console.error('[ChatMain] 获取消息失败:', error)
    }
  }
}

const pollMessages = async () => {
  if (props.chat.isOpenclaw) return
  if (!props.meshName || !props.chat.name) return
  
  const sinceTimestamp = Date.now() - (30 * 1000)
  
  try {
    let response
    if (props.chat.isGroup) {
      response = await chatService.getGroupMessagesSince(props.meshName, props.chat.creator, props.chat.groupId, sinceTimestamp)
    } else {
      response = await chatService.getMessagesSince(props.meshName, props.chat.name, sinceTimestamp)
    }
    if (response.data?.length > 0) {
      const newMessages = parseMessages(response.data)
      newMessages.forEach(newMsg => {
        // Deduplicate by timestamp only — sender display names may change between polls
        const existingIndex = props.chat.messages.findIndex(m => 
          !m.isTemp && m.timestamp === newMsg.timestamp
        )
        if (existingIndex !== -1) {
          // Already have this message (from a previous poll), skip
        } else {
          // Check if there's a temp message with matching content
          const fileHashes = JSON.stringify(newMsg.files?.map(f => f.hash) || [])
          const tempIndex = props.chat.messages.findIndex(m =>
            m.isTemp && m.text === newMsg.text &&
            JSON.stringify(m.files?.map(f => f.hash) || []) === fileHashes
          )
          if (tempIndex !== -1) {
            props.chat.messages[tempIndex] = newMsg
          } else {
            props.chat.messages.push(newMsg)
          }
        }
      })
      // scrollToBottom()
    }
  } catch (error) {
    if (error.response?.status !== 404) {
      console.error('[ChatMain] 轮询消息失败:', error)
    }
  }
}

const startPolling = () => {
  stopPolling()
  pollTimer = setInterval(pollMessages, 1000)
}

const stopPolling = () => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

const scrollToBottom = () => {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
    }
  })
}

// ── peer mode (P/N/M/A buttons) ─────────────────────────────────────────────────

function derivePeerMode(cfg) {
  if (!cfg) return 'manual'
  if (cfg.isBlocked) return 'blocked'
  if (cfg.muted) return 'muted'
  if (cfg.halfAutomation) return 'half'
  if (cfg.autoReply) return 'auto'
  return 'manual'
}

function getPeerKey() {
  if (!props.chat || props.chat.isOpenclaw) return null
  if (props.chat.isGroup) return props.chat.gcid || null
  return props.chat.name || null
}

async function loadPeerMode() {
  const key = getPeerKey()
  if (!key || !props.meshName) { 
    peerMode.value = ''
    currentPeerConfig.value = null
    allPeerConfigs.value = []
    return 
  }
  try {
    const res = await chatService.getPeerConfig(props.meshName, key)
    peerMode.value = derivePeerMode(res.data)
    currentPeerConfig.value = res.data  // 存储完整的 peerConfig 数据
    
    // 获取所有 peer 配置，用于消息发送者名称显示
    const allRes = await chatService.getAllPeerConfigs(props.meshName)
    allPeerConfigs.value = Array.isArray(allRes.data) ? allRes.data : []
  } catch {
    peerMode.value = 'manual'
    currentPeerConfig.value = null
    allPeerConfigs.value = []
  }
}

async function handlePeerModeChange(mode) {
  if (mode === peerMode.value) return
  const key = getPeerKey()
  if (!key || !props.meshName) return
  const configMap = {
    blocked: { isBlocked: true,  muted: false, autoReply: false, halfAutomation: false },
    muted:   { isBlocked: false, muted: true,  autoReply: true,  halfAutomation: false },
    manual:  { isBlocked: false, muted: false, autoReply: false, halfAutomation: false },
    auto:    { isBlocked: false, muted: false, autoReply: true,  halfAutomation: false },
    half:    { isBlocked: false, muted: false, autoReply: true,  halfAutomation: true },
  }
  try {
    await chatService.updatePeerConfig(props.meshName, key, configMap[mode])
    peerMode.value = mode
    
    // When switching to half mode, check for existing drafts
    if (mode === 'half' && props.chat.messages) {
      const halfDrafts = props.chat.messages.filter(m => m.isHalfDraft)
      if (halfDrafts.length > 0) {
        const latestDraft = halfDrafts[halfDrafts.length - 1]
        if (latestDraft.text) {
          halfDraftText.value = latestDraft.text
          nextTick(() => {
            if (halfInputRef.value) {
              halfInputRef.value.setDraft(latestDraft.text)
            }
          })
        }
      }
    }
  } catch (e) {
    console.error('[ChatMain] Failed to update peer mode:', e)
  }
}

// ── half automation handlers ────────────────────────────────────────────────────

async function handleHalfSend(text) {
  if (!text.trim()) return
  if (!props.meshName || !props.chat.name) return
  
  try {
    const sessionId = makeSessionId(props.currentUserName, props.chat.name)
    await chatService.sendMessage(props.meshName, props.chat.name, text, sessionId)
    
    // Add sent message to local messages
    const now = new Date()
    const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
    if (!props.chat.messages) props.chat.messages = []
    props.chat.messages.push({
      text: text,
      time: time,
      sender: props.currentUserName,
      timestamp: now.getTime(),
      isSent: true,
      isTemp: true,
    })
    
    // Clear the draft
    halfDraftText.value = ''
    if (halfInputRef.value) {
      halfInputRef.value.setDraft('')
    }
    
    // Remove the half draft messages
    props.chat.messages = props.chat.messages.filter(m => !m.isHalfDraft)
  } catch (e) {
    console.error('[ChatMain] Failed to send half automation message:', e)
  }
}

function handleDraftUpdated(text) {
  halfDraftText.value = text
}

function makeSessionId(peerA, peerB) {
  return peerA < peerB ? peerA + '~' + peerB : peerB + '~' + peerA
}

// Watch for new half automation drafts in messages
watch(() => props.chat.messages?.length, () => {
  if (peerMode.value !== 'half') return
  if (!props.chat.messages || props.chat.messages.length === 0) return
  
  // Find the latest half draft message
  const halfDrafts = props.chat.messages.filter(m => m.isHalfDraft)
  if (halfDrafts.length > 0) {
    const latestDraft = halfDrafts[halfDrafts.length - 1]
    // isHalfDraft messages have text directly in the message object, not in message.text
    const draftText = latestDraft.text || latestDraft.message?.text || ''
    if (draftText && draftText !== halfDraftText.value) {
      halfDraftText.value = draftText
      nextTick(() => {
        if (halfInputRef.value) {
          halfInputRef.value.setDraft(draftText)
        }
      })
    }
  }
})

// Update session ID when peer changes
watch(() => props.chat.name, (name) => {
  if (name && props.currentUserName) {
    currentSessionId.value = makeSessionId(props.currentUserName, name)
  }
}, { immediate: true })

// ── end peer mode ────────────────────────────────────────────────────────────────

// Fetch history + start polling only when this chat is both named AND active.
// Previously every mounted ChatMain (one per chat in the list) fired fetchMessages
// on mount via `immediate: true`, which on startup triggered a flood of
// syncGroupMessages/syncPeerMessages calls — each one reads every historical
// message file for that chat from the hub.
watch(
  () => [props.chat.name, props.isActive],
  async ([name, isActive], prev) => {
    if (prev && prev[0] === name && prev[1] === isActive) return
    if (!isActive) {
      if (prev && prev[1]) {
        stopPolling()
      }
      return
    }
    await loadPeerMode()
    if (name && !props.chat.isOpenclaw) {
      fetchMessages().then(() => {
        startPolling()
      })
    }
    if (props.chat.isZeroClaw || props.chat.isGroupChat) {
      await loadTasks()
      showTaskPanel.value = true
      requestAnimationFrame(() => {
        calcTaskPanelHeight()
      })
      showMembersPanel.value = !!props.chat.isGroupChat
    }
  },
  { immediate: true }
)

watch(() => props.chat.messages?.length, () => {
  scrollToBottom()
}, { immediate: true })

// When Task button toggles showTaskPanel back on, immediately load
watch(showTaskPanel, async (visible) => {
  if (visible && (props.chat.isZeroClaw || props.chat.isGroupChat)) {
    requestAnimationFrame(() => {
      calcTaskPanelHeight()
    })
    await loadTasks()
    await loadKanbanConfig()
  }
})

const handleMessagesClick = (e) => {
  // Don't close panels if clicking on interactive elements inside messages
  if (e.target.closest('button, a, select, input, textarea, .attachment-item')) return
  showWebSharePanel.value = false
  showTaskPanel.value = false
  showWikiPanel.value = false
  showRadarPanel.value = false
}

onUnmounted(() => {
  stopPolling()
})
</script>

<style scoped>
.chat-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--bg-chat);
  min-width: 0;
	z-index: 1;
}

.messages {
  flex: 1;
  overflow-y: auto;
  padding: 0 20px 20px;
}

.date-divider {
  display: flex;
  align-items: center;
  margin: 28px 0 20px;
  padding-top: 20px;
}

.date-divider::before,
.date-divider::after {
  content: '';
  flex: 1;
  height: 1px;
  background: rgba(0, 0, 0, 0.1);
}

.date-divider span {
  padding: 0 16px;
  color: var(--text-dim);
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
}

.message {
  display: flex;
  padding: 2px 0;
  margin-top: 20px;
  position: relative;
}

.message:hover {
  background: rgba(0, 0, 0, 0.02);
}

.message.sent {
  flex-direction: row-reverse;
}

.message-avatar {
  margin-right: 12px;
  flex-shrink: 0;
}

.message.sent .message-avatar {
  margin-right: 0;
  margin-left: 12px;
}

.avatar-placeholder {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: 700;
  font-size: 18px;
  cursor: pointer;
}

.avatar-placeholder:hover {
  opacity: 0.9;
}

.avatar-emoji {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  cursor: pointer;
}

.message-body {
  flex: 1;
  min-width: 0;
  max-width: 80%;
}

.message.sent .message-body {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}

.message-actions {
  display: flex;
  flex-direction: row;
  gap: 8px;
  margin-top: 4px;
}

.message-header {
  display: flex;
  align-items: baseline;
  margin-bottom: 4px;
}

.message.sent .message-header {
  flex-direction: row-reverse;
}

.message-author {
  color: var(--text-primary);
  font-size: 15px;
  font-weight: 700;
  margin-right: 8px;
  cursor: pointer;
}

.message-author:hover {
  text-decoration: underline;
}

.message.sent .message-author {
  margin-right: 0;
  margin-left: 8px;
}

.message-time {
  color: var(--text-dim);
  font-size: 11px;
}

.message-bubble {
  background: #f2f0f0;
  border-radius: 12px;
  padding: 11px 15px;
  position: relative;
  max-width: 80%;
  width: fit-content;
}

.message-bubble.system-hint {
  background: transparent;
  border: none;
  color: #333;
  box-shadow: none;
  padding: 0;
  max-width: none;
  width: auto;
}

.message-bubble.system-hint::before {
  display: none;
}

.group-request-actions {
  margin-top: 8px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.agent-select {
  border: 1px solid var(--border-color, #ddd);
  border-radius: 6px;
  padding: 4px 8px;
  font-size: 13px;
  background: var(--bg-primary, #fff);
  color: var(--text-primary, #1d1c1d);
  cursor: pointer;
}

.approve-btn {
  background: var(--slack-green, #2bac76);
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 5px 14px;
  font-size: 13px;
  cursor: pointer;
  font-weight: 600;
}

.approve-btn:hover {
  opacity: 0.85;
}

.message.sent .message-bubble {
  background: var(--slack-purple);
}

.message.sent .message-bubble::before {
  content: '';
  position: absolute;
  left: -8px;
  top: 12px;
  border: 8px solid transparent;
  border-right-color: var(--slack-purple);
}

.message:not(.sent) .message-bubble::before {
  content: '';
  position: absolute;
  right: -8px;
  top: 12px;
  border: 8px solid transparent;
  border-left-color: #f2f0f0;
}

.message-images {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 4px;
}

.chat-image {
  max-width: 320px;
  max-height: 240px;
  border-radius: 8px;
  cursor: pointer;
}

.attachment-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  max-width: 320px;
}

.file-text {
  font-size: 14px;
  color: var(--text-secondary);
  padding: 8px 0;
  word-break: break-all;
  background: var(--bg-hover);
  border-radius: 4px;
  padding: 6px 10px;
  flex-grow: 1;
}

.chat-image {
  max-width: 320px;
  max-height: 240px;
  border-radius: 8px;
  cursor: pointer;
  object-fit: contain;
  background: rgba(0, 0, 0, 0.03);
}

.chat-image:hover {
  opacity: 0.85;
}

.message-content {
  color: var(--text-primary);
  font-size: 15px;
  line-height: 1.4667;
  word-wrap: break-word;
}

.message-content :deep(table.peer-config-table) {
  width: 100%;
  border-collapse: collapse;
  margin: 8px 0;
  font-size: 13px;
}

.message-content :deep(table.peer-config-table th),
.message-content :deep(table.peer-config-table td) {
  border: 1px solid #999;
  padding: 4px 8px;
  text-align: left;
  white-space: nowrap;
}

.message-content :deep(table.peer-config-table th) {
  background: #e0e0e0;
  font-weight: 600;
}

.message-content :deep(table.peer-config-table tr:nth-child(even) td) {
  background: #f5f5f5;
}

.message.sent .message-content {
  color: #ffffff;
}

.message-content :deep(p) {
  margin: 0 0 8px 0;
}

.message-content :deep(p:last-child) {
  margin-bottom: 0;
}

.message-content :deep(code) {
  background: rgba(0, 0, 0, 0.08);
  padding: 2px 5px;
  border-radius: 4px;
  font-family: 'SF Mono', Monaco, Consolas, monospace;
  font-size: 13px;
}

.message.sent .message-content :deep(code) {
  background: rgba(255, 255, 255, 0.2);
}

.message-content :deep(pre) {
  background: rgba(0, 0, 0, 0.05);
  padding: 10px 12px;
  border-radius: 6px;
  overflow-x: auto;
  margin: 8px 0;
}

.message-content :deep(pre code) {
  background: none;
  padding: 0;
}

.message-content :deep(ul),
.message-content :deep(ol) {
  margin: 8px 0;
  padding-left: 20px;
}

.message-content :deep(blockquote) {
  border-left: 3px solid var(--text-secondary);
  padding-left: 12px;
  margin: 8px 0;
  color: var(--text-secondary);
}

.message-content :deep(a) {
  color: var(--slack-blue);
  text-decoration: none;
}

.message.sent .message-content :deep(a) {
  color: #fff;
}

.message-content :deep(a:hover) {
  text-decoration: underline;
}

.typing-indicator-hidden {
  display: none !important;
}

.typing-indicator {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  background: var(--bg-hover);
  border-radius: 8px;
}

.typing-dot {
  width: 6px;
  height: 6px;
  background: var(--text-secondary);
  border-radius: 50%;
  animation: typing-bounce 1.4s infinite ease-in-out;
}

.typing-dot:nth-child(1) {
  animation-delay: 0s;
}

.typing-dot:nth-child(2) {
  animation-delay: 0.2s;
}

.typing-dot:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes typing-bounce {
  0%, 60%, 100% {
    transform: translateY(0);
  }
  30% {
    transform: translateY(-4px);
  }
}

.message-content :deep(strong) {
  font-weight: 700;
}

.message-content :deep(em) {
  font-style: italic;
}

/* ── Group chat styles ─────────────────────────────────────── */

.agent-tag {
  display: inline-block;
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
  color: #fff;
  margin-right: 6px;
}

.owner-tag {
  background: #611f69 !important;
}

.group-agent-avatar {
  font-weight: 700;
  font-size: 14px;
}

.group-owner-avatar {
  font-size: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* ── Members panel ─────────────────────────────────────────── */

.chat-body-wrapper {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.chat-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.members-panel {
  width: 240px;
  border-left: 1px solid var(--border-subtle);
  background: var(--bg-secondary);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  padding: 16px;
}

.members-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.members-panel-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
}

.members-panel-close {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.members-panel-close:hover {
  background: var(--bg-hover);
}

.members-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-dim);
  text-transform: uppercase;
  margin-bottom: 8px;
  letter-spacing: 0.5px;
}

.members-item,
.members-add-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 6px;
  cursor: default;
  margin-bottom: 4px;
}

.members-add-item {
  cursor: pointer;
}

.members-add-item:hover {
  background: var(--bg-hover);
}

.members-avatar {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  flex-shrink: 0;
}

.members-name {
  flex: 1;
  font-size: 13px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.members-remove-btn {
  width: 20px;
  height: 20px;
  border: none;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  flex-shrink: 0;
}

.members-remove-btn:hover {
  background: rgba(255, 0, 0, 0.1);
  color: #e22;
}

.members-add-icon {
  font-size: 16px;
  color: var(--slack-green, #2bac76);
  font-weight: 700;
}

.members-empty {
  font-size: 12px;
  color: var(--text-dim);
  padding: 8px;
  text-align: center;
}

.members-owner {
  margin-bottom: 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-subtle);
}

.members-list {
  margin-bottom: 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-subtle);
}

@media (max-width: 768px) {
  .chat-main {
    flex: 1;
    width: 100%;
    height: calc(100vh - 48px - env(safe-area-inset-top, 0));
    min-height: 0;
    overflow-x: hidden;
  }
  
  .chat-header {
    padding-top: 8px;
  }
  
  .message-body {
    max-width: 85%;
    overflow-wrap: break-word;
    word-break: break-word;
  }
  
  .messages {
    padding: 0 12px 12px;
    overflow-x: hidden;
  }
  
  .message-bubble {
    max-width: 100%;
    overflow-wrap: break-word;
    word-break: break-word;
  }
}
</style>
