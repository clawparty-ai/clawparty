<template>
  <main class="chat-main">
    <ChatHeader
      :chat="chat"
      :openclawSessions="openclawSessions"
      :currentUserName="currentUserName"
      :showBackButton="showBackButton"
      @switchSession="$emit('switchSession', $event)"
      @deleteGroup="$emit('deleteGroup', $event)"
      @leaveGroup="$emit('leaveGroup', $event)"
      @back="$emit('back')"
      @download="handleDownload"
      @download-md="handleDownloadMd"
      @download-pdf="handleDownloadPdf"
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
              <ConfigTable v-if="msg.isConfigTable" :rows="msg.configRows" :meshName="msg.meshName || meshName" />
              <template v-else>
                <div v-if="msg.files && msg.files.length > 0" class="message-images">
                  <img v-for="file in msg.files" :key="file.hash"
                    :src="file.url"
                    :alt="file.name || 'image'"
                    class="chat-image"
                    loading="lazy"
                    @click="openImagePreview(file.url)"
                  />
                </div>
                <div v-if="msg.text" class="message-content" v-html="msg.isHtml ? msg.text : renderMarkdown(msg.text)"></div>
              </template>
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
      :isOpenclaw="chat.isOpenclaw"
      :agentId="chat.agentId"
      :autoFocus="autoFocus"
      :members="chat.isGroup ? (chat.members || []).filter(m => m !== currentUserName) : []"
      :agentGroups="agentGroupChats"
      :peerMode="peerMode"
      :showPeerMode="!chat.isOpenclaw && !!chat.name"
      @update:modelValue="$emit('update:modelValue', $event)" 
      @send="$emit('send')"
      @send-images="$emit('send-images', $event)"
      @send-files="$emit('send-files', $event)"
      @hash-command="handleHashCommand"
      @update:peerMode="handlePeerModeChange"
    />
  </main>
</template>

<script setup>
import { ref, watch, nextTick, computed, onUnmounted, inject } from 'vue'
import { marked } from 'marked'
import ChatHeader from './ChatHeader.vue'
import MessageInput from './MessageInput.vue'
import ConfigTable from './ConfigTable.vue'
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
  showBackButton: {
    type: Boolean,
    default: false
  },
  autoFocus: {
    type: Boolean,
    default: true
  }
})

defineEmits(['send', 'update:modelValue', 'switchSession', 'deleteGroup', 'leaveGroup', 'back', 'send-images', 'send-files'])

const messagesContainer = ref(null)
let pollTimer = null
const openclawAgents = inject('openclawAgents', ref([]))
const allGroupChats = inject('groupChats', ref([]))
const resolveEpDisplayName = inject('resolveEpDisplayName', (u) => u)
const peerMode = ref('')  // 'blocked' | 'muted' | 'manual' | 'auto' | ''

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
        ? (props.currentUserName || 'Me')
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
  msgs.forEach((m) => {
    if (!!m.text && m.text.indexOf(' GMT') >= 0) {
      m.text = m.text.split(/[^[]*] /).slice(1)[0]
    }
    if (!m.text) {
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

const openImagePreview = (url) => {
  window.open(url, '_blank')
}

const parseMessages = (data) => {
  return data.map(item => {
    // sender may be "gcid/username" for group chat messages; strip the gcid prefix for display
    const rawSender = item.sender || ''
    const displaySender = rawSender.indexOf('/') !== -1 ? rawSender.split('/')[1] : rawSender
    const isSent = displaySender === props.currentUserName
    const senderDisplay = props.chat?.isGroup ? resolveEpDisplayName(displaySender) : displaySender
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
  if (props.chat.isOpenclaw) return
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

// ── peer mode (P/N/M/A buttons) ─────────────────────────────────────────────────

function derivePeerMode(cfg) {
  if (!cfg) return 'manual'
  if (cfg.isBlocked) return 'blocked'
  if (cfg.muted) return 'muted'
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
  if (!key || !props.meshName) { peerMode.value = ''; return }
  try {
    const res = await chatService.getPeerConfig(props.meshName, key)
    peerMode.value = derivePeerMode(res.data)
  } catch {
    peerMode.value = 'manual'
  }
}

async function handlePeerModeChange(mode) {
  if (mode === peerMode.value) return
  const key = getPeerKey()
  if (!key || !props.meshName) return
  const configMap = {
    blocked: { isBlocked: true,  muted: false, autoReply: false },
    muted:   { isBlocked: false, muted: true,  autoReply: true  },
    manual:  { isBlocked: false, muted: false, autoReply: false },
    auto:    { isBlocked: false, muted: false, autoReply: true  },
  }
  try {
    await chatService.updatePeerConfig(props.meshName, key, configMap[mode])
    peerMode.value = mode
  } catch (e) {
    console.error('Failed to update peer mode:', e)
  }
}

// ── end peer mode ────────────────────────────────────────────────────────────────

watch(() => props.chat.name, () => {
  if (props.chat.name) {
    loadPeerMode()
    if (!props.chat.isOpenclaw) {
      fetchMessages().then(() => {
        startPolling()
      })
    }
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
