<template>
  <div v-if="showTokenDialog" class="token-dialog-wrap">
    <div class="token-dialog">
      <h2>Enter Access Token</h2>
      <p>Enter the API token for your ztm agent</p>
      <input
        v-model="tokenInput"
        type="password"
        autocomplete="off"
        placeholder="API token"
        @keyup.enter="submitToken"
      />
      <button :disabled="tokenChecking || !tokenInput.trim()" @click="submitToken">
        {{ tokenChecking ? 'Verifying...' : 'Continue' }}
      </button>
      <div v-if="tokenError" class="token-error">{{ tokenError }}</div>
    </div>
  </div>
  <div v-else class="chat-container">
    <ChatSidebar
      :chats="chats"
      :activeChat="activeChat"
      @select="selectChat"
      @selectOpenclaw="selectOpenclawAgent"
      @changeOrg="(org) => { mobileActiveOrg = org; activeChat = null; activeOpenclawAgent = null; }"
      @resetActiveChat="activeChat = null"
      @openLocalTemplates="openLocalTemplates"
      @openSharedTemplates="openSharedTemplates"
      @zagentTemplateCreated="handleZAgentTemplateCreated"
    />
    <!-- Mobile agents list view -->
    <div v-if="isMobile && activeChat === null && mobileActiveOrg === 'agents'" class="mobile-agents-view">
      <div class="mobile-agents-header">My Agents</div>
      <div class="mobile-agents-list">
        <div
          v-for="agent in openclawAgents"
          :key="agent.id"
          class="mobile-agent-item"
          @click="selectOpenclawAgent(agent)"
        >
          <div class="item-avatar openclaw-avatar">{{ agent.emoji }}</div>
          <span class="item-name">{{ agent.name }}</span>
        </div>
        <div v-if="!openclawAgents || openclawAgents.length === 0" class="mobile-empty">
          <div>No local agents</div>
          <div class="mobile-empty-hint">openclaw is not installed locally.</div>
        </div>
      </div>
    </div>
    <!-- Mobile groups list view -->
    <div v-if="isMobile && activeChat === null && mobileActiveOrg === 'groups'" class="mobile-agents-view">
      <div class="mobile-agents-header">Group Chats</div>
      <div class="mobile-agents-list">
        <div
          v-for="chat in localGroupChats"
          :key="chat.groupId"
          class="mobile-agent-item"
          @click="enterGroupChat(chat.groupId)"
        >
          <div class="item-avatar">#</div>
          <span class="item-name">{{ chat.groupName }}</span>
        </div>
        <div v-if="!localGroupChats || localGroupChats.length === 0" class="mobile-empty">
          还没有群聊
        </div>
      </div>
    </div>
    <!-- Mobile mesh (ClawParty) list view -->
    <div v-if="isMobile && activeChat === null && mobileActiveOrg && mobileActiveOrg !== 'agents' && mobileActiveOrg !== 'groups'" class="mobile-agents-view">
      <div class="mobile-agents-header">{{ mobileActiveOrg }}</div>
      <div class="mobile-agents-list">
        <div
          v-for="user in users"
          :key="'user-' + user.name"
          class="mobile-agent-item"
          @click="selectUser(user)"
        >
          <div class="item-avatar" :style="{ background: getAvatarColor(user.username || user.name) }">{{ user.username[0].toUpperCase() }}</div>
          <span class="item-name">{{ user.name }}</span>
        </div>
        <div
          v-for="chat in meshChats"
          :key="chat.id"
          class="mobile-agent-item"
          @click="selectChat(getChatIndex(chat.id))"
        >
          <div class="item-avatar" :style="{ background: getAvatarColor(chat.name) }">{{ chat.name[0].toUpperCase() }}</div>
          <span class="item-name">{{ chat.name }}</span>
        </div>
      </div>
    </div>
    <ChatMain
      v-for="item in activeZAgentConnectionItems"
      :key="'zagent-' + item.id"
      v-show="currentActiveChatId === item.id && item.type === 'zagent'"
      :chat="item.chat"
      :meshName="null"
      :currentUserName="currentMeshAgentUsername"
      :sending="sending && currentActiveChatId === item.id"
      :openclawSessions="[]"
      :showBackButton="isMobile"
      :autoFocus="!isMobile"
      :agentStatus="item.agent?.status"
      :agentErrorMsg="item.agent?.error_msg"
      :agentName="item.id"
      :isActive="currentActiveChatId === item.id"
      v-model="newMessage"
      @send="(text) => handleZAgentSend(item.id, text)"
      @send-images="handleSendImages"
      @send-files="handleSendFiles"
      @switchSession="() => {}"
      @deleteGroup="handleDeleteGroup"
      @leaveGroup="handleLeaveGroup"
      @back="currentActiveChatId = null"
      @start-agent="handleStartZAgent"
    />
    <ChatMain
      v-for="item in activeChatConnectionItems"
      :key="item.id"
      v-show="currentActiveChatId === item.id"
      :chat="item.chat"
      :meshName="currentMesh"
      :currentUserName="currentMeshAgentUsername"
      :sending="sending && currentActiveChatId === item.id"
      :openclawSessions="[]"
      :isActive="currentActiveChatId === item.id"
      :showBackButton="isMobile"
      :autoFocus="!isMobile"
      v-model="newMessage"
      @send="(text) => handleChatSend(item.id, text)"
      @send-images="handleSendImages"
      @send-files="handleSendFiles"
      @switchSession="() => {}"
      @deleteGroup="handleDeleteGroup"
      @leaveGroup="handleLeaveGroup"
      @back="handleChatBack(item.id)"
    />
    <ChatMain
      v-if="activeGroupId"
      :chat="localGroupChats.find(g => g.groupId === activeGroupId)"
      :meshName="null"
      :currentUserName="currentMeshAgentUsername"
      :sending="sending && activeGroupId"
      :isGroupChat="true"
      :showBackButton="isMobile"
      :autoFocus="!isMobile"
      v-model="newMessage"
      @send="sendMessage"
      @send-images="handleSendImages"
      @send-files="handleSendFiles"
      @deleteGroup="handleDeleteLocalGroup(localGroupChats.find(g => g.groupId === activeGroupId))"
      @leaveGroup="leaveGroupChat(activeGroupId)"
      @back="activeGroupId = null"
    />
    <ChatMain
      v-if="activeOpenclawAgent || activeZeroClawSession"
      :chat="activeZeroClawSession || activeOpenclawAgent"
      :meshName="(activeOpenclawAgent && activeOpenclawAgent.agentId !== 'main') ? null : currentMesh"
      :currentUserName="currentMeshAgentUsername"
      :sending="sending"
      :openclawSessions="openclawSessions"
      :showBackButton="isMobile"
      :autoFocus="!isMobile"
      v-model="newMessage"
      @send="sendMessage"
      @send-images="handleSendImages"
      @send-files="handleSendFiles"
      @switchSession="(sessionId) => switchOpenclawSession(activeOpenclawAgent, sessionId)"
      @deleteGroup="handleDeleteGroup"
      @leaveGroup="handleLeaveGroup"
      @back="activeOpenclawAgent ? (activeOpenclawAgent = null) : (activeZeroClawSession = null)"
    />
    <div
      v-if="!isMobile && activeChat === null && !activeGroupId && !activeOpenclawAgent && !activeZeroClawSession && currentActiveChatId === null"
      class="empty-state"
    >
      <div class="empty-icon">
        <svg width="80" height="80" viewBox="0 0 80 80" fill="none">
          <circle cx="40" cy="40" r="40" fill="#E8E8E8"/>
          <path d="M40 20C29.5 20 21 28.5 21 39c0 7.3 4.2 13.7 10.5 17.5v5.5c0 2.2 1.8 4 4 4h9c2.2 0 4-1.8 4-4v-5.5c6.3-3.8 10.5-10.2 10.5-17.5C59 28.5 50.5 20 40 20z" fill="#1D6CFF"/>
        </svg>
      </div>
      <h2>Welcome to ClawParty!</h2>
    </div>
  </div>

  <TemplatePicker
    :show="showLocalTemplates"
    source="local"
    :installedAgentIds="installedAgentIds"
    @close="showLocalTemplates = false"
    @installed="handleTemplateInstalled"
    @open-main-chat="openMainChatForInstall"
    @send-messages="handleSendMessages"
  />
  <TemplatePicker
    :show="showSharedTemplates"
    source="shared"
    :installedAgentIds="installedAgentIds"
    @close="showSharedTemplates = false"
    @installed="handleTemplateInstalled"
    @open-main-chat="openMainChatForInstall"
    @send-messages="handleSendMessages"
  />
</template>

<script setup>
import { ref, onMounted, onUnmounted, provide, computed, watch, reactive } from 'vue'
import ChatSidebar from './components/ChatSidebar.vue'
import ChatMain from './components/ChatMain.vue'
import TemplatePicker from './components/TemplatePicker.vue'
import { chatService, meshService, openclawService, zeroclawService, zagentService, groupChatService, taskService, ZeroClawWS, VOICE_MESSAGE_TYPES, setApiToken, getApiToken } from './services/chatService'
import ShellService from './services/ShellService'
import { platform } from '@tauri-apps/plugin-os';
import { getAvatarColor } from './utils/avatar'
import { getSemanticEmoji } from './utils/emoji'

import { useVoiceCall } from './composables/useVoiceCall'

const shellService = new ShellService();

const meshes = ref([])
const openclawAgents = ref([])
const openclawSessions = ref([])
const zeroclawSessions = ref([])
const activeZeroClawSession = ref(null)
const zAgents = ref([])
const activeZAgent = ref(null)
const currentActiveChatId = ref(null)
const currentMesh = ref('')
const currentMeshAgentUsername = ref('')

const voiceCallSendChatFn = (text) => {
  newMessage.value = text
  sendMessage()
}

const voiceCallSendFn = (msgType, payload) => {
  // In Agent voice mode, skip P2P voice signaling (voice-invite, voice-end, etc.)
  if (voiceCallStore?.isAgentMode?.value && msgType.startsWith('voice-')) {
    return
  }

  const agent = activeZAgent.value
  const session = activeZeroClawSession.value
  const conn = agent
    ? wsConnections[agent.agent_name]
    : null
  const ws = conn?.zeroclawWS || zeroclawWS
  if (ws && ws.sendVoiceMessage) {
    ws.sendVoiceMessage(msgType, payload)
    return
  }

  const peer = activeChat.value?.name
  const mesh = currentMesh.value
  if (mesh && peer) {
    chatService.sendVoiceSignaling(mesh, peer, msgType, payload)
      .catch(e => console.error('[voiceCallSendFn] Mesh send failed:', e))
  }
}

const voiceCallStore = useVoiceCall(voiceCallSendFn, currentMeshAgentUsername.value || 'You', voiceCallSendChatFn)
const chats = ref([])
const activeChat = ref(null)
const activeOpenclawAgent = ref(null)  // 当前活动的 openclaw agent
const newMessage = ref('')
const sending = ref(false)
const showTokenDialog = ref(false)
const tokenInput = ref('')
const tokenChecking = ref(false)
const tokenError = ref('')
const switchingTo = ref(null)
const isMobile = ref(window.innerWidth <= 768)
const mobileActiveOrg = ref('agents')
const users = ref([])
const localOpenclawAvailable = ref(false)
const showLocalTemplates = ref(false)
const showSharedTemplates = ref(false)
let appStarted = false
let chatsPollTimer = null
let usersPollTimer = null
let zeroclawSessionsPollTimer = null
let zeroclawWS = null
const wsConnections = reactive({})

// Local group chat state (ZeroClaw agent groups)
const localGroupChats = ref([])
const activeGroupId = ref(null)
const activeGroupWsMap = reactive(new Map())  // groupId -> [{ agentName, ws }]

const activeZAgentConnectionItems = computed(() => {
  if (!zAgents.value) return []
  
  const activeAgent = zAgents.value.find(a => a.agent_name === currentActiveChatId.value)
  
  return zAgents.value
    .filter(agent => {
      return wsConnections[agent.agent_name] || agent === activeAgent
    })
    .map(agent => {
      const cached = wsConnections[agent.agent_name] || {}
      const msgCount = cached._msgCount || 0
      return {
        type: 'zagent',
        id: agent.agent_name,
        agent: {
          ...agent,
          isZeroClaw: true,
          _msgCount: msgCount,
          messages: cached.messages || []
        },
        messages: cached.messages || [],
        chat: {
          ...agent,
          isZeroClaw: true,
          emoji: getSemanticEmoji(agent.display_name || agent.agent_name),
          _msgCount: msgCount,
          messages: cached.messages || []
        }
      }
    })
})

const activeChatConnectionItems = computed(() => {
  return chats.value
    .filter(c => !c.isOpenclaw && !c.isZeroClaw)
    .map(c => ({
      type: c.isGroup ? 'group' : 'peer',
      id: c.id,
      chat: c,
      messages: c.messages || []
    }))
})

const allChatConnectionItems = computed(() => {
  return [...activeZAgentConnectionItems.value, ...activeChatConnectionItems.value]
})

const handleResize = () => {
  isMobile.value = window.innerWidth <= 768
}


window.addEventListener('resize', handleResize)

provide('currentMesh', currentMesh)

const groupChats = computed(() =>
  chats.value.filter(c => c.isGroup && !c.isOpenclaw)
)

