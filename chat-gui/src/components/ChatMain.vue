<template>
  <main class="chat-main">
    <ChatHeader
      :chat="chat"
      :openclawSessions="openclawSessions"
      :currentUserName="currentUserName"
      :showBackButton="showBackButton"
      @search="handleSearch"
      @switchSession="$emit('switchSession', $event)"
      @deleteGroup="$emit('deleteGroup', $event)"
      @leaveGroup="$emit('leaveGroup', $event)"
      @back="$emit('back')"
      @download="handleDownload"
    />
    <div class="messages" ref="messagesContainer">
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
          <div v-if="chat.isOpenclaw && !isMessageSent(msg) && !msg.isTyping" class="avatar-emoji">
            {{ chat.emoji }}
          </div>
          <div v-else-if="msg.isTyping && chat.isOpenclaw" class="avatar-emoji">
            {{ chat.emoji }}
          </div>
          <div v-else-if="!msg.isTyping" class="avatar-placeholder" :style="{ background: getAvatarColor(isMessageSent(msg) ? (currentUserName || 'You') : (msg.sender || chat.name)) }">
            {{ (isMessageSent(msg) ? (currentUserName || 'You') : (msg.sender || chat.name))[0].toUpperCase() }}
          </div>
        </div>
        <div class="message-body">
          <div v-if="msg.isTyping" class="typing-indicator">
            <span class="typing-dot"></span>
            <span class="typing-dot"></span>
            <span class="typing-dot"></span>
          </div>
          <template v-else>
            <div class="message-header">
              <span class="message-author">{{ isMessageSent(msg) ? (currentUserName || 'You') : (msg.sender || chat.name) }}</span>
              <span class="message-time">{{ msg.time }}</span>
            </div>
            <div class="message-bubble" :class="{ 'system-hint': msg.isSystemHint }">
              <div class="message-content" v-html="renderMarkdown(msg.text)"></div>
              <div v-if="msg.isGroupRequest || msg.isPeerRequest" class="group-request-actions">
                <template v-if="(msg.isPeerRequest || msg.isGroupEpRequest) && msg.availableAgents && msg.availableAgents.length > 0">
                  <select v-model="msg.selectedAgent" class="agent-select">
                    <option v-for="a in msg.availableAgents" :key="a" :value="a">{{ a }}</option>
                  </select>
                </template>
                <button class="approve-btn" @click="approveGroupRequest(msg)">Approve</button>
              </div>
            </div>
          </template>
        </div>
      </div>
    </div>
    <MessageInput 
      :chatName="chat.name" 
      :loading="sending" 
      :modelValue="modelValue" 
      :agents="availableAgents"
      :selectedAgent="selectedAgent"
      :isOpenclaw="chat.isOpenclaw"
      :autoFocus="autoFocus"
      :members="chat.isGroup ? (chat.members || []).filter(m => m !== currentUserName) : []"
      :agentGroups="agentGroupChats"
      @update:modelValue="$emit('update:modelValue', $event)" 
      @update:selectedAgent="$emit('update:selectedAgent', $event)"
      @send="$emit('send')" 
    />
  </main>
</template>

<script setup>
import { ref, watch, nextTick, computed, onUnmounted, inject } from 'vue'
import { marked } from 'marked'
import ChatHeader from './ChatHeader.vue'
import MessageInput from './MessageInput.vue'
import { chatService } from '../services/chatService'
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
  	
  modelValue: String,
  selectedAgent: {
    type: String,
    default: ''
  },
  showBackButton: {
    type: Boolean,
    default: false
  },
  autoFocus: {
    type: Boolean,
    default: true
  }
})

defineEmits(['send', 'update:modelValue', 'update:selectedAgent', 'switchSession', 'deleteGroup', 'leaveGroup', 'back'])

const messagesContainer = ref(null)
let pollTimer = null
const searchQuery = ref('')
const openclawAgents = inject('openclawAgents', ref([]))
const allGroupChats = inject('groupChats', ref([]))
const resolveEpDisplayName = inject('resolveEpDisplayName', (u) => u)

defineExpose({})

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

const handleSearch = (query) => {
  searchQuery.value = query
}

