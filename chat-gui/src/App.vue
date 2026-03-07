<template>
  <div v-if="showTokenDialog" class="token-dialog-wrap">
    <div class="token-dialog">
      <h2>Enter Access Token</h2>
      <p>请输入 ztm agent 的 API token</p>
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
    />
    <ChatMain
      v-if="activeChat !== null && activeChat < chats.length"
      :chat="chats[activeChat]"
      :meshName="currentMesh"
      :currentUserName="currentMeshAgentUsername"
      :sending="sending"
      :openclawSessions="openclawSessions"
      v-model="newMessage"
      v-model:selectedAgent="selectedAgent"
      @send="sendMessage"
      @switchSession="(sessionId) => switchOpenclawSession(chats[activeChat], sessionId)"
    />
    <div v-else class="empty-state">
      <div class="empty-icon">
        <svg width="80" height="80" viewBox="0 0 80 80" fill="none">
          <circle cx="40" cy="40" r="40" fill="#E8E8E8"/>
          <path d="M40 20C29.5 20 21 28.5 21 39c0 7.3 4.2 13.7 10.5 17.5v5.5c0 2.2 1.8 4 4 4h9c2.2 0 4-1.8 4-4v-5.5c6.3-3.8 10.5-10.2 10.5-17.5C59 28.5 50.5 20 40 20z" fill="#4A154B"/>
        </svg>
      </div>
      <h2>Welcome to ClawParty!</h2>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, provide } from 'vue'
import ChatSidebar from './components/ChatSidebar.vue'
import ChatMain from './components/ChatMain.vue'
import { meshService, chatService, openclawService, setApiToken, getApiToken } from './services/chatService'

const meshes = ref([])
const openclawAgents = ref([])
const openclawSessions = ref([])
const currentMesh = ref('')
const currentMeshAgentUsername = ref('')
const chats = ref([])
const activeChat = ref(null)
const newMessage = ref('')
const selectedAgent = ref('')
const sending = ref(false)
const showTokenDialog = ref(false)
const tokenInput = ref('')
const tokenChecking = ref(false)
const tokenError = ref('')
let appStarted = false
let chatsPollTimer = null

provide('currentMesh', currentMesh)

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

