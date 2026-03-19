<template>
  <div class="input-area" style="position:relative;">
    <div class="resize-handle" @mousedown="startResize"></div>
    <div class="editor-wrapper" :style="{ height: editorHeight + 'px' }">
      <div class="editor-toolbar">
        <!-- Format buttons (hidden, kept for future use) -->
        <div style="display:none">
          <button class="toolbar-btn" @click="insertFormat('**', '**')" title="粗体 (Ctrl+B)">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8.21 13c2.106 0 3.412-1.087 3.412-2.823 0-1.306-.984-2.283-2.324-2.386v-.055a2.176 2.176 0 0 0 1.852-2.14c0-1.51-1.162-2.46-3.014-2.46H3.843V13H8.21zM5.908 4.674h1.696c.963 0 1.517.451 1.517 1.244 0 .834-.629 1.32-1.73 1.32H5.908V4.673zm0 6.788V8.598h1.73c1.216 0 1.88.492 1.88 1.415 0 .943-.643 1.449-1.832 1.449H5.907z"/>
            </svg>
          </button>
          <button class="toolbar-btn" @click="insertFormat('*', '*')" title="斜体 (Ctrl+I)">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M7.991 11.674 9.53 4.455c.123-.595.246-.71 1.347-.807l.11-.52H7.211l-.11.52c1.06.096 1.128.212 1.005.807L6.57 11.674c-.123.595-.246.71-1.346.806l-.11.52h3.774l.11-.52c-1.06-.095-1.129-.211-1.006-.806z"/>
            </svg>
          </button>
          <button class="toolbar-btn" @click="insertFormat('`', '`')" title="行内代码">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M5.854 4.854a.5.5 0 1 0-.708-.708l-3.5 3.5a.5.5 0 0 0 0 .708l3.5 3.5a.5.5 0 0 0 .708-.708L2.707 8l3.147-3.146zm4.292 0a.5.5 0 0 1 .708-.708l3.5 3.5a.5.5 0 0 1 0 .708l-3.5 3.5a.5.5 0 0 1-.708-.708L13.293 8l-3.147-3.146z"/>
            </svg>
          </button>
          <button class="toolbar-btn" @click="insertFormat('```\n', '\n```')" title="代码块">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M14 1a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1h12zM2 0a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2H2z"/>
              <path d="M6.354 5.5H4a3 3 0 0 0 0 6h3a3 3 0 0 0 2.83-4H9c-.086 0-.17.01-.25.031A2 2 0 0 1 7 9.5H5a1 1 0 0 0 0 2h1.354a.5.5 0 0 0 .354-.854l-.854-.854z"/>
            </svg>
          </button>
          <button class="toolbar-btn" @click="insertFormat('[', '](url)')" title="链接">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4.715 6.542 3.343 7.914a3 3 0 1 0 4.243 4.243l1.828-1.829A3 3 0 0 0 8.586 5.5L8 6.086a1.002 1.002 0 0 0-.154.199 2 2 0 0 1 .861 3.337L6.88 11.45a2 2 0 1 1-2.83-2.83l.793-.792a4.018 4.018 0 0 1-.128-1.287z"/>
              <path d="M6.586 4.672A3 3 0 0 0 7.414 9.5l.775-.776a2 2 0 0 1-.896-3.346L9.12 3.55a2 2 0 1 1 2.83 2.83l-.793.792c.112.42.155.855.128 1.287l1.372-1.372a3 3 0 1 0-4.243-4.243L6.586 4.672z"/>
            </svg>
          </button>
          <button class="toolbar-btn" @click="insertFormat('\n- ', '')" title="无序列表">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path fill-rule="evenodd" d="M5 11.5a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5zm0-4a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5zm0-4a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5zm-3 1a1 1 0 1 0 0-2 1 1 0 0 0 0 2zm0 4a1 1 0 1 0 0-2 1 1 0 0 0 0 2zm0 4a1 1 0 1 0 0-2 1 1 0 0 0 0 2z"/>
            </svg>
          </button>
          <button class="toolbar-btn" @click="insertFormat('\n1. ', '')" title="有序列表">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path fill-rule="evenodd" d="M5 11.5a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5zm0-4a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5zm0-4a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5z"/>
              <path d="M1.713 11.865v-.474H2c.217 0 .363-.137.363-.317 0-.185-.158-.31-.361-.31-.223 0-.367.152-.373.31h-.59c.016-.467.373-.787.986-.787.588-.002.954.291.957.703a.595.595 0 0 1-.492.594v.033a.615.615 0 0 1 .569.631c.003.533-.502.8-1.051.8-.656 0-1-.37-1.008-.794h.582c.008.178.312.382.738.327.648-.062 1.129-.533 1.11-1.23-.005-.272.245-.435.76-.532.255-.048.492-.152.687-.2a.626.626 0 0 1 .226-.103.507.507 0 0 1 .339.14c.14.172.224.394.224.646 0 .197-.064.345-.224.461-.18.11-.414.165-.687.159H1.969c-.52 0-.82-.225-.848-.567-.03-.38-.196-.694-.437-.859-.247-.17-.566-.263-.863-.263-.478 0-.87.147-1.16.438-.287.29-.432.7-.432 1.14v.474h2.145z"/>
            </svg>
          </button>
          <button class="toolbar-btn" @click="insertFormat('\n> ', '')" title="引用">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M2.5 3a.5.5 0 0 0 0 1h11a.5.5 0 0 0 0-1h-11zm0 3a.5.5 0 0 0 0 1h6a.5.5 0 0 0 0-1h-6zm0 3a.5.5 0 0 0 0 1h11a.5.5 0 0 0 0-1h-11zm0 3a.5.5 0 0 0 0 1h6a.5.5 0 0 0 0-1h-6z"/>
            </svg>
          </button>
          <button class="toolbar-btn" @click="insertFormat('~~', '~~')" title="删除线">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8.527 13.164c-2.153 0-3.589-1.107-3.705-2.81h1.23c.144 1.06 1.254 1.562 2.49 1.562 1.41 0 2.39-.751 2.39-1.961 0-.964-.697-1.541-1.841-1.844H9.88c.093.593.468 1.025 1.303 1.025.86 0 1.536-.468 1.697-1.306H11.5c-.11.625-.564.961-1.482.961-.943 0-1.694-.593-1.694-1.598 0-1.193.751-1.905 2.207-1.905 1.454 0 2.248.752 2.248 2.058 0 .768-.421 1.296-.984 1.296l-1.627-.016z"/>
              <path d="M1.5 8.5a.5.5 0 0 1 .5-.5h12a.5.5 0 0 1 0 1h-12a.5.5 0 0 1-.5-.5z"/>
            </svg>
          </button>
        </div>
        <button class="toolbar-btn" @click="triggerImagePicker" title="发送图片">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M6.002 5.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0z"/>
            <path d="M2.002 1a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V3a2 2 0 0 0-2-2h-12zm12 1a1 1 0 0 1 1 1v6.5l-3.777-1.947a.5.5 0 0 0-.577.093l-3.71 3.71-2.66-1.772a.5.5 0 0 0-.63.062L1.002 12V3a1 1 0 0 1 1-1h12z"/>
          </svg>
        </button>
        <input ref="imageInputRef" type="file" accept="image/*" multiple style="display:none" @change="handleImageSelect" />
        <button class="toolbar-btn" @click="triggerFilePicker" title="发送文件">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M14 4.5V14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V2a2 2 0 0 1 2-2h5.5L14 4.5zm-3 0A1.5 1.5 0 0 1 9.5 3V1H4a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V4.5h-2z"/>
            <path d="M4.5 8a.5.5 0 0 0 0 1h7a.5.5 0 0 0 0-1h-7zm0-2a.5.5 0 0 0 0 1h7a.5.5 0 0 0 0-1h-7zm0-2a.5.5 0 0 0 0 1h4a.5.5 0 0 0 0-1h-4z"/>
          </svg>
        </button>
        <input ref="fileInputRef" type="file" accept="*/*" multiple style="display:none" @change="handleFileSelect" />
        <span class="toolbar-spacer"></span>
        <div v-if="showPeerMode" class="peer-mode-group">
          <button
            class="mode-btn"
            :class="{ active: peerMode === 'blocked', 'mode-blocked': peerMode === 'blocked' }"
            title="Blocked — 停止聊天，自动回复 You are blocked"
            @click="$emit('update:peerMode', 'blocked')"
          >P</button>
          <button
            class="mode-btn"
            :class="{ active: peerMode === 'muted', 'mode-muted': peerMode === 'muted' }"
            title="Muted — Agent 仍运行但回复不发送"
            @click="$emit('update:peerMode', 'muted')"
          >N</button>
          <button
            class="mode-btn"
            :class="{ active: peerMode === 'manual', 'mode-manual': peerMode === 'manual' }"
            title="Manual — 手动输入，不自动回复"
            @click="$emit('update:peerMode', 'manual')"
          >M</button>
          <button
            class="mode-btn"
            :class="{ active: peerMode === 'auto', 'mode-auto': peerMode === 'auto' }"
            title="Auto Reply — 自动回复"
            @click="$emit('update:peerMode', 'auto')"
          >A</button>
        </div>
        <button 
          class="toolbar-btn send-btn" 
          @click="$emit('send')" 
          :disabled="loading"
          :class="{ loading: loading }"
          title="发送 (Ctrl+Enter)"
        >
          <svg v-if="!loading" width="18" height="18" viewBox="0 0 20 20" fill="currentColor">
            <path d="M10.894 2.553a1 1 0 00-1.788 0l-7 14a1 1 0 001.169 1.409l5-1.429A1 1 0 009 15.571V11a1 1 0 112 0v4.571a1 1 0 00.725.962l5 1.428a1 1 0 001.17-1.408l-7-14z"/>
          </svg>
          <svg v-else class="loading-icon" width="18" height="18" viewBox="0 0 20 20">
            <circle cx="10" cy="10" r="8" fill="none" stroke="currentColor" stroke-width="2" stroke-dasharray="40" stroke-dashoffset="10"/>
          </svg>
        </button>
      </div>
      <div class="editor-content">
        <textarea
          ref="textareaRef"
          :value="modelValue"
          @input="handleInput"
          @keydown="handleKeydown"
          @paste="handlePaste"
          @blur="closeMentionList"
          :placeholder="`发送消息到 #${chatName || 'channel'}`"
          rows="3"
        ></textarea>
      </div>
    </div>
    <div
      v-if="showMentionList && filteredMembers.length > 0"
      class="mention-dropdown"
    >
      <div
        v-for="(member, i) in filteredMembers"
        :key="member"
        class="mention-item"
        :class="{ active: i === selectedMentionIndex }"
        @mousedown.prevent="insertMention(member)"
      >@{{ resolveEpDisplayName(member) }}</div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, inject } from 'vue'

