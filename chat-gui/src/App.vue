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
          <div class="item-avatar">#</div>
          <span class="item-name">{{ chat.name }}</span>
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
          <div class="item-avatar">{{ user.name[0].toUpperCase() }}</div>
          <span class="item-name">{{ user.name }}</span>
        </div>
        <div
          v-for="chat in meshChats"
          :key="chat.id"
          class="mobile-agent-item"
          @click="selectChat(getChatIndex(chat.id))"
        >
          <div class="item-avatar">{{ chat.name[0].toUpperCase() }}</div>
          <span class="item-name">{{ chat.name }}</span>
        </div>
      </div>
    </div>
    <ChatMain
      v-if="(activeChat !== null && activeChat < chats.length) || activeOpenclawAgent"
      :chat="activeOpenclawAgent || chats[activeChat]"
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
      @back="activeOpenclawAgent ? (activeOpenclawAgent = null) : (activeChat = null)"
    />
    <div v-else-if="!isMobile" class="empty-state">
      <div class="empty-icon">
        <svg width="80" height="80" viewBox="0 0 80 80" fill="none">
          <circle cx="40" cy="40" r="40" fill="#E8E8E8"/>
          <path d="M40 20C29.5 20 21 28.5 21 39c0 7.3 4.2 13.7 10.5 17.5v5.5c0 2.2 1.8 4 4 4h9c2.2 0 4-1.8 4-4v-5.5c6.3-3.8 10.5-10.2 10.5-17.5C59 28.5 50.5 20 40 20z" fill="#0A2E6F"/>
        </svg>
      </div>
      <h2>Welcome to ClawParty!</h2>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, provide, computed } from 'vue'
import ChatSidebar from './components/ChatSidebar.vue'
import ChatMain from './components/ChatMain.vue'
import { meshService, chatService, openclawService, setApiToken, getApiToken } from './services/chatService'
import ShellService from './services/ShellService'
import { platform } from '@tauri-apps/plugin-os';

const shellService = new ShellService();
const meshes = ref([])
const openclawAgents = ref([])
const openclawSessions = ref([])
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
let appStarted = false
let chatsPollTimer = null
let usersPollTimer = null

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
      name: agent.identityName || agent.id,
      emoji: agent.identityEmoji || '🤖',
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

const selectChat = (index) => {
  activeOpenclawAgent.value = null
  activeChat.value = index
  if (chats.value[index]) {
    chats.value[index].updated = 0
  }
}

const sendMessage = async () => {
  if (!newMessage.value.trim() || (!activeOpenclawAgent.value && activeChat.value === null) || sending.value) return
  
  const chat = activeOpenclawAgent.value || chats.value[activeChat.value]
  const text = newMessage.value
  sending.value = true
  
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
  // user is an EP object; peer identity for chat is ep.username
  const peerName = user.username || user.name
  const existingChat = chats.value.find(c => c.name === peerName)
  if (existingChat) {
    const index = chats.value.indexOf(existingChat)
    activeChat.value = index
    if (chats.value[index]) {
      chats.value[index].updated = 0
    }
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
    activeChat.value = 0
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
        historyData.filter((n)=>n.type=='message').forEach((n,i)=>{
          const text = n.message.content.filter((n)=>n.type=='text')[0]?.text;
          if(!!text){
            chat.messages.push({
              "text": text,
              "time": new Date(n.message.timestamp).toLocaleTimeString(),
              "sender": n.message.role,
              "isSent": n.message.role=='user',
              "timestamp": n.message.timestamp
            })
          }
        })
      }
      chat.sessionId = sessionId
    } catch (fallbackError) {
      console.error('Failed to load session history fallback:', fallbackError)
    }
  }
}

const switchOpenclawSession = async (chat, sessionId) => {
  try {
    // Load chat history from chat_log database
    const response = await openclawService.getChatLog(chat.agentId)
    const messages = Array.isArray(response.data) ? response.data : []
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
    try {
      const response = await openclawService.getSessionHistory(chat.agentId, sessionId)
      let data = null
      try {
        data = typeof response.data === 'string' ? JSON.parse(response.data) : response.data
      } catch (e) {
        console.error('Failed to parse history:', e)
      }
      chat.messages = data?.messages || []
      chat.sessionId = sessionId
    } catch (fallbackError) {
      console.error('Failed to fetch session history fallback:', fallbackError)
    }
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

const joinParty = async (regUrl) => {
  await meshService.joinParty(regUrl)
  await fetchMeshes()
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

provide('switchMesh', switchMesh)
provide('meshes', meshes)
provide('openclawAgents', openclawAgents)
provide('fetchUsers', fetchUsers)
provide('users', users)
provide('selectUser', selectUser)
provide('createGroupChat', createGroupChat)
provide('renameGroupChat', renameGroupChat)
provide('updateGroupMembers', updateGroupMembers)
provide('groupChats', groupChats)
provide('currentMeshAgentUsername', currentMeshAgentUsername)
provide('joinParty', joinParty)
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
}

onMounted(async () => {
	await shellService.startPipy(()=>{});
	if(window.__TAURI_OS_PLUGIN_INTERNALS__ && !!platform()){
		setTimeout(()=>{
			initAuth()
		},3000)
	} else {
		initAuth()
	}
})

onUnmounted(() => {
  stopChatsPolling()
})

const startApp = () => {
  if (appStarted) return
  appStarted = true
  
  // 清理可能存在的旧 openclaw chats
  chats.value = chats.value.filter(c => !c.isOpenclaw)
  
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
    padding-top: 48px;
    box-sizing: border-box;
  }
  
  .empty-state {
    height: calc(100vh - 48px);
  }
  
  .mobile-agents-view {
    position: absolute;
    top: 48px;
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
  }
  
  .mobile-agent-item .openclaw-avatar {
    background: linear-gradient(135deg, #ff6b6b, #ffa500);
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
