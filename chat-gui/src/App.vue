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
          <div class="mobile-empty-hint">openclaw is not installed locally. You can still interact with remote openclaw agents via group chat.</div>
        </div>
      </div>
    </div>
    <!-- Mobile groups list view -->
    <div v-if="isMobile && activeChat === null && mobileActiveOrg === 'groups'" class="mobile-agents-view">
      <div class="mobile-agents-header">Group Chats</div>
      <div class="mobile-agents-list">
        <div
          v-for="chat in groupChats"
          :key="chat.id"
          class="mobile-agent-item"
          @click="selectChat(getChatIndex(chat.id))"
        >
          <div class="item-avatar" >#</div>
          <span class="item-name">{{ chat.name }}</span>
        </div>
        <div v-if="!groupChats || groupChats.length === 0" class="mobile-empty">
          No group chats yet
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
      v-model="newMessage"
      @send="(text) => handleZAgentSend(item.id, text)"
      @send-images="handleSendImages"
      @send-files="handleSendFiles"
      @switchSession="() => {}"
      @deleteGroup="handleDeleteGroup"
      @leaveGroup="handleLeaveGroup"
      @back="currentActiveChatId = null"
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
    <div v-else-if="!isMobile && allChatConnectionItems.length === 0" class="empty-state">
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
import { ref, onMounted, onUnmounted, provide, computed, watch } from 'vue'
import ChatSidebar from './components/ChatSidebar.vue'
import ChatMain from './components/ChatMain.vue'
import TemplatePicker from './components/TemplatePicker.vue'
import { meshService, chatService, openclawService, zeroclawService, zagentService, ZeroClawWS, setApiToken, getApiToken } from './services/chatService'
import ShellService from './services/ShellService'
import { platform } from '@tauri-apps/plugin-os';
import { getAvatarColor } from './utils/avatar'

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
const chats = ref([])
const activeChat = ref(null)
const activeOpenclawAgent = ref(null)  // 当前活动的 openclaw agent
const newMessage = ref('')
const sending = ref(false)
const showTokenDialog = ref(false)
const tokenInput = ref('')
const tokenChecking = ref(false)
const tokenError = ref('')
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