const props = defineProps({
  modelValue: {
    type: String,
    default: ''
  },
  chatName: {
    type: String,
    default: ''
  },
  loading: {
    type: Boolean,
    default: false
  },
  isOpenclaw: {
    type: Boolean,
    default: false
  },
  agentId: {
    type: String,
    default: ''
  },
  autoFocus: {
    type: Boolean,
    default: true
  },
  peerMode: {
    type: String,
    default: ''
  },
  showPeerMode: {
    type: Boolean,
    default: false
  },
  members: {
    type: Array,
    default: () => []
  }
})

const emit = defineEmits(['send', 'update:modelValue', 'hash-command', 'update:peerMode', 'send-images', 'send-files'])

const resolveEpDisplayName = inject('resolveEpDisplayName', (u) => u)

const textareaRef = ref(null)

// ── @ mention ────────────────────────────────────────────────────────────────
const showMentionList = ref(false)
const mentionFilter = ref('')
const mentionStartPos = ref(-1)
const selectedMentionIndex = ref(0)

const filteredMembers = computed(() => {
  if (!props.members.length) return []
  const q = mentionFilter.value.toLowerCase()
  if (!q) return props.members
  return props.members.filter(m => m.toLowerCase().includes(q) || resolveEpDisplayName(m).toLowerCase().includes(q))
})