const parseChatData = (data) => {
  return data.map(item => {
    const name = item.peer || item.name || '未知'
    const latestMsg = item.latest?.message?.text || ''
    const firstLine = latestMsg.split('\n')[0].substring(0, 30)
    const isGroup = !!item.group
    
    return {
      id: item.group || item.peer || Math.random().toString(),
      name: name,
      time: formatTime(item.time),
      lastMessage: firstLine,
      updated: item.updated || 0,
      isGroup: isGroup,
      creator: item.creator || '',
      groupId: item.group || '',
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
    console.error('获取 meshes 失败:', error)
  }
}

const fetchOpenclawAgents = async () => {
  try {
    const response = await openclawService.getAgents()
    const agentsData = Array.isArray(response.data) ? response.data : []
    openclawAgents.value = agentsData.map(agent => ({
      id: agent.id,
      name: agent.identityName || agent.id,
      emoji: agent.identityEmoji || '🤖',
      model: agent.model,
      isOpenclaw: true
    }))
    
    agentsData.forEach(agent => {
      const existingChat = chats.value.find(c => c.isOpenclaw && c.agentId === agent.id)
      if (!existingChat) {
        chats.value.push({
          id: agent.id,
          agentId: agent.id,
          name: agent.identityName || agent.id,
          emoji: agent.identityEmoji || '🤖',
          time: '',
          lastMessage: '',
          updated: 0,
          messages: [],
          sessions: [],
          sessionId: null,
          isOpenclaw: true,
          isTemp: true
        })
      }
    })
  } catch (error) {
    console.error('获取 OpenClaw agents 失败:', error)
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
      const existingChatNames = new Set(chats.value.map(c => c.name))
      const newChatNames = new Set(newChats.map(c => c.name))
      
      newChats.forEach(newChat => {
        const existingIndex = chats.value.findIndex(c => c.name === newChat.name && !c.isOpenclaw)
        if (existingIndex !== -1) {
          chats.value[existingIndex].time = newChat.time
          chats.value[existingIndex].lastMessage = newChat.lastMessage
          chats.value[existingIndex].updated = newChat.updated
          chats.value[existingIndex].isTemp = false
        } else {
          chats.value.push(newChat)
        }
      })
      
      for (let i = chats.value.length - 1; i >= 0; i--) {
        if (!chats.value[i].isOpenclaw && !newChatNames.has(chats.value[i].name) && !chats.value[i].isTemp) {
          chats.value.splice(i, 1)
        }
      }
      
      if (savedChatId !== null) {
        const newIndex = chats.value.findIndex(c => c.id === savedChatId && c.isOpenclaw === savedIsOpenclaw)
        if (newIndex !== -1) {
          activeChat.value = newIndex
        }
      } else if (activeChat.value === null && chats.value.length > 0) {
        activeChat.value = 0
      }
    }
  } catch (error) {
    console.error('获取聊天列表失败:', error)
  }
}

const selectChat = (index) => {
  activeChat.value = index
  if (chats.value[index]) {
    chats.value[index].updated = 0
  }
}

const sendMessage = async () => {
  if (!newMessage.value.trim() || activeChat.value === null || sending.value) return
  
  const chat = chats.value[activeChat.value]
  const text = newMessage.value
  sending.value = true
  
  try {
    if (chat.isOpenclaw) {
      if (!chat.messages) chat.messages = []
      sending.value = false
      const now = new Date()
      const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
      
      const isBotChat = !!selectedAgent.value
      const typingSender = isBotChat ? 'A to ' + selectedAgent.value : chat.name
      const responseSender = isBotChat ? selectedAgent.value : chat.name
      
      setTimeout(()=>{
        chat.messages.push({
          text: '',
          time: time,
          sender: typingSender,
          timestamp: now.getTime(),
          isTyping: true
        })
      },300)
      
      const sendPromise = isBotChat 
        ? openclawService.botChat(chat.agentId, selectedAgent.value, text)
        : openclawService.sendMessage(chat.agentId, text)
      
      sendPromise.then((response)=>{
        let replyText = response.data?.payloads?.[0]?.text || response.data?.result?.payloads?.[0]?.text;
        
        const typingIndex = chat.messages.findIndex(m => m.isTyping)
        if (typingIndex !== -1) {
          chat.messages.splice(typingIndex, 1)
        }
        if (replyText) {
          const replyTime = new Date().getHours().toString().padStart(2, '0') + ':' + new Date().getMinutes().toString().padStart(2, '0')
          chat.messages.push({
            text: replyText,
            time: replyTime,
            sender: responseSender,
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
        let replyText = '响应超时，请稍后刷新。'
        if (replyText) {
          const replyTime = new Date().getHours().toString().padStart(2, '0') + ':' + new Date().getMinutes().toString().padStart(2, '0')
          chat.messages.push({
            text: replyText,
            time: replyTime,
            sender: responseSender,
            timestamp: new Date().getTime(),
            isTemp: false
          })
          chat.lastMessage = replyText
          chat.time = replyTime
        }
      })
    } else if (chat.isGroup) {
      await chatService.sendGroupMessage(currentMesh.value, chat.creator, chat.groupId, text)
    } else {
      await chatService.sendMessage(currentMesh.value, chat.name, text)
    }
  } catch (error) {
    console.error('发送消息失败:', error)
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
    isTemp: true
  })
  
  chat.lastMessage = text
  chat.time = time
  
  newMessage.value = ''
}

const switchMesh = async (meshName) => {
  currentMesh.value = meshName
  const mesh = meshes.value.find(m => m.name === meshName)
  currentMeshAgentUsername.value = mesh?.agent?.username || ''
  chats.value = chats.value.filter(c => !c.isTemp)
  await fetchChats()
  await fetchUsers()
}

const users = ref([])

const fetchUsers = async () => {
  console.log('[fetchUsers] currentMesh:', currentMesh.value)
  if (!currentMesh.value) return
  try {
    console.log('[fetchUsers] calling getUsers for mesh:', currentMesh.value)
    const response = await chatService.getUsers(currentMesh.value)
    console.log('[fetchUsers] response:', response.data)
    users.value = response.data
  } catch (error) {
    console.error('[fetchUsers] error:', error)
  }
}

const selectUser = async (user) => {
  const existingChat = chats.value.find(c => c.name === user.name)
  if (existingChat) {
    const index = chats.value.indexOf(existingChat)
    activeChat.value = index
    if (chats.value[index]) {
      chats.value[index].updated = 0
    }
  } else {
    const newChat = {
      id: 'dm-' + Date.now(),
      name: user.name,
      time: '',
      lastMessage: '',
      updated: 0,
      messages: [],
      isTemp: true
    }
    chats.value.unshift(newChat)
    activeChat.value = 0
  }
}

const selectOpenclawAgent = async (agent) => {
  const chat = chats.value.find(c => c.isOpenclaw && c.agentId === agent.id)
  if (chat) {
    activeChat.value = chats.value.indexOf(chat)
    if (!chat.sessions || chat.sessions.length === 0) {
      try {
        const response = await openclawService.getSessions(agent.id)
        const rawData = response.data
        let sessions = []
        try {
          const parsed = typeof rawData === 'string' ? JSON.parse(rawData) : rawData
          sessions = parsed?.sessions || []
        } catch (e) {
          console.error('解析 sessions 失败:', e)
        }
        openclawSessions.value = sessions
        chat.sessions = sessions
        const defaultSessionId = sessions.length > 0 ? String(sessions[0].sessionId) : null
        if (defaultSessionId) {
          const historyResponse = await openclawService.getSessionHistory(agent.id, defaultSessionId)
          let historyData = null
          try {
            historyData = JSON.parse(`[${historyResponse.data.replaceAll('\n',',')}{}]`)
          } catch (e) {
            console.error('解析 history 失败:', e)
          }
					chat.messages = [];
          historyData.filter((n)=>n.type=='message').forEach((n,i)=>{
						console.log(n.message.content)
						const text = n.message.content.filter((n)=>n.type=='text')[0]?.text;
						if(!!text){
							chat.messages.push(
							{ 
								"text": n.message.content.filter((n)=>n.type=='text')[0]?.text, 
								"time": new Date(n.message.timestamp).toLocaleTimeString(), 
								"sender": n.message.role, "isSent": n.message.role=='user', "timestamp": n.message.timestamp }
							)
						}
					})
          chat.sessionId = defaultSessionId
        }
        chat.isTemp = false
      } catch (error) {
        console.error('获取 sessions 失败:', error)
      }
    }
  }
}

const switchOpenclawSession = async (chat, sessionId) => {
  try {
    const response = await openclawService.getSessionHistory(chat.agentId, sessionId)
    let data = null
    try {
      data = typeof response.data === 'string' ? JSON.parse(response.data) : response.data
    } catch (e) {
      console.error('解析 history 失败:', e)
    }
    chat.messages = data?.messages || []
    chat.sessionId = sessionId
  } catch (error) {
    console.error('获取历史消息失败:', error)
  }
}

const createGroupChat = async (selectedUsers, groupName) => {
  if (!currentMesh.value || !currentMeshAgentUsername.value || selectedUsers.length < 2) return
  
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
    
    await chatService.sendGroupMessage(currentMesh.value, creator, groupId, `${groupName} 已创建`)
    
    await fetchChats()
    
    const newChat = chats.value.find(c => c.groupId === groupId)
    if (newChat) {
      activeChat.value = chats.value.indexOf(newChat)
    }
  } catch (error) {
    console.error('创建群组失败:', error)
  }
}

provide('switchMesh', switchMesh)
provide('meshes', meshes)
provide('openclawAgents', openclawAgents)
provide('fetchUsers', fetchUsers)
provide('users', users)
provide('selectUser', selectUser)
provide('createGroupChat', createGroupChat)

const startChatsPolling = () => {
  stopChatsPolling()
  chatsPollTimer = setInterval(fetchChats, 3000)
}

const stopChatsPolling = () => {
  if (chatsPollTimer) {
    clearInterval(chatsPollTimer)
    chatsPollTimer = null
  }
}

onMounted(() => {
  initAuth()
})

onUnmounted(() => {
  stopChatsPolling()
})

const startApp = () => {
  if (appStarted) return
  appStarted = true
  fetchMeshes().then(() => {
    startChatsPolling()
  })
  fetchOpenclawAgents()
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
  tokenError.value = 'token 无效，请重新输入'
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
      tokenError.value = 'token 无效'
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
    flex-direction: column;
  }
}
</style>