const activeZAgentConnectionItems = computed(() => {
  if (!zAgents.value || !wsConnections) return []
  
  const activeAgent = zAgents.value.find(a => a.agent_name === currentActiveChatId.value)
  
  return zAgents.value
    .filter(agent => {
      return wsConnections.has(agent.agent_name) || agent === activeAgent
    })
    .map(agent => {
      const cached = wsConnections.get(agent.agent_name) || {}
      return {
        type: 'zagent',
        id: agent.agent_name,
        agent: {
          ...agent,
          isZeroClaw: true,
          messages: cached.messages || []
        },
        messages: cached.messages || [],
        chat: {
          ...agent,
          isZeroClaw: true,
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

const fetchZAgents = async () => {
  try {
    const response = await zagentService.getAgents()
    zAgents.value = response.data || []
  } catch (error) {
    console.error('Failed to fetch zAgents:', error)
  }
}

const createZAgent = async (agentName) => {
  try {
    await zagentService.createAgent(agentName, agentName)
    await zagentService.startAgent(agentName)
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

const selectZAgent = async (agent) => {
  const agentName = agent.agent_name

  currentActiveChatId.value = agentName
  currentZAgentName = agentName

  // Check if we already have a cached connection to this agent
  const cached = wsConnections.get(agentName)
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

  const doConnect = () => {
    if (currentZAgentName !== agentName) return
    if (currentActiveChatId.value !== agentName) return
    
    zeroclawWS = new ZeroClawWS(
      agentName,
      'me',
      handleZeroClawMessage,
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
    handleZeroClawMessage,
    handleZeroClawOpen,
    handleZeroClawClose,
    handleZeroClawError,
    wsPort
  )
  zeroclawWS.reconnectAttempts = 0
  zeroclawWS.onError = handleConnectError
  zeroclawWS.connect()

  // Cache the connection with reference to messages array
  wsConnections.set(agentName, {
    zeroclawWS: zeroclawWS,
    port: wsPort,
    messages: []
  })

  activeZAgent.value = {
    ...agent,
    isZeroClaw: true,
    messages: wsConnections.get(agentName).messages
  }
}

const handleZeroClawOpen = () => {
  console.log('[ZeroClaw] WebSocket connected')
  zcReconnectAttempts = 0
  currentZAgentName = null
}

let zcReconnectAttempts = 0
const maxZcReconnectAttempts = 5
let currentZAgentName = null
const wsConnections = new Map()

const handleZeroClawClose = (event) => {
  console.log('[ZeroClaw] WebSocket closed:', event.code, event.reason)
  
  const agent = activeZAgent.value
  const session = activeZeroClawSession.value
  if (!agent && !session) return
  
  if (event.code === 1000) return
  
  if (zcReconnectAttempts >= maxZcReconnectAttempts) {
    console.log('[ZeroClaw] Max reconnection attempts reached')
    zcReconnectAttempts = 0
    currentZAgentName = null
    return
  }
  
  if (agent && currentZAgentName !== agent.agent_name) {
    console.log('[ZeroClaw] Close handler ignored - agent changed')
    return
  }
  
  zcReconnectAttempts++
  const delay = 1000 * zcReconnectAttempts
  console.log('[ZeroClaw] Reconnecting... attempt ' + zcReconnectAttempts + '/' + maxZcReconnectAttempts + ' in ' + delay + 'ms')
  
  const agentNameToReconnect = agent?.agent_name
  setTimeout(() => {
    if (currentZAgentName !== agentNameToReconnect) {
      console.log('[ZeroClaw] Reconnect ignored - agent changed')
      return
    }
    if (zeroclawWS) zeroclawWS.close()
    
    if (agent && currentZAgentName === agentNameToReconnect) {
      zeroclawWS = new ZeroClawWS(
        agent.agent_name,
        'me',
        handleZeroClawMessage,
        handleZeroClawOpen,
        handleZeroClawClose,
        handleZeroClawError,
        agent.port
      )
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
  
  if (zcReconnectAttempts >= maxZcReconnectAttempts) {
    console.log('[ZeroClaw] Max reconnection attempts reached')
    zcReconnectAttempts = 0
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
  
  const agentNameToReconnect = agent?.agent_name
  setTimeout(() => {
    if (currentZAgentName !== agentNameToReconnect) {
      console.log('[ZeroClaw] Error reconnect ignored - agent changed')
      return
    }
    if (zeroclawWS) zeroclawWS.close()
    
    if (agent && currentZAgentName === agentNameToReconnect) {
      zeroclawWS = new ZeroClawWS(
        agent.agent_name,
        'me',
        handleZeroClawMessage,
        handleZeroClawOpen,
        handleZeroClawClose,
        handleZeroClawError,
        agent.port
      )
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

const handleZeroClawMessage = (data) => {
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
  }
}

const handleZAgentSend = (agentName, text) => {
  const cached = wsConnections.get(agentName)
  if (!cached) {
    console.error('[zAgent] No connection found for:', agentName)
    return
  }
  
  const zagent = zAgents.value.find(a => a.agent_name === agentName)
  const displayName = zagent?.display_name || zagent?.agent_name || 'ZeroClaw'
  if (!cached.messages) cached.messages = []
  const now = new Date()
  const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
  
  cached.messages.push({
    text: text,
    time: time,
    sender: currentMeshAgentUsername.value || 'You',
    timestamp: now.getTime(),
    isSent: true,
    isTemp: true
  })
  
  cached.messages.push({
    text: '',
    time: time,
    sender: displayName,
    timestamp: now.getTime() + 1,
    isSent: false,
    isTyping: true
  })
  
  newMessage.value = ''
  
  if (cached.zeroclawWS && cached.zeroclawWS.isConnected()) {
    cached.zeroclawWS.sendMessage(text)
  } else {
    const typingIdx = cached.messages.findIndex(m => m.isTyping)
    if (typingIdx >= 0) cached.messages.splice(typingIdx, 1)
    cached.messages.push({
      text: 'WebSocket not connected. Please try again.',
      time: time,
      sender: displayName,
      timestamp: new Date().getTime(),
      isSent: false,
      isTemp: false
    })
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

const sendMessage = async () => {
  if (!newMessage.value.trim() || (!activeOpenclawAgent.value && !activeZeroClawSession.value && !activeZAgent.value && activeChat.value === null) || sending.value) return
   
  const chat = activeOpenclawAgent.value || chats.value[activeChat.value]
  const text = newMessage.value
  sending.value = true

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
  currentMesh.value = meshName
  const mesh = meshes.value.find(m => m.name === meshName)
  currentMeshAgentUsername.value = mesh?.agent?.username || ''
  chats.value = chats.value.filter(c => !c.isTemp)
  await fetchChats()
  await fetchUsers()
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
    
    // Fallback to session file if chat_log returns less than 10 messages
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
    // Fallback to session history
    await loadFromSessionFile(chat, chat.agentId, sessionId)
  }
}

const createGroupChat = async (selectedUsers, groupName) => {
  if (!currentMesh.value || !currentMeshAgentUsername.value || selectedUsers.length < 1) return
  
  const creator = currentMeshAgentUsername.value
  const groupId = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
    const r = Math.random() * 16 | 0
    const v = c === 'x' ? r : (r & 0x3 | 0x8)
    return v.toString(16)
  })
  
  const members = [creator, ...selectedUsers.map(u => u.name)]
  
  try {
    await chatService.createGroup(currentMesh.value, creator, groupId, {
      name: groupName,
      members: members
    })
    
    await chatService.sendGroupMessage(currentMesh.value, creator, groupId, `Group "${groupName}" created`)
    
    await fetchChats()
    
    const newChat = chats.value.find(c => c.groupId === groupId)
    if (newChat) {
      activeChat.value = chats.value.indexOf(newChat)
    }
  } catch (error) {
    console.error('Failed to create group:', error)
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

const joinParty = async (regUrl, userName) => {
  await meshService.joinParty(regUrl, userName)
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
provide('currentMeshAgentUsername', currentMeshAgentUsername)
provide('joinParty', joinParty)
provide('leaveMesh', leaveMesh)
provide('deleteAgent', deleteAgent)
provide('createSession', createSession)
provide('fetchZeroClawSessions', fetchZeroClawSessions)
provide('localOpenclawAvailable', localOpenclawAvailable)

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