function closeMentionList() {
  showMentionList.value = false
  mentionFilter.value = ''
  mentionStartPos.value = -1
  selectedMentionIndex.value = 0
}

function insertMention(member) {
  const textarea = textareaRef.value
  if (!textarea) return
  const text = props.modelValue
  const before = text.slice(0, mentionStartPos.value)
  const after = text.slice(textarea.selectionStart)
  const newText = before + '@' + member + ' ' + after
  emit('update:modelValue', newText)
  closeMentionList()
  const newPos = mentionStartPos.value + member.length + 2
  setTimeout(() => {
    textarea.focus()
    textarea.setSelectionRange(newPos, newPos)
  }, 0)
}
// ── Image upload ─────────────────────────────────────────────────────────────
const imageInputRef = ref(null)

function triggerImagePicker() {
  if (imageInputRef.value) imageInputRef.value.click()
}

function handleImageSelect(e) {
  const files = e.target.files
  if (!files || files.length === 0) return
  emitImageFiles(files)
  // Reset so the same file can be selected again
  if (imageInputRef.value) imageInputRef.value.value = ''
}

function handlePaste(e) {
  const items = e.clipboardData && e.clipboardData.items
  if (!items) return
  const imageFiles = []
  for (let i = 0; i < items.length; i++) {
    if (items[i].type.indexOf('image') !== -1) {
      const file = items[i].getAsFile()
      if (file) imageFiles.push(file)
    }
  }
  if (imageFiles.length > 0) {
    e.preventDefault()
    emitImageFiles(imageFiles)
  }
}