const handleDownload = () => {
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
        ? (props.currentUserName || 'Me')
        : (msg.sender || chatName)
      const time = escapeHtml(msg.time || '')
      const isOpenclaw = props.chat.isOpenclaw && !isSent
      const emoji = props.chat.emoji || ''

      // Avatar HTML
      let avatarHtml
      if (isOpenclaw) {
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
      max-width: 600px;
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
      background: #4a154b;
      color: #ffffff;
    }
    .message.sent .message-bubble::before {
      left: auto;
      right: -6px;
      border-right: none;
      border-left: 6px solid #4a154b;
      border-right-color: transparent;
    }
    .message-bubble.system-hint {
      background: #2a2a40;
      border: 1px solid #4a4a6a;
      color: #c8c8e8;
    }
    .message-bubble.system-hint::before {
      border-right-color: #2a2a40;
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

  const blob = new Blob([html], { type: 'text/html;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `${chatName}-chat-history.html`
  a.click()
  URL.revokeObjectURL(url)
}

const filteredMessages = computed(() => {
	let msgs = [];
  if (!searchQuery.value.trim()) {
    msgs = props.chat.messages || []
  } else {
		const query = searchQuery.value.toLowerCase()
		msgs = (props.chat.messages || []).filter(msg => 
			msg.text && msg.text.toLowerCase().includes(query)
		);
	}
	msgs.forEach((m)=>{
		if(!!m.text && m.text.indexOf(' GMT')>=0){
			m.text = m.text.split(/[^[]*] /).slice(1)[0]
		}
		if(!m.text){
			m.text = '[Empty]'
		}
	})
	return msgs
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
  return marked.parse(text)
}

const isMessageSent = (msg) => {
  return msg.isSent || msg.sender === props.currentUserName
}

const parseMessages = (data) => {
  return data.map(item => {
    // sender may be "gcid/username" for group chat messages; strip the gcid prefix for display
    const rawSender = item.sender || ''
    const displaySender = rawSender.indexOf('/') !== -1 ? rawSender.split('/')[1] : rawSender
    const isSent = displaySender === props.currentUserName
    const senderDisplay = props.chat?.isGroup ? resolveEpDisplayName(displaySender) : displaySender
    return {
      text: item.message?.text || '',
      time: formatTime(item.time),
      sender: senderDisplay,
      isSent,
      timestamp: item.time,
      isSystemHint: item.isSystemHint || false,
      isGroupRequest: item.isGroupRequest || false,
      isGroupEpRequest: item.isGroupEpRequest || false,
      isPeerRequest: item.isPeerRequest || false,
      gcid: item.gcid || '',
      peer: item.peer || '',
      agentName: item.agentName || '',
      groupName: item.groupName || '',
      availableAgents: item.availableAgents || [],
      selectedAgent: item.availableAgents?.[0] || 'main',
    }
  })
}

const approveGroupRequest = async (msg) => {
  if (!props.meshName) return
  try {
    if (msg.isPeerRequest) {
      // Peer chat: enable auto-reply with selected agent
      const agentName = msg.selectedAgent || 'main'
      await chatService.approvePeerAutoReply(props.meshName, msg.peer, agentName)
      msg.isPeerRequest = false
      msg.isSystemHint = false
      msg.text = `Auto-reply enabled for "${msg.peer}" via agent "${agentName}".`
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
    console.error('Failed to approve auto-reply:', err)
  }
}

const fetchMessages = async () => {
  if (!props.meshName || !props.chat.name) return
  
  try {
    let response
    if (props.chat.isGroup) {
      response = await chatService.getGroupMessages(props.meshName, props.chat.creator, props.chat.groupId)
    } else {
      response = await chatService.getMessages(props.meshName, props.chat.name)
    }
    const messages = parseMessages(response.data || [])
    props.chat.messages = messages
    scrollToBottom()
  } catch (error) {
    if (error.response?.status === 404) {
      props.chat.messages = []
    } else {
      console.error('获取消息失败:', error)
    }
  }
}

const pollMessages = async () => {
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
        const existingIndex = props.chat.messages.findIndex(m => 
          m.sender === newMsg.sender && m.text === newMsg.text
        )
        if (existingIndex !== -1) {
          if (props.chat.messages[existingIndex].isTemp) {
            props.chat.messages[existingIndex] = newMsg
          }
        } else {
          props.chat.messages.push(newMsg)
        }
      })
      // scrollToBottom()
    }
  } catch (error) {
    if (error.response?.status !== 404) {
      console.error('轮询消息失败:', error)
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

watch(() => props.chat.name, () => {
  if (props.chat.name) {
    fetchMessages().then(() => {
      startPolling()
    })
  }
}, { immediate: true })

watch(() => props.chat.messages?.length, () => {
  scrollToBottom()
}, { immediate: true })

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
  max-width: 600px;
  width: fit-content;
}

.message-bubble.system-hint {
  background: #2a2a40;
  border: 1px solid #4a4a6a;
  color: #c8c8e8;
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

.message-content {
  color: var(--text-primary);
  font-size: 15px;
  line-height: 1.4667;
  word-wrap: break-word;
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

@media (max-width: 768px) {
  .chat-main {
    flex: 1;
    width: 100%;
    height: calc(100vh - 48px);
    min-height: 0;
    overflow-x: hidden;
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