const meshChats = computed(() => {
  if (!mobileActiveOrg.value || mobileActiveOrg.value === 'agents' || mobileActiveOrg.value === 'groups') {
    return []
  }
  return chats.value.filter(c => !c.isGroup && !c.isOpenclaw)
})

const getChatIndex = (chatId) => {
  return chats.value.findIndex(c => c.id === chatId)
}

const formatTime = (timestamp) => {
  if (!timestamp) return ''
  const date = new Date(timestamp)
  const now = new Date()
  const diff = now - date
  
  if (diff < 86400000) {
    return date.getHours().toString().padStart(2, '0') + ':' + 
           date.getMinutes().toString().padStart(2, '0')
  } else if (diff < 172800000) {
    return 'Yesterday'
  } else {
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
  }
}

const parseChatData = (data) => {
  return data.map(item => {
    const name = item.peer || item.name || 'Unknown'
    var latestMsg = item.latest?.message?.text || ''
    if (!latestMsg && Array.isArray(item.latest?.message?.files) && item.latest.message.files.length > 0) {
      latestMsg = '[图片/文件]'
    }
    const firstLine = latestMsg.split('\n')[0].substring(0, 30)
    const isGroup = !!item.group
    const peerAgentName = item.peerAgentName || ''
    const displayName = (peerAgentName && !isGroup) ? `${name}/${peerAgentName}` : name

    return {
      id: item.group || item.peer || Math.random().toString(),
      name: name,
      displayName: displayName,
      peerAgentName: peerAgentName,
      time: formatTime(item.time),
      lastMessage: firstLine,
      updated: item.updated || 0,
      isGroup: isGroup,
      gcid: item.gcid || '',
      creator: item.creator || '',
      groupId: item.group || '',
      members: item.members || [],
      messages: item.latest ? [
        {
          text: item.latest.message?.text || '',
          time: formatTime(item.latest.time),
          isSent: false
        }
      ] : []
    }
  })
}

const fetchMeshes = async () => {
  try {
    const response = await meshService.getMeshes()
    meshes.value = response.data
    if (meshes.value.length > 0) {
      currentMesh.value = meshes.value[0].name
      currentMeshAgentUsername.value = meshes.value[0].agent?.username || ''
      await fetchChats()
      await fetchUsers()
    }
  } catch (error) {
    console.error('Failed to fetch meshes:', error)
  }
}

const fetchOpenclawAgents = async () => {
  try {
    const response = await openclawService.getAgents()
    const agentsData = Array.isArray(response.data) ? response.data : []
    localOpenclawAvailable.value = agentsData.length > 0
    // 只更新 openclawAgents 列表，不添加到 chats 列表
    openclawAgents.value = agentsData.map(agent => ({
      id: agent.id,
      name: agent.name || agent.identityName || agent.id,
      emoji: agent.identityEmoji || agent.emoji || '🤖',
      model: agent.model,
      isOpenclaw: true
    }))
  } catch (error) {
    localOpenclawAvailable.value = false
    console.error('Failed to fetch OpenClaw agents:', error)
  }
}

const fetchChats = async () => {
  if (!currentMesh.value) return
  try {
    const response = await chatService.getChats(currentMesh.value)
    const newChats = parseChatData(response.data)
    
    const savedChatId = activeChat.value !== null ? chats.value[activeChat.value]?.id : null
    const savedIsOpenclaw = activeChat.value !== null ? chats.value[activeChat.value]?.isOpenclaw : false
    
    if (newChats.length > 0) {
      const newChatIds = new Set(newChats.map(c => c.id))

      newChats.forEach(newChat => {
        // Only skip chats that have peerAgentName (openclaw agent related)
        // Don't skip based on name containing '-lobster' - those are normal peer chats
        if (newChat.peerAgentName) {
          return
        }
        const existingIndex = chats.value.findIndex(c => c.id === newChat.id && !c.isOpenclaw)
        if (existingIndex !== -1) {
          chats.value[existingIndex].time = newChat.time
          chats.value[existingIndex].lastMessage = newChat.lastMessage
          chats.value[existingIndex].updated = newChat.updated
          chats.value[existingIndex].name = newChat.name
          if (newChat.members) chats.value[existingIndex].members = newChat.members
          chats.value[existingIndex].isTemp = false
        } else {
          chats.value.push(newChat)
        }
      })

      for (let i = chats.value.length - 1; i >= 0; i--) {
        if (!chats.value[i].isOpenclaw && !newChatIds.has(chats.value[i].id) && !chats.value[i].isTemp) {
          chats.value.splice(i, 1)
        }
      }
      
      if (savedChatId !== null) {
        const newIndex = chats.value.findIndex(c => c.id === savedChatId && c.isOpenclaw === savedIsOpenclaw)
        if (newIndex !== -1) {
          // activeChat.value = newIndex
        }
      } else if (activeChat.value === null && chats.value.length > 0) {
        // activeChat.value = 0
      }
    }
  } catch (error) {
    console.error('Failed to fetch chats:', error)
  }
}


const selectZeroClawSession = (session) => {
  // Close existing WebSocket connection
  if (zeroclawWS) {
    zeroclawWS.close()
    zeroclawWS = null
  }

  activeZeroClawSession.value = session
  activeChat.value = null
  activeOpenclawAgent.value = null
  loadZeroClawChatHistory(session)

  // Create new WebSocket connection
  zeroclawWS = new ZeroClawWS(
    'main',  // agentName - default to 'main'
    session.session_id,
    handleZeroClawMessage,
    handleZeroClawOpen,
    handleZeroClawClose,
    handleZeroClawError
  )
  zeroclawWS.connect()
}

const loadZeroClawChatHistory = async (session) => {
  try {
    const response = await zeroclawService.getMessages(session.session_id)
    if (response.data && response.data.messages) {
      session.messages = response.data.messages.map(msg => ({
        text: msg.content,
        sender: msg.role === 'user' ? (currentMeshAgentUsername.value || 'You') : (session.name || 'ZeroClaw'),
        time: new Date().toLocaleTimeString(),
        isSent: msg.role === 'user',
        isTemp: false
      }))
      session.isZeroClaw = true
    }
  } catch (error) {
    console.error('Failed to load zeroclaw chat history:', error)
  }
}

const formatChatTime = (rfc3339) => {
  if (!rfc3339) return ''
  try {
    const d = new Date(rfc3339)
    return d.getHours().toString().padStart(2, '0') + ':' + d.getMinutes().toString().padStart(2, '0')
  } catch {
    return ''
  }
}

const loadZAgentHistory = async (agentName, messages) => {
  if (messages.length > 0) return
  try {
    const response = await zeroclawService.getMessages(agentName, 'me')
    const history = response.data?.messages || []
    for (let i = 0; i < history.length; i++) {
      const msg = history[i]
      const sender = msg.role === 'user' ? (currentMeshAgentUsername.value || 'You') : agentName
      messages.push({
        text: msg.content,
        sender: sender,
        time: formatChatTime(msg.created_at),
        isSent: msg.role === 'user',
        isTemp: false,
        timestamp: msg.created_at ? new Date(msg.created_at).getTime() : 0
      })
    }
  } catch (e) {
    console.warn('[zAgent] Failed to load history:', e)
  }
}

const fetchZAgents = async () => {
  try {
    const response = await zagentService.getAgents()
    // Backend may return array directly or wrapped in { data: [...] }
    const agents = Array.isArray(response) ? response : (response.data || [])
    // Filter out agents with invalid agent_name to prevent phantom entries
    const validAgents = agents.filter(a => a && a.agent_name)
    // Move 0#Agent to the top of the list
    var zeroAgentIndex = -1
    for (var i = 0; i < validAgents.length; i++) {
      if (validAgents[i].agent_name === '0#Agent') {
        zeroAgentIndex = i
        break
      }
    }
    if (zeroAgentIndex > 0) {
      var zeroAgent = validAgents.splice(zeroAgentIndex, 1)[0]
      validAgents.unshift(zeroAgent)
    }
    zAgents.value = validAgents
  } catch (error) {
    console.error('Failed to fetch zAgents:', error)
  }
}

const fetchLocalGroupChats = async () => {
  try {
    const response = await groupChatService.getGroupChats()
    const chats = response.data || []
      localGroupChats.value = chats.map(c => ({
        groupId: c.group_id,
        groupName: c.group_name,
        ownerAgent: c.owner_agent,
        members: c.members,
        sessionId: c.session_id,
        messages: [],
        created_at: c.created_at,
        isGroupChat: true,
        name: c.group_name
      }))
  } catch (error) {
    console.error('Failed to fetch local group chats:', error)
  }
}

const createZAgent = async (agentConfig) => {
  try {
    const config = typeof agentConfig === 'string'
      ? { agent_name: agentConfig, display_name: agentConfig }
      : { display_name: agentConfig.agent_name, ...agentConfig }
    await zagentService.createAgent(config)
    await zagentService.startAgent(config.agent_name)
    await fetchZAgents()
  } catch (error) {
    console.error('Failed to create zAgent:', error)
    throw error
  }
}

const deleteZAgent = async (agentName) => {
  try {
    await zagentService.deleteAgent(agentName)
    await fetchZAgents()
    if (activeZAgent.value?.agent_name === agentName) {
      activeZAgent.value = null
    }
  } catch (error) {
    console.error('Failed to delete zAgent:', error)
    throw error
  }
}

const handleStartZAgent = async () => {
  const agentName = currentActiveChatId.value
  if (!agentName) return

  const agent = zAgents.value.find(a => a.agent_name === agentName)
  if (!agent) return

  try {
    await zagentService.startAgent(agentName)
    await fetchZAgents()
    // Re-select to reconnect WebSocket
    const updatedAgent = zAgents.value.find(a => a.agent_name === agentName)
    if (updatedAgent) {
      await selectZAgent(updatedAgent)
    }
  } catch (error) {
    console.error('Failed to start zAgent:', error)
  }
}

const createZeroClawMessageHandler = (connectionAgentName) => {
  return (data) => {
    const cached = wsConnections[connectionAgentName]
    if (!cached) return

    const messages = cached.messages
    if (!messages) return

    const session = activeZeroClawSession.value
    const agent = activeZAgent.value
    if (session) {
      return handleZeroClawMessage(data)
    }

    const senderName = agent?.display_name || agent?.agent_name || connectionAgentName

    const now = new Date()
    const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')

    if (data.type === 'session_start') {
      console.log('[ZeroClaw] Session started:', data.session_id)
    } else if (data.type === 'chunk' || data.type === 'thinking') {
      let idx = messages.findIndex(m => !!m.isTyping)
      if (idx < 0) {
        idx = messages.findIndex(function(m) {
          return !!m.isStreaming && m.sender === senderName && !m.isSent
        })
      }
      if (idx >= 0) {
        const newTimestamp = data.timestamp || now.getTime()
        if (data.type === 'thinking') {
          if (!messages[idx].thinking) messages[idx].thinking = ''
          messages[idx].thinking += data.content
        } else {
          messages[idx].text += data.content
        }
        messages[idx].timestamp = newTimestamp
        messages[idx].time = time
      } else {
        const typingIdx = messages.findIndex(m => !!m.isTyping)
        if (typingIdx >= 0) {
          if (data.type === 'thinking') {
            if (!messages[typingIdx].thinking) messages[typingIdx].thinking = ''
            messages[typingIdx].thinking += data.content
            messages[typingIdx].timestamp = now.getTime()
            messages[typingIdx].time = time
          } else {
            messages[typingIdx].text += data.content
            messages[typingIdx].isTyping = true
            messages[typingIdx].timestamp = now.getTime()
            messages[typingIdx].time = time
          }
        } else {
          messages.push({
            text: data.content,
            time: time,
            sender: senderName,
            timestamp: now.getTime(),
            isSent: false,
            isTemp: true,
            isStreaming: true
          })
        }
      }
    } else if (data.type === 'done') {
      // For agent voice mode, read the full reply aloud via TTS once fully streamed
      if (voiceCallStore?.isAgentMode?.value && data.full_response) {
        console.log('[App] zAgent WS done - triggering TTS, length:', data.full_response?.length)
        voiceCallStore.handleAgentResponse(data.full_response)
      }
      let idx = messages.findIndex(m => !!m.isTyping)
      if (idx >= 0) {
        messages[idx].text = data.full_response || messages[idx].text
        const hasThinking = !!messages[idx].thinking
        messages[idx].isTyping = false
        messages[idx].isTemp = false
        messages[idx].isStreaming = false
        if (hasThinking && !messages[idx].thinking) {
          messages.splice(idx, 1)
          return
        }
      } else {
        const lastIdx = messages.length - 1
        if (lastIdx >= 0) {
          const last = messages[lastIdx]
          if (last.isStreaming && last.sender === senderName && !last.isSent) {
            const hasThinking = !!last.thinking
            messages[lastIdx].text = data.full_response || messages[lastIdx].text
            messages[lastIdx].isStreaming = false
            messages[lastIdx].isTemp = false
            if (hasThinking && !messages[lastIdx].thinking) {
              messages.splice(lastIdx, 1)
              return
            }
          }
        }
      }
      var i
      for (i = 0; i < messages.length; i++) {
        if (messages[i].isTemp && !messages[i].isSent) {
          messages[i].isTemp = false
          messages[i].isStreaming = false
        }
      }
      sending.value = false
    } else if (data.type === 'error') {
      console.error('[ZeroClaw] Error:', data.message)
      const typingIdx = messages.findIndex(m => !!m.isTyping)
      if (typingIdx >= 0) {
        messages.splice(typingIdx, 1)
      }
      messages.push({
        text: 'Error: ' + (data.message || 'Unknown error'),
        time: time,
        sender: senderName,
        timestamp: now.getTime(),
        isSent: false,
        isTemp: false
      })
      sending.value = false
    } else if (data.type === 'tool_call') {
      console.log('[ZeroClaw] Tool call:', data.name, data.args)
    } else if (data.type === 'tool_result') {
      console.log('[ZeroClaw] Tool result:', data.name, data.output)
    }
  }
}