function emitImageFiles(fileList) {
  const files = []
  for (let i = 0; i < fileList.length; i++) {
    files.push(fileList[i])
  }
  emit('send-images', files)
}

const fileInputRef = ref(null)

function triggerFilePicker() {
  if (fileInputRef.value) fileInputRef.value.click()
}

function handleFileSelect(e) {
  const files = e.target.files
  if (!files || files.length === 0) return
  emitGenericFiles(files)
  if (fileInputRef.value) fileInputRef.value.value = ''
}

function emitGenericFiles(fileList) {
  const files = []
  for (let i = 0; i < fileList.length; i++) {
    files.push(fileList[i])
  }
  emit('send-files', files)
}

const editorHeight = ref(160)
const isResizing = ref(false)
let startY = 0
let startHeight = 0

const startResize = (e) => {
  isResizing.value = true
  startY = e.clientY
  startHeight = editorHeight.value
  document.addEventListener('mousemove', doResize)
  document.addEventListener('mouseup', stopResize)
  document.body.style.cursor = 'ns-resize'
  document.body.style.userSelect = 'none'
}

const doResize = (e) => {
  if (!isResizing.value) return
  const delta = startY - e.clientY
  const newHeight = Math.max(60, Math.min(400, startHeight + delta))
  editorHeight.value = newHeight
}

const stopResize = () => {
  isResizing.value = false
  document.removeEventListener('mousemove', doResize)
  document.removeEventListener('mouseup', stopResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}

const insertFormat = (prefix, suffix) => {
  const textarea = textareaRef.value
  if (!textarea) return
  
  const start = textarea.selectionStart
  const end = textarea.selectionEnd
  const text = props.modelValue
  const selectedText = text.substring(start, end)
  
  const newText = text.substring(0, start) + prefix + selectedText + suffix + text.substring(end)
  emit('update:modelValue', newText)
  
  setTimeout(() => {
    textarea.focus()
    if (selectedText) {
      textarea.setSelectionRange(start + prefix.length, end + prefix.length)
    } else {
      textarea.setSelectionRange(start + prefix.length, start + prefix.length)
    }
  }, 0)
}

function handleInput(e) {
  const val = e.target.value
  emit('update:modelValue', val)

  if (!props.members.length) return

  const cursor = e.target.selectionStart
  // Find the nearest '@' before cursor on the same line
  const textBeforeCursor = val.slice(0, cursor)
  const atIdx = textBeforeCursor.lastIndexOf('@')
  if (atIdx === -1) { closeMentionList(); return }

  // No spaces or newlines between '@' and cursor
  const fragment = textBeforeCursor.slice(atIdx + 1)
  if (/[\s\n]/.test(fragment)) { closeMentionList(); return }

  mentionStartPos.value = atIdx
  mentionFilter.value = fragment
  selectedMentionIndex.value = 0
  showMentionList.value = true
}

const handleKeydown = (e) => {
  // Handle mention list navigation first
  if (showMentionList.value && filteredMembers.value.length > 0) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      selectedMentionIndex.value = (selectedMentionIndex.value + 1) % filteredMembers.value.length
      return
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      selectedMentionIndex.value = (selectedMentionIndex.value - 1 + filteredMembers.value.length) % filteredMembers.value.length
      return
    }
    if (e.key === 'Enter' || e.key === 'Tab') {
      e.preventDefault()
      insertMention(filteredMembers.value[selectedMentionIndex.value])
      return
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      closeMentionList()
      return
    }
  }

  if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
    e.preventDefault()
    insertFormat('**', '**')
  } else if ((e.ctrlKey || e.metaKey) && e.key === 'i') {
    e.preventDefault()
    insertFormat('*', '*')
  } else if (e.key === 'Enter' && !e.shiftKey && !e.metaKey) {
    e.preventDefault()
    const val = e.target.value
    // Only intercept # commands in main agent chat
    if (props.isOpenclaw && props.agentId === 'main' && val.startsWith('#')) {
      emit('hash-command', val)
      emit('update:modelValue', '')
    } else {
      emit('send')
    }
  }
}
</script>

