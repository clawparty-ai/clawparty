<template>
  <div class="half-automation-wrapper">
    <div class="resize-handle" @mousedown="startResize"></div>
    <div class="half-mode-bar">
      <div class="peer-mode-group">
        <button
          class="mode-btn"
          :class="{ active: currentMode === 'blocked', 'mode-blocked': currentMode === 'blocked' }"
          title="Blocked — 停止聊天，自动回复 You are blocked"
          @click="switchMode('blocked')"
        >P</button>
        <button
          class="mode-btn"
          :class="{ active: currentMode === 'muted', 'mode-muted': currentMode === 'muted' }"
          title="Muted — Agent 仍运行但回复不发送"
          @click="switchMode('muted')"
        >N</button>
        <button
          class="mode-btn"
          :class="{ active: currentMode === 'manual', 'mode-manual': currentMode === 'manual' }"
          title="Manual — 手动输入，不自动回复"
          @click="switchMode('manual')"
        >M</button>
        <button
          class="mode-btn"
          :class="{ active: currentMode === 'auto', 'mode-auto': currentMode === 'auto' }"
          title="Auto Reply — 自动回复"
          @click="switchMode('auto')"
        >A</button>
        <button
          class="mode-btn"
          :class="{ active: currentMode === 'half', 'mode-half': currentMode === 'half' }"
          title="Half Automation — AI生成回复，人工审核后发送"
          @click="switchMode('half')"
        >H</button>
      </div>
    </div>
    <div class="half-automation-container" :style="{ height: containerHeight + 'px' }">
      <div class="draft-section">
        <div class="section-header">
          <span class="section-title">AI 回复草稿</span>
          <button 
            class="send-draft-btn"
            @click="sendDraft"
            :disabled="!draftText.trim()"
          >
            发送草稿
          </button>
        </div>
        <textarea
          ref="draftTextarea"
          v-model="draftText"
          class="draft-textarea"
          placeholder="AI 生成的回复将显示在这里，你可以直接编辑..."
          @input="handleDraftInput"
        ></textarea>
      </div>
      <div class="hint-section">
        <div class="section-header">
          <span class="section-title">人类批注</span>
          <button 
            class="rewrite-btn"
            @click="rewrite"
            :disabled="!hintText.trim() || rewriting"
            :class="{ loading: rewriting }"
          >
            <svg v-if="!rewriting" width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
            </svg>
            {{ rewriting ? '生成中...' : 'Rewrite' }}
          </button>
        </div>
        <textarea
          ref="hintTextarea"
          v-model="hintText"
          class="hint-textarea"
          placeholder="输入你的修改建议或指导，点击 Rewrite 让 AI 重新生成..."
          @keydown="handleHintKeydown"
        ></textarea>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, nextTick } from 'vue'
import { chatService } from '../services/chatService'

const props = defineProps({
  meshName: {
    type: String,
    required: true
  },
  peerName: {
    type: String,
    required: true
  },
  sessionId: {
    type: String,
    default: ''
  },
  initialDraft: {
    type: String,
    default: ''
  },
  currentMode: {
    type: String,
    default: 'half'
  }
})

const emit = defineEmits(['send', 'draft-updated', 'update:peerMode'])

const draftText = ref(props.initialDraft)
const hintText = ref('')
const rewriting = ref(false)
const draftTextarea = ref(null)
const hintTextarea = ref(null)
const containerHeight = ref(200)
const isResizing = ref(false)
let startY = 0
let startHeight = 0

watch(() => props.initialDraft, (newVal) => {
  if (newVal && !draftText.value) {
    draftText.value = newVal
  }
})

const startResize = (e) => {
  isResizing.value = true
  startY = e.clientY
  startHeight = containerHeight.value
  document.addEventListener('mousemove', doResize)
  document.addEventListener('mouseup', stopResize)
  document.body.style.cursor = 'ns-resize'
  document.body.style.userSelect = 'none'
}

const doResize = (e) => {
  if (!isResizing.value) return
  const delta = startY - e.clientY
  const newHeight = Math.max(100, Math.min(500, startHeight + delta))
  containerHeight.value = newHeight
}

const stopResize = () => {
  isResizing.value = false
  document.removeEventListener('mousemove', doResize)
  document.removeEventListener('mouseup', stopResize)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}

function handleDraftInput() {
  emit('draft-updated', draftText.value)
}

async function rewrite() {
  if (!hintText.value.trim() || rewriting.value) return
  
  rewriting.value = true
  try {
    const res = await chatService.halfAutomationRewrite(
      props.meshName,
      props.peerName,
      draftText.value,
      hintText.value,
      props.sessionId
    )
    if (res.data?.text) {
      draftText.value = res.data.text
      emit('draft-updated', draftText.value)
      hintText.value = ''
    }
  } catch (e) {
    console.error('Rewrite failed:', e)
  } finally {
    rewriting.value = false
  }
}

function sendDraft() {
  if (!draftText.value.trim()) return
  emit('send', draftText.value)
}

function handleHintKeydown(e) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault()
    rewrite()
  }
}

function setDraft(text) {
  draftText.value = text
}

function switchMode(mode) {
  emit('update:peerMode', mode)
}

defineExpose({
  setDraft
})
</script>

<style scoped>
.half-automation-wrapper {
  display: flex;
  flex-direction: column;
}

.resize-handle {
  height: 4px;
  cursor: ns-resize;
  background: transparent;
  border-radius: 2px;
}

.resize-handle:hover {
  background: var(--slack-blue);
}

.half-mode-bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 8px 20px;
  border-top: 1px solid var(--border-light);
}

.peer-mode-group {
  display: flex;
  align-items: center;
  gap: 2px;
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

.mode-btn.active.mode-half {
  background: #8b5cf6;
  color: #fff;
}

.half-automation-container {
  display: flex;
  gap: 12px;
  padding: 12px 20px 24px;
  min-height: 100px;
}

.draft-section,
.hint-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-light);
  border-radius: 10px;
  overflow: hidden;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: #f8f8f8;
  border-bottom: 1px solid var(--border-subtle);
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.send-draft-btn,
.rewrite-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border: none;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}

.send-draft-btn {
  background: var(--slack-purple);
  color: #fff;
}

.send-draft-btn:hover:not(:disabled) {
  background: var(--slack-aubergine);
}

.send-draft-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.rewrite-btn {
  background: #8b5cf6;
  color: #fff;
}

.rewrite-btn:hover:not(:disabled) {
  background: #7c3aed;
}

.rewrite-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.rewrite-btn.loading {
  opacity: 0.7;
}

.draft-textarea,
.hint-textarea {
  flex: 1;
  width: 100%;
  padding: 10px 12px;
  border: none;
  resize: none;
  font-size: 14px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace;
  line-height: 1.5;
  outline: none;
  background: #fff;
}

.draft-textarea::placeholder,
.hint-textarea::placeholder {
  color: var(--text-secondary);
}

@media (max-width: 768px) {
  .half-automation-container {
    flex-direction: column;
    height: auto;
  }
  
  .draft-section,
  .hint-section {
    min-height: 120px;
  }
}
</style>