const selectZAgent = async (agent) => {
  const agentName = agent.agent_name

  // Bug fix: clear group chat so only one ChatMain is visible
  activeGroupId.value = null

  currentActiveChatId.value = agentName
  currentZAgentName = agentName

  // Check if we already have a cached connection to this agent
  const cached = wsConnections[agentName]
  if (cached && cached.zeroclawWS && cached.zeroclawWS.isConnected()) {
    console.log('[zAgent] Reusing cached connection for:', agentName)
    zeroclawWS = cached.zeroclawWS
    activeZeroClawSession.value = null
    activeChat.value = null
    activeOpenclawAgent.value = null
    activeZAgent.value = {
      ...agent,
      isZeroClaw: true,
      messages: cached.messages || [],
      port: cached.port
    }
    zcReconnectAttempts = 0
    return
  }

  activeZeroClawSession.value = null
  activeChat.value = null
  activeOpenclawAgent.value = null

  if (agent.status !== 'running') {
    console.log('[zAgent] Starting agent:', agent.agent_name)
    try {
      await zagentService.startAgent(agent.agent_name)
      await fetchZAgents()
    } catch (error) {
      console.error('[zAgent] Failed to start agent:', error)
      currentZAgentName = null
      return
    }
  }

  const latestAgent = zAgents.value.find(a => a.agent_name === agent.agent_name)
  const wsPort = latestAgent?.port
  zcReconnectAttempts = 0

  // Create agent-specific message handler
  const msgHandler = createZeroClawMessageHandler(agentName)

  const doConnect = () => {
    if (currentZAgentName !== agentName) return
    if (currentActiveChatId.value !== agentName) return
    
    zeroclawWS = new ZeroClawWS(
      agentName,
      'me',
      msgHandler,
      handleZeroClawOpen,
      handleZeroClawClose,
      handleZeroClawError,
      wsPort
    )
    zeroclawWS.connect()
  }

  const maxConnectAttempts = 5

  const handleConnectError = (error) => {
    handleZeroClawError(error)
    if (currentZAgentName !== agentName) return
    if (currentActiveChatId.value !== agentName) return
    if (!zeroclawWS || zeroclawWS.reconnectAttempts >= maxConnectAttempts) {
      console.log('[zAgent] Max connection attempts reached for:', agentName)
      if (zeroclawWS) zeroclawWS.reconnectAttempts = 0
      currentZAgentName = null
      return
    }
    
    zeroclawWS.reconnectAttempts++
    const delay = 1000 * zeroclawWS.reconnectAttempts
    console.log('[zAgent] Connection attempt ' + zeroclawWS.reconnectAttempts + '/' + maxConnectAttempts + ' failed, retrying in ' + delay + 'ms')
    setTimeout(doConnect, delay)
  }

  zeroclawWS = new ZeroClawWS(
    agentName,
    'me',
    msgHandler,
    handleZeroClawOpen,
    handleZeroClawClose,
    handleZeroClawError,
    wsPort
  )
  zeroclawWS.reconnectAttempts = 0
  zeroclawWS.onError = handleConnectError
  zeroclawWS._agentName = agentName
  zeroclawWS.connect()

  // Cache the connection with reference to messages array
  wsConnections[agentName] = reactive({
    zeroclawWS: zeroclawWS,
    port: wsPort,
    messages: [],
    _msgCount: 0
  })

  activeZAgent.value = {
    ...agent,
    isZeroClaw: true,
    messages: wsConnections[agentName].messages
  }

  // Load historical messages (oldest first) for display
  loadZAgentHistory(agentName, wsConnections[agentName].messages)
}

const handleZeroClawOpen = () => {
  console.log('[ZeroClaw] WebSocket connected')
  zcReconnectAttempts = 0
  currentZAgentName = null
}

let zcReconnectAttempts = 0
const maxZcReconnectAttempts = 5
let currentZAgentName = null

const handleZeroClawClose = (event) => {
  console.log('[ZeroClaw] WebSocket closed:', event.code, event.reason)
  
  const agent = activeZAgent.value
  const session = activeZeroClawSession.value
  if (!agent && !session) return
  
  if (event.code === 1000) return
  if (zeroclawWS && zeroclawWS.reconnectAttempts >= maxZcReconnectAttempts) {
    console.log('[ZeroClaw] Max reconnection attempts reached')
    if (zeroclawWS) zeroclawWS.reconnectAttempts = 0
    currentZAgentName = null
    return
  }
  
  if (agent && currentZAgentName !== agent.agent_name) {
    console.log('[ZeroClaw] Close handler ignored - agent changed')
    return
  }
  
  const agentNameToReconnect = agent?.agent_name || ''
  if (zeroclawWS && zeroclawWS._agentName && zeroclawWS._agentName !== agentNameToReconnect) {
    console.log('[ZeroClaw] Close handler ignored - agent changed')
    return
  }
  
  zcReconnectAttempts++
  const delay = 1000 * zcReconnectAttempts
  console.log('[ZeroClaw] Reconnecting... attempt ' + zcReconnectAttempts + '/' + maxZcReconnectAttempts + ' in ' + delay + 'ms')
  
  setTimeout(() => {
    if (currentZAgentName !== agentNameToReconnect) {
      console.log('[ZeroClaw] Reconnect ignored - agent changed')
      return
    }
    if (zeroclawWS) zeroclawWS.close()
    
    const cached = wsConnections[agentNameToReconnect]
    if (cached && cached.zeroclawWS) {
      cached.zeroclawWS.close()
    }
    
    if (agent && currentZAgentName === agentNameToReconnect && wsConnections[agentNameToReconnect]) {
      const msgHandler = createZeroClawMessageHandler(agentNameToReconnect)
      zeroclawWS = new ZeroClawWS(
        agentNameToReconnect,
        'me',
        msgHandler,
        handleZeroClawOpen,
        handleZeroClawClose,
        handleZeroClawError,
        agent.port || wsConnections[agentNameToReconnect].port
      )
      zeroclawWS.reconnectAttempts = zcReconnectAttempts - 1
      zeroclawWS._agentName = agentNameToReconnect
    } else if (session) {
      zeroclawWS = new ZeroClawWS(
        'main',
        session.session_id,
        handleZeroClawMessage,
        handleZeroClawOpen,
        handleZeroClawClose,
        handleZeroClawError
      )
    }
    if (zeroclawWS) zeroclawWS.connect()
  }, delay)
}

const handleZeroClawError = (error) => {
  console.error('[ZeroClaw] WebSocket error:', error)
  
  const agent = activeZAgent.value
  const session = activeZeroClawSession.value
  if (!agent && !session) return
  
  const agentNameToReconnect = agent?.agent_name || ''
  if (zeroclawWS && zeroclawWS._agentName && zeroclawWS._agentName !== agentNameToReconnect) {
    console.log('[ZeroClaw] Error handler ignored - agent changed')
    return
  }
  
  if (zeroclawWS && zeroclawWS.reconnectAttempts >= maxZcReconnectAttempts) {
    console.log('[ZeroClaw] Max reconnection attempts reached')
    if (zeroclawWS) zeroclawWS.reconnectAttempts = 0
    currentZAgentName = null
    return
  }
  
  if (agent && currentZAgentName !== agent.agent_name) {
    console.log('[ZeroClaw] Error handler ignored - agent changed')
    return
  }
  
  zcReconnectAttempts++
  const delay = 1000 * zcReconnectAttempts
  console.log('[ZeroClaw] Reconnecting after error... attempt ' + zcReconnectAttempts + '/' + maxZcReconnectAttempts + ' in ' + delay + 'ms')
  
  setTimeout(() => {
    if (currentZAgentName !== agentNameToReconnect) {
      console.log('[ZeroClaw] Error reconnect ignored - agent changed')
      return
    }
    if (zeroclawWS) zeroclawWS.close()
    
    const cached = wsConnections[agentNameToReconnect]
    if (cached && cached.zeroclawWS) {
      cached.zeroclawWS.close()
    }
    
    if (agent && currentZAgentName === agentNameToReconnect && wsConnections[agentNameToReconnect]) {
      const msgHandler = createZeroClawMessageHandler(agentNameToReconnect)
      zeroclawWS = new ZeroClawWS(
        agentNameToReconnect,
        'me',
        msgHandler,
        handleZeroClawOpen,
        handleZeroClawClose,
        handleZeroClawError,
        agent.port || wsConnections[agentNameToReconnect].port
      )
      zeroclawWS.reconnectAttempts = zcReconnectAttempts - 1
      zeroclawWS._agentName = agentNameToReconnect
    } else if (session) {
      zeroclawWS = new ZeroClawWS(
        'main',
        session.session_id,
        handleZeroClawMessage,
        handleZeroClawOpen,
        handleZeroClawClose,
        handleZeroClawError
      )
    }
    if (zeroclawWS) zeroclawWS.connect()
  }, delay)
}

// ── Task Management: parse <task> and <subtask> tags from AI responses ──