<style scoped>
.input-area {
  padding: 0 20px 24px;
}

.resize-handle {
  height: 4px;
  cursor: ns-resize;
  background: transparent;
  margin-bottom: 4px;
  border-radius: 2px;
}

.resize-handle:hover {
  background: var(--slack-blue);
}

.editor-wrapper {
  border: 1px solid var(--border-light);
  border-radius: 10px;
  overflow: hidden;
  transition: all 0.15s;
  display: flex;
  flex-direction: column;
}

.editor-wrapper:focus-within {
  border-color: var(--slack-blue);
  box-shadow: 0 0 0 1px var(--slack-blue);
}

.editor-toolbar {
  display: flex;
  align-items: center;
  padding: 6px 8px;
  background: #f8f8f8;
  border-bottom: 1px solid var(--border-subtle);
  gap: 2px;
  flex-shrink: 0;
}

.toolbar-btn {
  width: 28px;
  height: 28px;
  border-radius: 4px;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.1s;
}

.toolbar-btn:hover {
  background: rgba(0, 0, 0, 0.08);
  color: var(--text-primary);
}

.toolbar-btn.send-btn {
  background: var(--slack-purple);
  color: #fff;
  margin-left: auto;
}

.toolbar-btn.send-btn:hover {
  background: var(--slack-aubergine);
}

.toolbar-btn.send-btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.toolbar-btn.send-btn.loading {
  opacity: 0.7;
}

.loading-icon {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.toolbar-divider {
  width: 1px;
  height: 18px;
  background: var(--border-light);
  margin: 0 4px;
}

.toolbar-spacer {
  flex: 1;
}

.peer-mode-group {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-right: 6px;
  border: 1px solid var(--border-light);
  border-radius: 5px;
  overflow: hidden;
}

.mode-btn {
  width: 26px;
  height: 26px;
  border: none;
  border-right: 1px solid var(--border-light);
  background: transparent;
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.1s;
  padding: 0;
}

.mode-btn:last-child {
  border-right: none;
}

.mode-btn:hover {
  background: rgba(0, 0, 0, 0.07);
  color: var(--text-primary);
}

.mode-btn.active.mode-blocked {
  background: #dc2626;
  color: #fff;
}

.mode-btn.active.mode-muted {
  background: #f59e0b;
  color: #fff;
}

.mode-btn.active.mode-manual {
  background: #6b7280;
  color: #fff;
}

.mode-btn.active.mode-auto {
  background: #16a34a;
  color: #fff;
}

.editor-content {
  background: #fff;
  flex: 1;
  overflow: hidden;
}

.editor-content textarea {
  width: 100%;
  height: 100%;
  padding: 12px 14px;
  border: none;
  color: var(--text-primary);
  font-size: 14px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace;
  line-height: 1.5;
  resize: none;
  outline: none;
}

.editor-content textarea::placeholder {
  color: var(--text-secondary);
}

.mention-dropdown {
  position: absolute;
  bottom: 100%;
  left: 20px;
  background: #fff;
  border: 1px solid var(--border-light);
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  max-height: 180px;
  overflow-y: auto;
  z-index: 100;
  min-width: 160px;
}

.mention-item {
  padding: 7px 14px;
  font-size: 13px;
  cursor: pointer;
  color: var(--text-primary);
  white-space: nowrap;
}

.mention-item:hover,
.mention-item.active {
  background: #e3f2fd;
  color: #1565c0;
}
</style>