const parseTaskTags = (content, agentName) => {
  if (!content || !agentName) return

  // ── Step 1: Parse <task> and <subtask> XML tags ──
  const taskRegex = /<task\s+id="([^"]+)"\s+title="([^"]*)"(?:\s+status="([^"]*)")?(?:\s+progress="(\d+)")?[^>]*>([\s\S]*?)<\/task>/gi
  let taskMatch
  let foundXml = false
  while ((taskMatch = taskRegex.exec(content)) !== null) {
    foundXml = true
    const taskId = taskMatch[1]
    const title = taskMatch[2]
    const status = taskMatch[3] || 'pending'
    const progress = parseInt(taskMatch[4] || '0', 10)
    const description = taskMatch[5]?.trim() || ''

    taskService.getAgentTasks(agentName).then(res => {
      const existingTasks = res.data?.tasks || []
      let found = false
      function findTaskById(list) {
        for (let i = 0; i < list.length; i++) {
          if (list[i].task_id === taskId) { found = list[i]; return }
          if (list[i].subtasks && list[i].subtasks.length > 0) findTaskById(list[i].subtasks)
        }
      }
      findTaskById(existingTasks)

      if (found) {
        taskService.updateTask(taskId, { status, progress, title, description }).catch(e => console.warn('[Task] Update failed:', e))
      } else {
        taskService.createTask({
          task_id: taskId,
          agent_name: agentName,
          parent_id: null,
          title,
          description,
          status,
          progress,
          priority: 'normal'
        }).catch(e => console.warn('[Task] Create failed:', e))
      }
    }).catch(e => console.warn('[Task] Query failed:', e))
  }

  const subtaskRegex = /<subtask\s+parent="([^"]+)"\s+id="([^"]+)"\s+title="([^"]*)"(?:\s+status="([^"]*)")?(?:\s+progress="(\d+)")?[^>]*>([\s\S]*?)<\/subtask>/gi
  let subMatch
  while ((subMatch = subtaskRegex.exec(content)) !== null) {
    const parentId = subMatch[1]
    const taskId = subMatch[2]
    const title = subMatch[3]
    const status = subMatch[4] || 'pending'
    const progress = parseInt(subMatch[5] || '0', 10)
    const description = subMatch[6]?.trim() || ''

    taskService.getAgentTasks(agentName).then(res => {
      const existingTasks = res.data?.tasks || []
      let found = false
      function findTaskById(list) {
        for (let i = 0; i < list.length; i++) {
          if (list[i].task_id === taskId) { found = list[i]; return }
          if (list[i].subtasks && list[i].subtasks.length > 0) findTaskById(list[i].subtasks)
        }
      }
      findTaskById(existingTasks)

      if (found) {
        taskService.updateTask(taskId, { status, progress, title, description }).catch(e => console.warn('[Subtask] Update failed:', e))
      } else {
        taskService.createTask({
          task_id: taskId,
          agent_name: agentName,
          parent_id: parentId,
          title,
          description,
          status,
          progress,
          priority: 'normal'
        }).catch(e => console.warn('[Subtask] Create failed:', e))
      }
    }).catch(e => console.warn('[Subtask] Query failed:', e))
  }

  // ── P3: Fallback — parse markdown table / key-value / text format ──
  // Only trigger when no XML <task> tags were found and content looks like a task report
  if (!foundXml) {
    const isTaskContext = /(?:已创建|创建成功|新建|任务已|任务名称|task created|created task|new task|🆔|📛)/i.test(content)
    if (!isTaskContext) return

    // Extract task ID
    let extractedId = null
    const idPatterns = [
      // key: value format
      /(?:任务ID|Task\s*ID|🆔\s*(?:任务)?ID)[:：\s\t]+([a-f0-9\-]{8,}|[A-Z0-9\-]{8,})/i,
      // markdown table column format
      /(?:任务ID|Task\s*ID|🆔)\s*[|｜]\s*([a-f0-9\-]{8,}|[A-Z0-9\-]{8,})/i,
      /(?:编号|No)[:：\s\t]+([a-zA-Z0-9\-_]+)/i,
    ]
    for (const p of idPatterns) {
      const m = content.match(p)
      if (m) { extractedId = m[1]; break }
    }

    // Extract task name/title — handles both "key: value" and markdown table "key | value" formats
    let extractedName = null
    const namePatterns = [
      // key: value format
      /(?:任务名称|📛\s*任务名称|Task\s*Name|任务[:：])[:：\s\t]+([^\n]+)/i,
      // markdown table column format (header cell)
      /(?:任务名称|Task\s*Name)\s*[|｜]\s*([^\n|｜]+)/i,
      // Standalone line after "任务名称"
      /^[^：:]*(?:任务名称|Task\s*Name)[^：:]*$[^\n]*\n\s*([^\n]+)/im,
      /(?:标题|Title)[:：\s\t]+([^\n]+)/i,
    ]
    for (const p of namePatterns) {
      const m = content.match(p)
      if (m) { extractedName = m[1].replace(/[*#\s]+$/g, '').trim(); break }
    }

    // Extract status
    let extractedStatus = 'running'
    const statusPatterns = [
      /(?:状态|Status)[:：\s\t]+(pending|running|completed|failed)/i,
    ]
    for (const p of statusPatterns) {
      const m = content.match(p)
      if (m) { extractedStatus = m[1].toLowerCase(); break }
    }

    // Create task if we have at least a name (or context + id)
    if (extractedName || (isTaskContext && extractedId)) {
      const taskId = extractedId ? ('md-' + extractedId) : ('fb-' + Date.now())
      const title = extractedName || '未命名任务'
      const description = content.slice(0, 200)

      taskService.getAgentTasks(agentName).then(res => {
        const existingTasks = res.data?.tasks || []
        let found = false
        function findTaskById(list) {
          for (let i = 0; i < list.length; i++) {
            if (list[i].task_id === taskId) { found = list[i]; return }
            if (list[i].subtasks && list[i].subtasks.length > 0) findTaskById(list[i].subtasks)
          }
        }
        findTaskById(existingTasks)

        if (found) {
          taskService.updateTask(taskId, { status: extractedStatus, title, description }).catch(e => console.warn('[Task/Fallback] Update failed:', e))
        } else {
          taskService.createTask({
            task_id: taskId,
            agent_name: agentName,
            title,
            description,
            status: extractedStatus,
            progress: 0,
            priority: 'normal'
          }).catch(e => console.warn('[Task/Fallback] Create failed:', e))
        }
      }).catch(e => console.warn('[Task/Fallback] Query failed:', e))
    }
  }
}

const handleZeroClawMessage = (data) => {
  if (data.type && data.type.startsWith('voice-')) {
    const peerName = activeChat.value?.name || activeZAgent.value?.agent_name
    if (data.from && data.from === peerName) {
      voiceCallStore.handleIncomingMessage(data)
    }
    return
  }

  const session = activeZeroClawSession.value
  const agent = activeZAgent.value
  if (!session && !agent) return

  const target = session || agent
  const senderName = session?.name || agent?.display_name || agent?.agent_name || 'ZeroClaw'

  const now = new Date()
  const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')

  if (data.type === 'session_start') {
    console.log('[ZeroClaw] Session started:', data.session_id)
    if (session) {
      session.session_id = data.session_id || session.session_id
    }
  } else if (data.type === 'chunk' || data.type === 'thinking') {
    if (data.content) {
      const agentName = agent?.agent_name || 'main'
      parseTaskTags(data.content, agentName)
    }
    const typingIdx = target.messages?.findIndex(m => m.isTyping)
    if (typingIdx >= 0) {
      if (agent) {
        target.messages[typingIdx].text += data.content
        target.messages[typingIdx].timestamp = now.getTime()
      } else {
        target.messages[typingIdx] = {
          text: data.content,
          time: time,
          sender: senderName,
          timestamp: now.getTime(),
          isSent: false,
          isTemp: true,
          isStreaming: true
        }
      }
    } else if (target.messages) {
      target.messages.push({
        text: data.content,
        time: time,
        sender: senderName,
        timestamp: now.getTime(),
        isSent: false,
        isTemp: true,
        isStreaming: true
      })
    }
  } else if (data.type === 'done') {
    // For agent voice mode, read the full reply aloud via TTS once fully streamed
    if (voiceCallStore?.isAgentMode?.value && data.full_response) {
      console.log('[App] Triggering TTS for full_response, length:', data.full_response?.length)
      voiceCallStore.handleAgentResponse(data.full_response)
    }
    // Parse full_response for task tags (AI may have sent complete markdown/table in full msg)
    if (agent && data.full_response) {
      parseTaskTags(data.full_response, agent.agent_name || 'main')
    }
    const typingIdx = target.messages?.findIndex(m => m.isTyping)
    if (typingIdx >= 0) {
      if (agent) {
        target.messages[typingIdx].text = data.full_response || target.messages[typingIdx].text
        target.messages[typingIdx].isTyping = false
        target.messages[typingIdx].isTemp = false
        target.messages[typingIdx].isStreaming = false
      } else {
        target.messages.splice(typingIdx, 1)
      }
    }
    if (target.messages) {
      target.messages.forEach(m => {
        if (m.isTemp && !m.isSent) {
          m.isTemp = false
          m.isStreaming = false
        }
      })
    }
    sending.value = false
  } else if (data.type === 'error') {
    console.error('[ZeroClaw] Error:', data.message)
    const typingIdx = target.messages?.findIndex(m => m.isTyping)
    if (typingIdx >= 0) {
      target.messages.splice(typingIdx, 1)
    }
    target.messages?.push({
      text: 'Error: ' + (data.message || 'Unknown error'),
      time: time,
      sender: senderName,
      timestamp: now.getTime(),
      isSent: false,
      isTemp: false
    })
    sending.value = false
  } else if (data.type === 'tool_call') {
    console.log('[ZeroClaw] Tool call:', data.name, data.args)
  } else if (data.type === 'tool_result') {
    console.log('[ZeroClaw] Tool result:', data.name, data.output)
  }
}

const fetchZeroClawSessions = async () => {
  try {
    const response = await zeroclawService.getSessions()
    if (response.data && response.data.sessions) {
      zeroclawSessions.value = response.data.sessions
    }
  } catch (error) {
    console.error('Failed to fetch zeroclaw sessions:', error)
  }
}

const createSession = async (sessionName) => {
  // With WebSocket, session is created automatically when connecting
  // For now, just trigger a session fetch and try to select the session
  try {
    await fetchZeroClawSessions()
    let newSession = zeroclawSessions.value.find(s => s.session_id === sessionName)
    if (!newSession && sessionName) {
      // Create a placeholder session that will be created on WS connect
      newSession = {
        session_id: sessionName,
        name: sessionName,
        created_at: new Date().toISOString(),
        last_activity: new Date().toISOString(),
        message_count: 0
      }
      zeroclawSessions.value.push(newSession)
    }
    if (newSession) {
      selectZeroClawSession(newSession)
    }
  } catch (error) {
    console.error('Failed to create session:', error)
    throw error
  }
}

const selectChat = (index) => {
  activeOpenclawAgent.value = null
  activeChat.value = index
  if (chats.value[index]) {
    chats.value[index].updated = 0
    currentActiveChatId.value = chats.value[index].id
  }
}

// P1: Check if user message contains "任务" or "task" keyword and auto-create task
// P1: Auto-create task when user mentions "任务" or "task"
// Supports both zAgent and group chat (groupId passed for group mode)
const autoCreateUserTask = async (agentName, text, groupId) => {
  if (!text) return
  var lower = text.toLowerCase()
  if (lower.indexOf('任务') >= 0 || lower.indexOf('task') >= 0) {
    var taskId = 'TASK-' + Math.floor(1000 + Math.random() * 9000)

    // Extract title from user message
    var title = text
    var titleMatch = text.match(/(?:任务名称|创建一个任务|任务名字)[：:\s]*["']*([^"'\n]{2,})/i)
    if (titleMatch) {
      title = titleMatch[1].trim()
    } else {
      var colonMatch = text.match(/[：:]\s*([^\n]+)/)
      if (colonMatch) title = colonMatch[1].trim()
    }
    if (title.length === 0 || title.length > 100) {
      // fallback: use full text if extraction failed, trim to 100 chars
      title = text.slice(0, 100)
    }

    try {
      var res = await taskService.createTask({
        task_id: taskId,
        agent_name: agentName,
        group_id: groupId || null,
        title: title,
        description: text,
        status: 'running',
        progress: 0,
        priority: 'normal'
      })
      console.log('[Task] User-side task created: ' + taskId)
      // Async: ask 0#Agent to generate short title
      generateTaskTitleByAI(taskId, text)
    } catch (e) {
      console.warn('[Task] User-side create failed:', e)
    }
  }
}

// Ask 0#Agent to generate short_title (<8 chars) and ai_description
// Uses a temporary WebSocket connection (hidden from UI)
const generateTaskTitleByAI = async (taskId, originalText) => {
  try {
    var agentsRes = await zagentService.getAgents()
    var zeroAgent = agentsRes.data.find(function(a) { return a.agent_name === '0#Agent' })
    if (!zeroAgent || !zeroAgent.port) { console.log('[TaskTitle] 0#Agent not found'); return }

    var sessionId = 'sys-title-gen-' + Date.now()
    var fullResponse = ''
    var hasResponded = false

    var ws = new ZeroClawWS(
      '0#Agent',
      sessionId,
      function(data) {
        if (data.type === 'chunk') fullResponse += data.content
        else if (data.type === 'done') {
          hasResponded = true
          ws.close()
          var nameMatch = fullResponse.match(/名字[：:]\s*([^\n]+)/)
          var descMatch = fullResponse.match(/描述[：:]\s*([^\n]+)/)
          if (nameMatch) {
            var shortTitle = nameMatch[1].replace(/[\s*#]/g, '').trim().slice(0, 8)
            var aiDesc = descMatch ? descMatch[1].replace(/[\s*#]/g, '').trim() : ''
            taskService.updateTask(taskId, {
              short_title: shortTitle,
              ai_description: aiDesc
            }).then(function() {
              console.log('[TaskTitle] Updated ' + taskId + ' with: ' + shortTitle)
            }).catch(function() {})
          }
        }
      },
      function() {
        ws.sendMessage('[SYSTEM] 请为这个任务起短名字（<8字）和描述：' + originalText + '\n\n用以下格式回复：\n名字：xxx\n描述：xxx')
      },
      function() {}, 
      function() {},
      zeroAgent.port
    )
    ws.connect()

    setTimeout(function() {
      if (!hasResponded) { ws.close(); console.log('[TaskTitle] Timeout for ' + taskId) }
    }, 8000)
  } catch (e) {
    console.warn('[TaskTitle] Failed:', e)
  }
}

const handleZAgentTemplateCreated = async (data) => {
  const { agentName, industrySlug, agentSlug, source, displayName } = data

  const prompt = `[系统初始化任务] 新 zAgent: ${agentName}

请帮我初始化这个 zAgent 的 workspace：

- **Agent 名称**: ${agentName}
- **显示名称**: ${displayName}
- **模板来源**: ${source}
- **行业目录**: ${industrySlug}
- **Agent 目录**: ${agentSlug}

请按以下步骤操作：

1. **定位模板目录**:
   - 如果 source=shared: ~/.clawparty/.agent-template/.shared/${industrySlug}/${agentSlug}/
   - 如果 source=local: ~/.clawparty/.agent-template/${industrySlug}/${agentSlug}/

2. **查找 zAgent workspace**: ~/.clawparty/agents/${agentName}/workspace/

3. **复制模板文件**: 把模板目录下所有 .md 文件（不区分大小写，包括 identity.md、IDENTITY.md、soul.md、SOUL.md、agents.md 等所有 .md 文件）复制到 zAgent 的 workspace 目录

4. **占位符替换**: 在复制后的所有 .md 文件中：
   - 把 {{AGENT_NAME}} 替换为 ${agentName}
   - 把 {{DISPLAY_NAME}} 替换为 ${displayName}

5. **LLM Wiki 方法论**: 检查模板目录中是否存在 llm-wiki.md 文件。如果存在，读取其内容，并将 wiki 方法论注入到 zAgent workspace 的 AGENTS.md（或 agents.md）中。如果不存在，跳过此步骤。

6. **工作追踪规则**: 确保 zAgent workspace 的 SOUL.md 或 AGENTS.md 中有完整的工作追踪规则（使用 <task> 和 <subtask> XML 标签来追踪进度）。如果没有，请在 AGENTS.md 中添加。

7. **格式优化**: 检查所有 .md 文件的排版和格式

完成后回复："✅ ${agentName} 初始化完成"`

  // Ensure 0#Agent is connected
  let conn = wsConnections['0#Agent']
  if (!conn || !conn.zeroclawWS || !conn.zeroclawWS.isConnected()) {
    const agentsRes = await zagentService.getAgents()
    const zeroAgent = agentsRes.data.find(a => a.agent_name === '0#Agent')
    if (!zeroAgent) { console.warn('[ZAgentInit] 0#Agent not found'); return }
    await selectZAgent(zeroAgent)
    conn = wsConnections['0#Agent']
  }

  if (conn && conn.zeroclawWS && conn.zeroclawWS.isConnected()) {
    const now = new Date()
    const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
    conn.messages.push({
      text: prompt,
      time: time,
      sender: currentMeshAgentUsername.value || 'You',
      timestamp: now.getTime(),
      isSent: true,
      isTemp: false
    })
    conn.messages.push({
      text: '',
      time: time,
      sender: '0#Agent',
      timestamp: now.getTime() + 1,
      isSent: false,
      isTyping: true
    })
    conn._msgCount = conn.messages.length
    conn.zeroclawWS.sendMessage(prompt)
  } else {
    console.warn('[ZAgentInit] 0#Agent WebSocket not connected')
  }
}

const handleZAgentSend = (agentName, text) => {
  console.log('[zAgent] handleZAgentSend called:', agentName, 'text:', JSON.stringify(text))

  // P1: Auto-create task if user mentions "任务" or "task"
  autoCreateUserTask(agentName, text)

  const cached = wsConnections[agentName]
  if (!cached) {
    console.error('[zAgent] No connection found for:', agentName)
    return
  }
  
  const zagent = zAgents.value.find(a => a.agent_name === agentName)
  const displayName = zagent?.display_name || zagent?.agent_name || 'ZeroClaw'
  
  const now = new Date()
  const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
  
  const newMsg = {
    text: text,
    time: time,
    sender: currentMeshAgentUsername.value || 'You',
    timestamp: now.getTime(),
    isSent: true,
    isTemp: true
  }
  const typingMsg = {
    text: '',
    time: time,
    sender: displayName,
    timestamp: now.getTime() + 1,
    isSent: false,
    isTyping: true
  }
  
  if (!cached.messages) cached.messages = []
  cached.messages.push(newMsg, typingMsg)
  cached._msgCount = cached.messages.length
  
  // Update activeZAgent to reference the same messages array
  activeZAgent.value = {
    ...zagent,
    display_name: displayName,
    isZeroClaw: true,
    messages: cached.messages,
    port: cached.port
  }
  
  newMessage.value = ''
  
  if (cached.zeroclawWS && cached.zeroclawWS.isConnected()) {
    cached.zeroclawWS.sendMessage(text)
  } else {
    cached.messages.push({
      text: 'WebSocket not connected. Please try again.',
      time: time,
      sender: displayName,
      timestamp: new Date().getTime(),
      isSent: false,
      isTemp: false
    })
    cached._msgCount = cached.messages.length
  }
}

const handleChatSend = (chatId, text) => {
  const chat = chats.value.find(c => c.id === chatId)
  if (!chat) {
    console.error('[Chat] Chat not found:', chatId)
    return
  }
  
  if (!chat.messages) chat.messages = []

  const now = new Date()
  const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
  
  chat.messages.push({
    text: text,
    time: time,
    sender: currentMeshAgentUsername.value || 'You',
    timestamp: now.getTime(),
    isSent: true,
    isTemp: true
  })
  
  newMessage.value = ''
  
  if (chat.isGroup) {
    chatService.sendGroupMessage(currentMesh.value, chat.creator, chat.gcid, text).catch(err => {
      console.error('[Chat] Failed to send group message:', err)
    })
  } else {
    chatService.sendMessage(currentMesh.value, chat.name, text).catch(err => {
      console.error('[Chat] Failed to send peer message:', err)
    })
  }
}

const handleChatBack = (chatId) => {
  currentActiveChatId.value = null
}

// ── Local Group Chat Functions ───────────────────────────────────────

const enterGroupChat = async (groupId) => {
  const group = localGroupChats.value.find(g => g.groupId === groupId)
  if (!group) return

  // Clear other active chat states, but do NOT close zeroclawWS here —
  // that belongs to the zAgent connection cache and will be reused when switching back.
  activeOpenclawAgent.value = null
  activeZeroClawSession.value = null
  activeZAgent.value = null
  activeChat.value = null
  currentZAgentName = null
  currentActiveChatId.value = null

  activeGroupId.value = groupId
  const allMembers = [group.ownerAgent, ...group.members]

  // ── Auto-start non-running member agents ────────────────────────────
  const startPromises = allMembers.map(async (agentName) => {
    const agent = zAgents.value.find(a => a.agent_name === agentName)
    if (!agent || agent.status === 'running') return { agentName, ok: true }
    try {
      console.log('[GroupChat] Auto-starting member:', agentName)
      await zagentService.startAgent(agentName)
      return { agentName, ok: true }
    } catch (e) {
      console.error('[GroupChat] Failed to start member:', agentName, e)
      return { agentName, ok: false }
    }
  })
  await Promise.all(startPromises)

  // ── Wait for all members to reach running state ─────────────────────
  const MAX_WAIT_MS = 15000
  const POLL_INTERVAL_MS = 1000
  const waitStart = Date.now()
  while (Date.now() - waitStart < MAX_WAIT_MS) {
    await fetchZAgents()
    const allRunning = allMembers.every(name => {
      const agent = zAgents.value.find(a => a.agent_name === name)
      return agent?.status === 'running'
    })
    if (allRunning) {
      console.log('[GroupChat] All members running')
      break
    }
    await new Promise(r => setTimeout(r, POLL_INTERVAL_MS))
  }

  // Close old WS connections for this group if any
  const oldConnections = activeGroupWsMap.get(groupId)
  if (oldConnections) {
    for (const conn of oldConnections) {
      if (conn.ws) conn.ws.close()
    }
  }

  const connections = []
  for (const agentName of allMembers) {
    const agent = zAgents.value.find(a => a.agent_name === agentName)
    const wsPort = agent?.port

    const msgHandler = createGroupChatMessageHandler(groupId, agentName)

    const ws = new ZeroClawWS(
      agentName,
      groupId,  // use groupId as session_id for shared session context
      msgHandler,
      () => { console.log('[GroupChat] WS open:', agentName, groupId) },
      () => { console.log('[GroupChat] WS close:', agentName, groupId) },
      (err) => { console.error('[GroupChat] WS error:', agentName, groupId, err) },
      wsPort
    )
    ws.connect()
    connections.push({ agentName, ws })
  }

  activeGroupWsMap.set(groupId, connections)

  // Load historical messages from all member agents
  try {
    const res = await groupChatService.getGroupMessages(group.groupId)
    if (res.data && res.data.messages) {
      const parsed = []
      for (const msg of res.data.messages) {
        let text = msg.content || ''
        let sender
        let isSent = false

        if (msg.role === 'user') {
          // Parse injected text to recover original message and sender
          // Formats:
          //   在群聊"..."里，{sender} 说："..."，如果...
          //   在群聊"..."里，{sender} @了你并说："..."，请回复。
          //   在群聊"..."里，{sender} 对其他人说："..."，如果...
          const m = text.match(/在群聊"[^"]*"里，(.+?)(?:@了你并)?(?:对其他人)?说："([\s\S]*?)"[，。]/)
          if (m) {
            sender = m[1].trim()
            text = m[2]
          } else {
            sender = currentMeshAgentUsername.value || 'You'
          }
          isSent = sender === (currentMeshAgentUsername.value || 'You')
        } else {
          // assistant message — use the tagged agent name
          sender = msg._agentName || group.ownerAgent
          // Filter out NO_REPLY responses
          if (text.trim() === 'NO_REPLY' || text.trim() === '不回复') continue
        }

        parsed.push({
          text,
          sender,
          time: formatTime(msg.created_at),
          timestamp: msg.created_at ? new Date(msg.created_at).getTime() : 0,
          isSent,
          isTemp: false
        })
      }
      group.messages = parsed
    }
  } catch (e) {
    console.error('[GroupChat] Failed to load messages:', e)
  }
}

const leaveGroupChat = (groupId) => {
  const connections = activeGroupWsMap.get(groupId)
  if (connections) {
    for (const conn of connections) {
      if (conn.ws) conn.ws.close()
    }
  }
  activeGroupWsMap.delete(groupId)
  if (activeGroupId.value === groupId) {
    activeGroupId.value = null
  }
}

const createGroupChatMessageHandler = (groupId, agentName) => {
  return (data) => {
    const group = localGroupChats.value.find(g => g.groupId === groupId)
    if (!group) return

    switch (data.type) {
    case 'chunk': {
      if (data.content) {
        parseTaskTags(data.content, agentName)
      }
      // Find the most recent typing message for this agent (from end)
      var typingMsg = null
      for (var i = group.messages.length - 1; i >= 0; i--) {
        if (group.messages[i].isTyping && group.messages[i].agentName === agentName) {
          typingMsg = group.messages[i]
          break
        }
      }
      if (typingMsg) {
        typingMsg.text += data.content || ''
      } else {
        group.messages.push({
          text: data.content || '',
          sender: agentName,
          agentName: agentName,
          time: formatTime(new Date().toISOString()),
          timestamp: Date.now(),
          isSent: false,
          isTyping: true
        })
      }
      break
    }
    case 'done': {
      // Parse full response for task tags
      if (data.full_response) {
        parseTaskTags(data.full_response, agentName)
      }
      const replyText = data.full_response || ''
      const isNoReply = replyText.includes('NO_REPLY') || replyText.includes('不回复')

      // Find the most recent typing message for this agent (from end)
      var lastTyping = null
      var lastTypingIdx = -1
      for (var j = group.messages.length - 1; j >= 0; j--) {
        if (group.messages[j].isTyping && group.messages[j].agentName === agentName) {
          lastTyping = group.messages[j]
          lastTypingIdx = j
          break
        }
      }
      if (lastTyping) {
        if (isNoReply) {
          group.messages.splice(lastTypingIdx, 1)
        } else {
          lastTyping.isTyping = false
          lastTyping.text = replyText
        }
      }

      // Persist agent response to chat_log
      if (replyText && !isNoReply) {
        try {
          groupChatService.sendGroupMessage(groupId, agentName, replyText, 'response')
        } catch (e) { /* non-blocking */ }
      }

      // Broadcast this agent's reply to all other agents in the group (skip if NO_REPLY)
      // Only broadcast user-originated messages, not agent-to-agent broadcasts, to prevent message storms.
      // We track this via conn._isBroadcastPending flag set when sending a broadcast message.
      if (replyText && !isNoReply) {
        const groupName = group.groupName || '群聊'
        const connections = activeGroupWsMap.get(groupId)
        // Find this agent's own connection to check if it was triggered by a broadcast
        const ownConn = connections ? connections.find(c => c.agentName === agentName) : null
        const wasTriggeredByBroadcast = ownConn && ownConn._isBroadcastPending
        if (ownConn) ownConn._isBroadcastPending = false

        if (!wasTriggeredByBroadcast && connections) {
          const injectedReply = `在群聊"${groupName}"里，${agentName} 说："${replyText}"，如果不需要你回复，请只回复 NO_REPLY。`
          for (const conn of connections) {
            if (conn.agentName !== agentName && conn.ws && conn.ws.isConnected()) {
              conn._isBroadcastPending = true
              conn.ws.sendMessage(injectedReply)
            }
          }
        }
      }
      break
    }
    case 'tool_call': {
      group.messages.push({
        text: `[Tool call: ${data.name}]`,
        sender: agentName,
        agentName: agentName,
        time: formatTime(new Date().toISOString()),
        timestamp: Date.now(),
        isSent: false,
        isSystem: true
      })
      break
    }
    case 'error': {
      group.messages.push({
        text: '[Error: ' + (data.message || 'Unknown error') + ']',
        sender: agentName,
        agentName: agentName,
        time: formatTime(new Date().toISOString()),
        timestamp: Date.now(),
        isSent: false,
        isError: true
      })
      break
    }
    default:
      console.log('[GroupChat] Unhandled message type:', data.type, 'from', agentName)
    }
  }
}

const handleDeleteLocalGroup = async (group) => {
  if (!confirm(`删除群聊 "${group.groupName}"?`)) return
  try {
    await groupChatService.deleteGroupChat(group.groupId)
    leaveGroupChat(group.groupId)
    const idx = localGroupChats.value.findIndex(g => g.groupId === group.groupId)
    if (idx >= 0) localGroupChats.value.splice(idx, 1)
  } catch (error) {
    console.error('[GroupChat] Failed to delete group:', error)
  }
}

const sendMessage = async () => {
  if (!newMessage.value.trim() || sending.value) return
  const text = newMessage.value
  sending.value = true

  // Safety timeout: reset sending state after 30 seconds to prevent permanent UI freeze
  const sendingTimeout = setTimeout(() => {
    if (sending.value) {
      console.warn('[sendMessage] Timeout: resetting sending state')
      sending.value = false
    }
  }, 30000)

  // Local group chat sending via WebSocket
  if (activeGroupId.value) {
    const group = localGroupChats.value.find(g => g.groupId === activeGroupId.value)
    if (group) {
      // Auto-create task for group chat if user mentions keyword
      autoCreateUserTask(group.ownerAgent, text, group.groupId)

      const now = new Date()
      const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')

      // Record user message locally
      group.messages.push({
        text: text,
        sender: currentMeshAgentUsername.value || 'You',
        agentName: 'user',
        time: time,
        timestamp: now.getTime(),
        isSent: true,
        isTemp: true
      })

      // Persist user message to chat_log
      try {
        groupChatService.sendGroupMessage(group.groupId, senderName, text, 'user')
      } catch (e) { /* non-blocking */ }

      // Send to all member agents via WS with prompt injection
      const connections = activeGroupWsMap.get(group.groupId)
      if (connections) {
        const groupName = group.groupName || '群聊'
        const senderName = currentMeshAgentUsername.value || 'You'

        // Parse @mentions from the message
        const mentionPunctChars = ' ,.!?;:\'"、。！？；："'
        const mentionedNames = []
        text.split(' ').forEach(token => {
          if (token.length > 1 && token.charAt(0) === '@') {
            let name = token.substring(1)
            for (let i = 0; i < name.length; i++) {
              if (mentionPunctChars.includes(name.charAt(i))) { name = name.substring(0, i); break }
            }
            if (name.length > 0) mentionedNames.push(name.toLowerCase())
          }
        })
        const hasMentions = mentionedNames.length > 0
        // Clean text: strip @name tokens
        const cleanedText = hasMentions
          ? text.split(' ').filter(t => !(t.length > 1 && t.charAt(0) === '@')).join(' ').trim()
          : text

        for (const conn of connections) {
          if (!conn.ws || !conn.ws.isConnected()) continue
          const isMentioned = hasMentions && mentionedNames.includes(conn.agentName.toLowerCase())
          let injectedText
          if (isMentioned) {
            injectedText = `在群聊"${groupName}"里，${senderName} @了你并说："${cleanedText}"，请回复。`
          } else if (hasMentions) {
            // Someone else was @-mentioned — agent just observes
            injectedText = `在群聊"${groupName}"里，${senderName} 对其他人说："${text}"，如果这条消息不需要你参与，请只回复 NO_REPLY。`
          } else {
            // No mentions — all agents should consider responding
            injectedText = `在群聊"${groupName}"里，${senderName} 说："${text}"，请根据你的角色参与群聊，如果这条消息完全不需要你回复，请只回复 NO_REPLY。`
          }
          conn.ws.sendMessage(injectedText)
        }
      }
      newMessage.value = ''
      clearTimeout(sendingTimeout)
      sending.value = false
      return
    }
  }

  if (!activeOpenclawAgent.value && !activeZeroClawSession.value && !activeZAgent.value && activeChat.value === null) {
    sending.value = false
    return
  }

  const chat = activeOpenclawAgent.value || chats.value[activeChat.value]

  // zAgent message sending via WebSocket - delegate to handleZAgentSend
  if (activeZAgent.value && currentActiveChatId.value) {
    handleZAgentSend(currentActiveChatId.value, text)
    sending.value = false
    return
  }

  // ZeroClaw message sending via WebSocket
  if (activeZeroClawSession.value) {
    const session = activeZeroClawSession.value
    if (!session.messages) session.messages = []
    const now = new Date()
    const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
    
    // Add user message
    session.messages.push({
      text: text,
      time: time,
      sender: currentMeshAgentUsername.value || 'You',
      timestamp: now.getTime(),
      isSent: true,
      isTemp: true
    })
    
    // Add typing indicator
    session.messages.push({
      text: '',
      time: time,
      sender: session.name || 'ZeroClaw',
      timestamp: now.getTime() + 1,
      isSent: false,
      isTyping: true
    })
    
    newMessage.value = ''
    
    // Send via WebSocket
    if (zeroclawWS && zeroclawWS.isConnected()) {
      zeroclawWS.sendMessage(text)
    } else {
      // WebSocket not connected, show error
      const typingIdx = session.messages.findIndex(m => m.isTyping)
      if (typingIdx >= 0) {
        session.messages.splice(typingIdx, 1)
      }
      session.messages.push({
        text: 'WebSocket not connected. Please select the session again.',
        time: time,
        sender: session.name || 'ZeroClaw',
        timestamp: new Date().getTime(),
        isSent: false,
        isTemp: false
      })
      sending.value = false
    }
    
    return
  }

  
  try {
      if (chat.isOpenclaw) {
        if (!chat.messages) chat.messages = []
        const now = new Date()
        const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
        
        setTimeout(()=>{
          chat.messages.push({
            text: '',
            time: time,
            sender: chat.name,
            timestamp: now.getTime(),
            isTyping: true
          })
        },300)
      
      // Detect #"group name" or #'group name' token — send message to that group as this agent
      const groupTokenMatch = text.match(/#["']([^"']+)["']/)
      if (groupTokenMatch) {
        const groupName = groupTokenMatch[1]
        const cleanText = text.replace(/#["'][^"']+["']\s*/g, '').trim()
        const targetGroup = chats.value.find(c => c.isGroup && !c.isOpenclaw && c.name === groupName)
        if (targetGroup && cleanText && currentMesh.value) {
          chatService.sendGroupMessageAsAgent(
            currentMesh.value, targetGroup.gcid, chat.agentId, cleanText
          ).catch(err => console.error('Failed to send to group as agent:', err))
        }
      }

      openclawService.sendMessage(chat.agentId, text).then((response)=>{
        const payloads = response.data?.payloads || response.data?.result?.payloads || [];
        const replyText = payloads.map(p => p?.text).filter(Boolean).join('\n\n');
        
        const typingIndex = chat.messages.findIndex(m => m.isTyping)
        if (typingIndex !== -1) {
          chat.messages.splice(typingIndex, 1)
        }
        if (replyText) {
          const replyTime = new Date().getHours().toString().padStart(2, '0') + ':' + new Date().getMinutes().toString().padStart(2, '0')
          chat.messages.push({
            text: replyText,
            time: replyTime,
            sender: chat.name,
            timestamp: new Date().getTime(),
            isTemp: false
          })
          chat.lastMessage = replyText
          chat.time = replyTime
        }
      
      }).catch((e)=>{
        
        const typingIndex = chat.messages.findIndex(m => m.isTyping)
        if (typingIndex !== -1) {
          chat.messages.splice(typingIndex, 1)
        }
        let replyText = localOpenclawAvailable.value
          ? 'Response timed out, please refresh.'
          : 'openclaw is not installed locally. You can still interact with remote openclaw agents via group chat.'
        if (replyText) {
          const replyTime = new Date().getHours().toString().padStart(2, '0') + ':' + new Date().getMinutes().toString().padStart(2, '0')
          chat.messages.push({
            text: replyText,
            time: replyTime,
            sender: chat.name,
            timestamp: new Date().getTime(),
            isTemp: false
          })
          chat.lastMessage = replyText
          chat.time = replyTime
        }
      })
    } else if (chat.isGroup) {
      const groupParts = [currentMeshAgentUsername.value, chat.gcid].sort()
      const groupSessionId = groupParts[0] + '~' + groupParts[1]
      await chatService.sendGroupMessage(currentMesh.value, chat.creator, chat.groupId, text, groupSessionId)
    } else {
      const peerParts = [currentMeshAgentUsername.value, chat.name].sort()
      const peerSessionId = peerParts[0] + '~' + peerParts[1]
      await chatService.sendMessage(currentMesh.value, chat.name, text, peerSessionId)
    }
  } catch (error) {
    console.error('Failed to send message:', error)
  } finally {
    sending.value = false
  }
  
  const now = new Date()
  const time = now.getHours().toString().padStart(2, '0') + ':' + 
               now.getMinutes().toString().padStart(2, '0')
  
  if (!chat.messages) {
    chat.messages = []
  }
  
  chat.messages.push({
    text: text,
    time: time,
    sender: currentMeshAgentUsername.value,
    timestamp: now.getTime(),
    isTemp: true,
    isSent: true
  })
  
  chat.lastMessage = text
  chat.time = time
  
  newMessage.value = ''
}

const handleSendImages = async (imageFiles) => {
  if (!imageFiles || imageFiles.length === 0) return
  if (!activeOpenclawAgent.value && activeChat.value === null) return
  const chat = activeOpenclawAgent.value || chats.value[activeChat.value]

  if (chat.isOpenclaw) {
    // Local openclaw agent: save pictures to agent workspace and show in chat
    try {
      const picturePaths = []
      for (let i = 0; i < imageFiles.length; i++) {
        const file = imageFiles[i]
        const fileName = file.name || ('img_' + Date.now() + '_' + i + '.png')
        const arrayBuffer = await file.arrayBuffer()
        const res = await openclawService.uploadPicture(chat.agentId, arrayBuffer, fileName)
        const data = typeof res.data === 'string' ? JSON.parse(res.data) : res.data
        if (data && data.name) {
          picturePaths.push({ 
            name: data.name, 
            path: data.path, 
            url: openclawService.getPictureUrl(chat.agentId, data.name),
            type: file.type || 'image/png'  // Preserve original file type
          })
        }
      }
      if (picturePaths.length === 0) return

      // Display pictures in local chat
      const now = new Date()
      const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
      if (!chat.messages) chat.messages = []
      chat.messages.push({
        text: '',
        files: picturePaths.map(p => ({ 
          name: p.name, 
          url: p.url,
          type: p.type
        })),
        time: time,
        sender: 'You',
        timestamp: now.getTime(),
        isSent: true
      })

      // Send the file paths as message to the agent
      const userText = newMessage.value.trim()
      const agentMessage = userText
        ? userText + '\n\n对方发送了一个图片，保存在：' + picturePaths.map(p => p.path).join('，')
        : '对方发送了一个图片，保存在：' + picturePaths.map(p => p.path).join('，')

      sending.value = true
      newMessage.value = ''
      setTimeout(() => {
        chat.messages.push({ text: '', time: time, sender: chat.name, timestamp: now.getTime() + 1, isTyping: true })
      }, 300)

      openclawService.sendMessage(chat.agentId, agentMessage).then((resp) => {
        const payloads = resp.data?.payloads || resp.data?.result?.payloads || [];
        const replyText = payloads.map(p => p?.text).filter(Boolean).join('\n\n')
        const typingIndex = chat.messages.findIndex(m => m.isTyping)
        if (typingIndex !== -1) chat.messages.splice(typingIndex, 1)
        if (replyText) {
          const replyTime = new Date().getHours().toString().padStart(2, '0') + ':' + new Date().getMinutes().toString().padStart(2, '0')
          chat.messages.push({ text: replyText, time: replyTime, sender: chat.name, timestamp: new Date().getTime() })
          chat.lastMessage = replyText
          chat.time = replyTime
        }
        sending.value = false
      }).catch(() => {
        const typingIndex = chat.messages.findIndex(m => m.isTyping)
        if (typingIndex !== -1) chat.messages.splice(typingIndex, 1)
        sending.value = false
      })
    } catch (error) {
      console.error('Failed to send images to openclaw agent:', error)
    }
    return
  }

  // Mesh chat: upload to mesh filesystem
  if (!currentMesh.value) return
  try {
    const uploadedFiles = []
    const savedPaths = []
    for (let i = 0; i < imageFiles.length; i++) {
      const file = imageFiles[i]
      const arrayBuffer = await file.arrayBuffer()
      const response = await chatService.uploadFile(currentMesh.value, arrayBuffer)
      const hash = typeof response.data === 'string' ? response.data : ''
      if (hash) {
        uploadedFiles.push({
          hash,
          name: file.name || 'image',
          type: file.type || 'image/png',
          size: file.size || 0,
          owner: currentMeshAgentUsername.value
        })
        savedPaths.push(`/shared/${currentMeshAgentUsername.value}/publish/files/${hash}`)
      }
    }
    if (uploadedFiles.length === 0) return

    const text = newMessage.value.trim()
    if (chat.isGroup) {
      const groupParts = [currentMeshAgentUsername.value, chat.gcid].sort()
      const groupSessionId = groupParts[0] + '~' + groupParts[1]
      await chatService.sendGroupMessage(currentMesh.value, chat.creator, chat.groupId, text, groupSessionId, uploadedFiles)
    } else {
      const peerParts = [currentMeshAgentUsername.value, chat.name].sort()
      const peerSessionId = peerParts[0] + '~' + peerParts[1]
      await chatService.sendMessage(currentMesh.value, chat.name, text, peerSessionId, uploadedFiles)
    }
    newMessage.value = ''
    
    // Show notification about saved paths for images
    if (savedPaths.length > 0) {
      const now = new Date()
      const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
      if (!chat.messages) chat.messages = []
      chat.messages.push({
        text: `[图片保存到以下路径]\n${savedPaths.join('\n')}`,
        time: time,
        sender: '系统',
        timestamp: now.getTime(),
        isSent: true,
        isSystemHint: true
      })
    }
  } catch (error) {
    console.error('Failed to send images:', error)
  }
}

const handleSendFiles = async (files) => {
  if (!files || files.length === 0) return
  if (!activeOpenclawAgent.value && activeChat.value === null) return
  const chat = activeOpenclawAgent.value || chats.value[activeChat.value]

  // OpenCLaw agent: save files to agent workspace
  if (chat.isOpenclaw) {
    const imageFiles = []
    const otherFiles = []
    for (let i = 0; i < files.length; i++) {
      const file = files[i]
      if (file.type && file.type.startsWith('image/')) {
        imageFiles.push(file)
      } else {
        otherFiles.push(file)
      }
    }

    // Handle images: show preview in chat
    if (imageFiles.length > 0) {
      const picturePaths = []
      for (let i = 0; i < imageFiles.length; i++) {
        const file = imageFiles[i]
        const fileName = file.name || ('img_' + Date.now() + '_' + i + '.png')
        const arrayBuffer = await file.arrayBuffer()
        try {
          const res = await openclawService.uploadPicture(chat.agentId, arrayBuffer, fileName)
          const data = typeof res.data === 'string' ? JSON.parse(res.data) : res.data
          if (data && data.name) {
            picturePaths.push({
              name: data.name,
              path: data.path,
              url: openclawService.getPictureUrl(chat.agentId, data.name),
              type: file.type || 'image/png'
            })
          }
        } catch (error) {
          console.error('Failed to upload image:', error)
        }
      }
      if (picturePaths.length > 0) {
        const now = new Date()
        const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
        if (!chat.messages) chat.messages = []
        chat.messages.push({
          text: '',
          files: picturePaths.map(p => ({ name: p.name, url: p.url, type: p.type })),
          time: time,
          sender: 'You',
          timestamp: now.getTime(),
          isSent: true
        })
      }
    }

    // Handle other files: show text notification
    if (otherFiles.length > 0) {
      const savedPaths = []
      for (let i = 0; i < otherFiles.length; i++) {
        const file = otherFiles[i]
        const fileName = file.name || ('file_' + Date.now() + '_' + i)
        const arrayBuffer = await file.arrayBuffer()
        try {
          const res = await openclawService.uploadPicture(chat.agentId, arrayBuffer, fileName)
          const data = typeof res.data === 'string' ? JSON.parse(res.data) : res.data
          if (data && data.path) {
            savedPaths.push(data.path)
          }
        } catch (error) {
          console.error('Failed to upload file:', error)
        }
      }
      if (savedPaths.length > 0) {
        const now = new Date()
        const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
        if (!chat.messages) chat.messages = []
        chat.messages.push({
          text: `文件已保存，保存在：\n${savedPaths.join('\n')}`,
          time: time,
          sender: '系统',
          timestamp: now.getTime(),
          isSent: true,
          isSystemHint: true
        })
      }
    }
    return
  }

  if (!currentMesh.value) return

    // Use mesh filesystem (same as image upload) - save to /shared/{owner}/publish/files/{hash}
    const uploadedFiles = []
    const sessionId = chat.isGroup
      ? [currentMeshAgentUsername.value, chat.gcid].sort().join('~')
      : [currentMeshAgentUsername.value, chat.name].sort().join('~')

    for (let i = 0; i < files.length; i++) {
      const file = files[i]
      const fileName = file.name || ('file_' + Date.now() + '_' + i)
      try {
        const arrayBuffer = await file.arrayBuffer()
        const response = await chatService.uploadFile(currentMesh.value, arrayBuffer)
        const hash = typeof response.data === 'string' ? response.data : ''
        if (hash) {
          uploadedFiles.push({
            hash: hash,
            name: fileName,
            type: file.type || 'application/octet-stream',
            size: file.size || 0,
            owner: currentMeshAgentUsername.value,
            path: '/shared/' + currentMeshAgentUsername.value + '/publish/files/' + hash
          })
        }
      } catch (error) {
        console.error('Failed to upload file:', fileName, error)
      }
    }

    const text = newMessage.value.trim()
    if (uploadedFiles.length > 0) {
      if (chat.isGroup) {
        await chatService.sendGroupMessage(currentMesh.value, chat.creator, chat.groupId, text, sessionId, uploadedFiles)
      } else {
        await chatService.sendMessage(currentMesh.value, chat.name, text, sessionId, uploadedFiles)
      }
      newMessage.value = ''
    }

    // Show feedback
    const now = new Date()
    const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
    if (!chat.messages) chat.messages = []
    if (uploadedFiles.length > 0) {
      chat.messages.push({
        text: '[文件已发送] ' + uploadedFiles.map(f => f.name).join('，'),
        time: time,
        sender: '系统',
        timestamp: now.getTime(),
        isSent: true,
        isSystemHint: true
      })
    } else {
      chat.messages.push({
        text: '[文件上传失败] 请重试',
        time: time,
        sender: '系统',
        timestamp: now.getTime(),
        isSent: true,
        isSystemHint: true
      })
    }
}

const switchMesh = async (meshName) => {
  if (switchingTo.value === meshName) return
  switchingTo.value = meshName
  currentMesh.value = meshName
  const mesh = meshes.value.find(m => m.name === meshName)
  currentMeshAgentUsername.value = mesh?.agent?.username || ''
  chats.value = chats.value.filter(c => !c.isTemp)
  await fetchChats()
  await fetchUsers()
  switchingTo.value = null
}

const fetchUsers = async () => {
  if (!currentMesh.value) return
  try {
    const response = await chatService.getUsers(currentMesh.value)
    // Response is now a list of EP objects: { id, name, username, online, ... }
    // Exclude own endpoint(s)
    users.value = (response.data || []).filter(ep => ep.username !== currentMeshAgentUsername.value)
  } catch (error) {
    console.error('[fetchUsers] error:', error)
  }
}

const selectUser = async (user) => {
  // Clear other active states
  activeOpenclawAgent.value = null
  activeZeroClawSession.value = null
  activeZAgent.value = null
  if (zeroclawWS) {
    zeroclawWS.close()
    zeroclawWS = null
  }

  // user is an EP object; peer identity for chat is ep.username
  const peerName = user.username || user.name
  const existingChat = chats.value.find(c => c.name === peerName)
  if (existingChat) {
    currentActiveChatId.value = existingChat.id
    existingChat.updated = 0
  } else {
    const newChat = {
      id: 'dm-' + Date.now(),
      name: peerName,
      displayName: user.name !== peerName ? user.name + ' (' + peerName + ')' : peerName,
      time: '',
      lastMessage: '',
      updated: 0,
      messages: [],
      isTemp: true
    }
    chats.value.unshift(newChat)
    currentActiveChatId.value = newChat.id
  }
}

const selectOpenclawAgent = async (agent) => {
  activeChat.value = null
  // 直接设置活动 openclaw agent，不添加到 chats 列表
  activeOpenclawAgent.value = {
    agentId: agent.id,
    name: agent.name,
    emoji: agent.emoji || '🤖',
    isOpenclaw: true,
    messages: [],
    sessions: [],
    isTemp: true
  }
  
  const chat = activeOpenclawAgent.value
  
  // 加载会话历史（仅当还没有会话时）
  if (!chat.sessions || chat.sessions.length === 0) {
    try {
      const response = await openclawService.getSessions(agent.id)
      const rawData = response.data
      let sessions = []
      try {
        const parsed = typeof rawData === 'string' ? JSON.parse(rawData) : rawData
        sessions = parsed?.sessions || []
      } catch (e) {
        console.error('Failed to parse sessions:', e)
      }
      openclawSessions.value = sessions
      chat.sessions = sessions
      const defaultSessionId = sessions.length > 0 ? String(sessions[0].sessionId) : null
      if (defaultSessionId) {
        await loadSessionHistory(chat, agent.id, defaultSessionId)
      }
      chat.isTemp = false
    } catch (error) {
      console.error('Failed to fetch sessions:', error)
    }
  } else if (!chat.messages || chat.messages.length === 0) {
    const defaultSessionId = chat.sessions.length > 0 ? String(chat.sessions[0].sessionId) : null
    if (defaultSessionId) {
      await loadSessionHistory(chat, agent.id, defaultSessionId)
    }
  }
}

const loadSessionHistory = async (chat, agentId, sessionId) => {
  try {
    // Load chat history from chat_log database instead of session JSONL files
    const response = await openclawService.getChatLog(agentId)
    const messages = Array.isArray(response.data) ? response.data : []
    
    // Fallback to session file if chat_log returns less than 10 messages
    if (messages.length < 10) {
      console.log('[loadSessionHistory] chat_log has only', messages.length, 'messages, falling back to session file')
      await loadFromSessionFile(chat, agentId, sessionId)
      return
    }
    
    chat.messages = messages.map(msg => ({
      text: msg.text || '',
      time: msg.time || '',
      sender: msg.sender || '',
      isSent: msg.isSent || false,
      timestamp: msg.timestamp || 0
    }))
    chat.sessionId = sessionId
  } catch (error) {
    console.error('Failed to load chat log:', error)
    // Fallback: try session history if chat-log fails
    await loadFromSessionFile(chat, agentId, sessionId)
  }
}

const loadFromSessionFile = async (chat, agentId, sessionId) => {
  try {
    const historyResponse = await openclawService.getSessionHistory(agentId, sessionId)
    let historyData = null
    try {
      historyData = JSON.parse(`[${historyResponse.data.replaceAll('\n',',')}{}]`)
    } catch (e) {
      console.error('Failed to parse history:', e)
    }
    chat.messages = [];
    if (historyData) {
      // Filter messages and take only latest 10
      const allMessages = []
      historyData.filter((n)=>n.type=='message').forEach((n,i)=>{
        const text = n.message.content.filter((n)=>n.type=='text')[0]?.text;
        if(!!text){
          allMessages.push({
            "text": text,
            "time": new Date(n.message.timestamp).toLocaleTimeString(),
            "sender": n.message.role,
            "isSent": n.message.role=='user',
            "timestamp": n.message.timestamp
          })
        }
      })
      // Reverse to get newest first, take 10, then reverse back to oldest first
      chat.messages = allMessages.reverse().slice(0, 10).reverse()
    }
    chat.sessionId = sessionId
  } catch (fallbackError) {
    console.error('Failed to load session history fallback:', fallbackError)
  }
}

const switchOpenclawSession = async (chat, sessionId) => {
  try {
    // Load chat history from chat_log database
    const response = await openclawService.getChatLog(chat.agentId)
    const messages = Array.isArray(response.data) ? response.data : []
    
    if (messages.length < 10) {
      console.log('[switchOpenclawSession] chat_log has only', messages.length, 'messages, falling back to session file')
      await loadFromSessionFile(chat, chat.agentId, sessionId)
      return
    }
    
    chat.messages = messages.map(msg => ({
      text: msg.text || '',
      time: msg.time || '',
      sender: msg.sender || '',
      isSent: msg.isSent || false,
      timestamp: msg.timestamp || 0
    }))
    chat.sessionId = sessionId
  } catch (error) {
    console.error('Failed to fetch chat log:', error)
    await loadFromSessionFile(chat, chat.agentId, sessionId)
  }
}

const createGroupChat = async (selectedAgentNames, groupName) => {
  if (!groupName.trim() || selectedAgentNames.length < 1) return

  try {
    const res = await groupChatService.createGroupChat(groupName, selectedAgentNames)
    const result = res.data
    console.log('[GroupChat] Created:', result)

    // Add to local state
    localGroupChats.value.push({
      groupId: result.group_id,
      groupName: groupName,
      ownerAgent: result.agent_name,
      members: selectedAgentNames,
      sessionId: result.group_id,
      messages: []
    })

    // Refresh zAgents to include the newly created owner agent
    await fetchZAgents()

    // Switch to this new group
    await enterGroupChat(result.group_id)
  } catch (error) {
    console.error('[GroupChat] Failed to create group:', error)
    const msg = error?.response?.data?.error || error?.message || 'Failed to create group chat'
    alert(msg)
  }
}

const handleDeleteGroup = async (chat) => {
  if (!confirm(`Delete group "${chat.name}"? This cannot be undone.`)) return
  try {
    await chatService.deleteGroup(currentMesh.value, chat.creator, chat.groupId)
  } catch (error) {
    console.error('Failed to delete group:', error)
  }
  // Remove from local list immediately regardless of API result
  const idx = chats.value.indexOf(chat)
  if (idx >= 0) chats.value.splice(idx, 1)
  activeChat.value = null
  await fetchChats()
}

const handleLeaveGroup = async (chat) => {
  const confirmed = window.confirm ? window.confirm(`Leave group "${chat.name}"?`) : true
  if (!confirmed) return
  
  console.log('[handleLeaveGroup] Leaving group:', { creator: chat.creator, groupId: chat.groupId, name: chat.name })
  
  try {
    const result = await chatService.leaveGroup(currentMesh.value, chat.creator, chat.groupId)
    console.log('[handleLeaveGroup] API result:', result)
  } catch (error) {
    console.error('[handleLeaveGroup] API error:', error)
    return
  }
  
  // Remove from local list immediately regardless of API result
  const idx = chats.value.findIndex(c => c.id === chat.id)
  console.log('[handleLeaveGroup] Removing from local list, idx:', idx)
  if (idx >= 0) chats.value.splice(idx, 1)
  if (activeChat.value === idx) {
    activeChat.value = null
  } else if (activeChat.value !== null && activeChat.value > idx) {
    activeChat.value--
  }
  await fetchChats()
}

const joinParty = async (regUrl, userName, inviteCode) => {
  await meshService.joinParty(regUrl, userName, inviteCode)
  await fetchMeshes()
}

const leaveMesh = async (meshName) => {
  await meshService.leaveMesh(meshName)
  await fetchMeshes()
}

const deleteAgent = async (agentId) => {
  await openclawService.deleteAgent(agentId)
  await fetchAgents()
}

const renameGroupChat = async (chat, newName) => {
  if (!currentMesh.value || !newName.trim()) return
  
  if (chat.creator !== currentMeshAgentUsername.value) {
    console.error('Failed to rename group: Only the group creator can rename the group')
    return
  }
  
  try {
    await chatService.createGroup(currentMesh.value, chat.creator, chat.groupId, {
      name: newName.trim(),
      members: chat.members || []
    })
    await fetchChats()
  } catch (error) {
    console.error('Failed to rename group:', error)
  }
}

const updateGroupMembers = async (chat, members) => {
  if (!currentMesh.value) return
  
  if (chat.creator !== currentMeshAgentUsername.value) {
    console.error('Failed to update group members: Only the group creator can update members')
    return
  }
  
  try {
    await chatService.updateGroupMembers(
      currentMesh.value, chat.creator, chat.groupId,
      chat.name, members
    )
    await fetchChats()
  } catch (error) {
    console.error('Failed to update group members:', error)
  }
}

const openLocalTemplates = () => {
  showLocalTemplates.value = true
  showSharedTemplates.value = false
}

const openSharedTemplates = () => {
  showSharedTemplates.value = true
  showLocalTemplates.value = false
}

const openMainChatForInstall = () => {
  const mainAgent = openclawAgents.value.find(a => a.id === 'main')
  if (!mainAgent) {
    console.error('[openMainChatForInstall] main agent not found')
    return
  }
  
  activeChat.value = null
  activeOpenclawAgent.value = {
    agentId: mainAgent.id,
    name: mainAgent.name,
    emoji: mainAgent.emoji || '🤖',
    isOpenclaw: true,
    messages: [],
    sessions: [],
    isTemp: true
  }
}

const handleSendMessages = async (messages) => {
  const mainAgent = openclawAgents.value.find(a => a.id === 'main')
  if (!mainAgent) {
    console.error('[handleSendMessages] main agent not found')
    return
  }
  
  // Set main agent as active
  activeChat.value = null
  activeOpenclawAgent.value = {
    agentId: mainAgent.id,
    name: mainAgent.name,
    emoji: mainAgent.emoji || '🤖',
    isOpenclaw: true,
    messages: [],
    sessions: [],
    isTemp: true
  }
  
  // Send each message with a delay between them
  for (let i = 0; i < messages.length; i++) {
    const message = messages[i]
    
    // Add user message to chat immediately
    const now = new Date()
    const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
    
    if (!activeOpenclawAgent.value.messages) {
      activeOpenclawAgent.value.messages = []
    }
    
    activeOpenclawAgent.value.messages.push({
      text: message,
      time: time,
      sender: currentMeshAgentUsername.value,
      timestamp: now.getTime(),
      isTemp: true,
      isSent: true
    })
    
    activeOpenclawAgent.value.lastMessage = message
    activeOpenclawAgent.value.time = time
    
    // Send API call
    try {
      const response = await openclawService.sendMessage('main', message)
      const payloads = response.data?.payloads || response.data?.result?.payloads || []
      const replyText = payloads.map(p => p?.text).filter(Boolean).join('\n\n')
      
      if (replyText) {
        const replyTime = new Date().getHours().toString().padStart(2, '0') + ':' + new Date().getMinutes().toString().padStart(2, '0')
        activeOpenclawAgent.value.messages.push({
          text: replyText,
          time: replyTime,
          sender: mainAgent.name,
          timestamp: new Date().getTime(),
          isTemp: false
        })
      }
    } catch (e) {
      console.error(`[handleSendMessages] Error sending message ${i + 1}:`, e)
    }
    
    // Delay between messages
    if (i < messages.length - 1) {
      await new Promise(resolve => setTimeout(resolve, 500))
    }
  }
}

const handleTemplateInstalled = async (data) => {
  await fetchOpenclawAgents()
  
  showLocalTemplates.value = false
  showSharedTemplates.value = false
  
  if (!data || !data.agentName || !data.soulContent) {
    console.warn('[handleTemplateInstalled] missing data, skipping auto-send')
    return
  }
  
  const mainAgent = openclawAgents.value.find(a => a.id === 'main')
  if (!mainAgent) {
    console.error('[handleTemplateInstalled] main agent not found')
    return
  }
  
  activeChat.value = null
  activeOpenclawAgent.value = {
    agentId: mainAgent.id,
    name: mainAgent.name,
    emoji: mainAgent.emoji || '🤖',
    isOpenclaw: true,
    messages: [],
    sessions: [],
    isTemp: true
  }
  
  const message = `帮我创建一个名字是'${data.agentName}'的agent，如果这个名字是中文的需要用名字对应的拼音作为 agent-id，他的soul.md是：\n\n${data.soulContent}`
  newMessage.value = message
  
  await new Promise(resolve => setTimeout(resolve, 100))
  sendMessage()
}

const installedAgentIds = computed(() => {
  // Include both id and name for matching
  const ids = openclawAgents.value.map(a => a.id)
  const names = openclawAgents.value.map(a => a.name)
  return [...ids, ...names]
})

provide('switchMesh', switchMesh)
provide('meshes', meshes)
provide('openclawAgents', openclawAgents)
provide('zeroclawSessions', zeroclawSessions)
provide('activeZeroClawSession', activeZeroClawSession)
provide('selectZeroClawSession', selectZeroClawSession)
provide('zeroclawSessions', zeroclawSessions)
provide('zAgents', zAgents)
provide('activeZAgent', activeZAgent)
provide('voiceCallStore', voiceCallStore)
provide('selectZAgent', selectZAgent)
provide('fetchZAgents', fetchZAgents)
provide('createZAgent', createZAgent)
provide('deleteZAgent', deleteZAgent)
provide('fetchUsers', fetchUsers)
provide('users', users)
provide('selectUser', selectUser)
provide('createGroupChat', createGroupChat)
provide('renameGroupChat', renameGroupChat)
provide('updateGroupMembers', updateGroupMembers)
provide('groupChats', groupChats)
provide('localGroupChats', localGroupChats)
provide('activeGroupId', activeGroupId)
provide('enterGroupChat', enterGroupChat)
provide('leaveGroupChat', leaveGroupChat)
provide('handleDeleteLocalGroup', handleDeleteLocalGroup)
provide('currentMeshAgentUsername', currentMeshAgentUsername)
provide('joinParty', joinParty)
provide('leaveMesh', leaveMesh)
provide('deleteAgent', deleteAgent)
provide('createSession', createSession)
provide('fetchZeroClawSessions', fetchZeroClawSessions)
provide('localOpenclawAvailable', localOpenclawAvailable)

// --- beforeunload: cleanly close WebSocket connections to prevent zeroclaw hang on refresh ---
function handleBeforeUnload() {
  if (zeroclawWS) {
    try { zeroclawWS.close() } catch (e) {}
    zeroclawWS = null
  }
  // Close all cached zAgent connections
  var agentNames = Object.keys(wsConnections)
  for (var k = 0; k < agentNames.length; k++) {
    var conn = wsConnections[agentNames[k]]
    if (conn && conn.zeroclawWS) {
      try { conn.zeroclawWS.close() } catch (e) {}
    }
  }
  // Close all group connections
  activeGroupWsMap.forEach(function(connections) {
    if (connections) {
      for (var k = 0; k < connections.length; k++) {
        var c = connections[k]
        if (c && c.ws) {
          try { c.ws.close() } catch (e) {}
        }
      }
    }
  })
}
window.addEventListener('beforeunload', handleBeforeUnload)

const resolveEpDisplayName = (username) => {
  if (!username) return username
  // 优先从 openclawAgents 获取 identityName（人可读的名字）
  const agent = openclawAgents.value.find(a => a.id === username)
  if (agent) return username + "/" + (agent.identityName || agent.name)
  // 如果不是本地 agent，使用 mesh 用户的 name
  const ep = users.value.find(u => u.username === username)
  if (ep) return ep.username + "/" + ep.name
  return username
}
provide('activeOpenclawAgent', activeOpenclawAgent)
provide('resolveEpDisplayName', resolveEpDisplayName)

const startChatsPolling = () => {
  stopChatsPolling()
  chatsPollTimer = setInterval(fetchChats, 3000)
  usersPollTimer = setInterval(fetchUsers, 5000)
}

const stopChatsPolling = () => {
  if (chatsPollTimer) {
    clearInterval(chatsPollTimer)
    chatsPollTimer = null
  }
  if (usersPollTimer) {
    clearInterval(usersPollTimer)
    usersPollTimer = null
  }
  if (zeroclawSessionsPollTimer) {
    clearInterval(zeroclawSessionsPollTimer)
    zeroclawSessionsPollTimer = null
  }
}

const startZeroClawSessionsPolling = () => {
  fetchZeroClawSessions()
  zeroclawSessionsPollTimer = setInterval(fetchZeroClawSessions, 10000)
}

onMounted(async () => {
	if(window.__TAURI_OS_PLUGIN_INTERNALS__ && !!platform()){
		const saved = getApiToken()
		if (!saved) {
			showTokenDialog.value = true
		}
		setTimeout(()=>{
			initAuth()
		},3000)
		await shellService.startPipy(()=>{});
	} else {
		initAuth()
	}
})

onUnmounted(() => {
  stopChatsPolling()
  stopZeroClawSessionsPolling()
})

const startApp = () => {
  if (appStarted) return
  appStarted = true

  // 清理可能存在的旧 openclaw chats
  chats.value = chats.value.filter(c => !c.isOpenclaw)

  fetchMeshes().then(() => {
    startChatsPolling()
  })
  // fetchOpenclawAgents() // Disabled - openclaw agents not used
  // startZeroClawSessionsPolling() // Disabled - zeroclaw sessions hidden
  fetchZAgents()
  fetchLocalGroupChats()
}

const verifyToken = async (token) => {
  setApiToken(token)
  try {
    await meshService.getMeshes()
    return true
  } catch (error) {
    if (error?.response?.status === 401) return false
    throw error
  }
}

const initAuth = async () => {
  const saved = getApiToken()
  if (!saved) {
    showTokenDialog.value = true
    return
  }

  try {
    const ok = await verifyToken(saved)
		
    if (ok) {
      showTokenDialog.value = false
      startApp()
      return
    }
  } catch (error) {
    console.error('验证 token 失败:', error)
  }

  setApiToken('')
  tokenInput.value = ''
    tokenError.value = 'Invalid token, please try again'
  showTokenDialog.value = true
}

const submitToken = async () => {
  const token = tokenInput.value.trim()
  if (!token || tokenChecking.value) return

  tokenChecking.value = true
  tokenError.value = ''

  try {
    const ok = await verifyToken(token)
    if (!ok) {
      tokenError.value = 'Invalid token'
      return
    }
    showTokenDialog.value = false
    startApp()
  } catch (error) {
    console.error('验证 token 失败:', error)
    tokenError.value = '无法连接 agent，请检查服务状态'
  } finally {
    tokenChecking.value = false
  }
}
</script>

<style scoped>
.chat-container {
  width: 100%;
  height: 100%;
  display: flex;
  overflow: hidden;
  background: var(--bg-primary);
}

.token-dialog-wrap {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(140deg, #f4f7fb 0%, #e8eef8 100%);
}

.token-dialog {
  width: min(420px, calc(100% - 32px));
  border-radius: 14px;
  background: #ffffff;
  padding: 24px;
  box-shadow: 0 10px 30px rgba(26, 43, 71, 0.16);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.token-dialog h2 {
  margin: 0;
  font-size: 20px;
  color: #1f2937;
}

.token-dialog p {
  margin: 0;
  color: #6b7280;
  font-size: 13px;
}

.token-dialog input {
  border: 1px solid #d1d5db;
  border-radius: 10px;
  padding: 10px 12px;
  font-size: 14px;
  outline: none;
}

.token-dialog input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15);
}

.token-dialog button {
  border: 0;
  border-radius: 10px;
  padding: 10px 12px;
  font-size: 14px;
  color: #ffffff;
  background: #2563eb;
  cursor: pointer;
}

.token-dialog button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.token-error {
  color: #dc2626;
  font-size: 13px;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: var(--bg-chat);
  color: var(--text-primary);
}

.empty-icon {
  margin-bottom: 16px;
}

.empty-state h2 {
  font-size: 24px;
  font-weight: 700;
  margin-bottom: 8px;
}

.empty-state p {
  color: var(--text-secondary);
  font-size: 14px;
}

@media (max-width: 768px) {
  .chat-container {
    position: relative;
    width: 100%;
    height: 100vh;
    padding-top: calc(48px + env(safe-area-inset-top, 0));
    box-sizing: border-box;
  }
  
  .empty-state {
    height: calc(100vh - 48px - env(safe-area-inset-top, 0));
  }
  
  .mobile-agents-view {
    position: absolute;
    top: calc(48px + env(safe-area-inset-top, 0));
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--slack-purple);
    overflow-y: auto;
  }
  
  .mobile-agents-header {
    padding: 12px 16px;
    color: #fff;
    font-size: 15px;
    font-weight: 700;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }
  
  .mobile-agents-list {
    padding: 8px 0;
  }
  
  .mobile-agent-item {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    cursor: pointer;
    transition: background 0.1s;
  }
  
  .mobile-agent-item:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  
  .mobile-agent-item .item-avatar {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-right: 12px;
    flex-shrink: 0;
		color: #fff;
  }
  
  .mobile-agent-item .openclaw-avatar {
    background: linear-gradient(135deg, #cecece, #cecece);
    font-size: 18px;
  }
  
  .mobile-agent-item .item-name {
    color: #fff;
    font-size: 15px;
    font-weight: 500;
  }
  
  .mobile-empty {
    padding: 32px 16px;
    text-align: center;
    color: rgba(255, 255, 255, 0.5);
    font-size: 14px;
  }
  
  .mobile-empty-hint {
    margin-top: 8px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.35);
  }
}
</style>
