<template>
  <div class="webshare-panel" :style="{ height: panelHeight + 'px' }">
    <div class="webshare-panel-header" @click="toggleExpanded">
      <span class="webshare-panel-icon">🌐</span>
      <span class="webshare-panel-title">共享文件</span>
      <span class="webshare-panel-count" v-if="files.length > 0">{{ files.length }} 个文件</span>
      <button class="refresh-btn" :class="{ spinning: refreshing }" @click.stop="onRefresh" title="刷新文件列表">
        <svg width="14" height="14" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z" clip-rule="evenodd"/>
        </svg>
      </button>
    </div>
    <div v-show="expanded" class="webshare-panel-body">
      <div v-if="files.length === 0" class="webshare-empty">
        <span class="webshare-empty-icon">📂</span>
        <span class="webshare-empty-text">暂无共享文件<br>在 agent workspace/web/ 目录下添加文件</span>
      </div>
      <div v-else class="webshare-file-list">
        <div class="webshare-file-row header-row">
          <span class="file-name-col">文件名</span>
          <span class="file-size-col">大小</span>
          <span class="file-time-col">修改时间</span>
        </div>
        <a
          v-for="file in files"
          :key="file.name"
          class="webshare-file-row"
          :href="fileUrl(file.name)"
          target="_blank"
          rel="noopener noreferrer"
          @click.stop
        >
          <span class="file-name-col" :title="file.name">
            <span class="file-icon">{{ fileIcon(file.name) }}</span>
            {{ file.name }}
          </span>
          <span class="file-size-col">{{ formatSize(file.size) }}</span>
          <span class="file-time-col">{{ formatTime(file.mtime) }}</span>
        </a>
      </div>
    </div>
    <div
      class="resize-handle"
      :class="{ resizing: isResizing }"
      @mousedown="startResize"
      @touchstart="startResize"
    ></div>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'
import { webshareService } from '../services/chatService'

const props = defineProps({
  agentName: {
    type: String,
    required: true
  },
  files: {
    type: Array,
    default: () => []
  },
  expanded: {
    type: Boolean,
    default: true
  },
  initialHeight: {
    type: Number,
    default: 180
  },
  refreshing: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['toggle', 'refresh'])

const toggleExpanded = () => {
  emit('toggle')
}

const onRefresh = () => {
  emit('refresh')
}

const fileUrl = (filename) => {
  return webshareService.getAgentWebshareFileUrl(props.agentName, filename)
}

const fileIcon = (name) => {
  const lower = name.toLowerCase()
  if (lower.endsWith('.html') || lower.endsWith('.htm')) return '🌐'
  if (lower.endsWith('.css')) return '🎨'
  if (lower.endsWith('.js') || lower.endsWith('.mjs')) return '⚡'
  if (lower.endsWith('.json')) return '📋'
  if (lower.endsWith('.png') || lower.endsWith('.jpg') || lower.endsWith('.jpeg') || lower.endsWith('.gif') || lower.endsWith('.webp') || lower.endsWith('.svg')) return '🖼️'
  if (lower.endsWith('.pdf')) return '📄'
  if (lower.endsWith('.md')) return '📝'
  if (lower.endsWith('.mp4') || lower.endsWith('.webm')) return '🎬'
  if (lower.endsWith('.mp3') || lower.endsWith('.wav') || lower.endsWith('.ogg')) return '🎵'
  return '📄'
}

const formatSize = (bytes) => {
  if (!bytes && bytes !== 0) return '-'
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1048576).toFixed(1) + ' MB'
}

const formatTime = (mtime) => {
  if (!mtime) return '-'
  const d = new Date(mtime * 1000)
  const yyyy = d.getFullYear()
  const mm = String(d.getMonth() + 1).padStart(2, '0')
  const dd = String(d.getDate()).padStart(2, '0')
  const hh = String(d.getHours()).padStart(2, '0')
  const min = String(d.getMinutes()).padStart(2, '0')
  return `${yyyy}-${mm}-${dd} ${hh}:${min}`
}

// Resizable panel height
const MIN_H = 60
const MAX_H = 500
const panelHeight = ref(props.initialHeight)

watch(() => props.initialHeight, (newH) => {
  panelHeight.value = newH
})

const isResizing = ref(false)
let startY = 0
let startH = 0

const startResize = (e) => {
  isResizing.value = true
  startY = e.clientY || e.touches?.[0]?.clientY || 0
  startH = panelHeight.value
  document.body.style.cursor = 'ns-resize'
  document.body.style.userSelect = 'none'
  window.addEventListener('mousemove', onResize)
  window.addEventListener('mouseup', stopResize)
  window.addEventListener('touchmove', onResize, { passive: false })
  window.addEventListener('touchend', stopResize)
}

const onResize = (e) => {
  if (!isResizing.value) return
  const y = e.clientY || e.touches?.[0]?.clientY || 0
  const delta = y - startY
  let h = startH + delta
  if (h < MIN_H) h = MIN_H
  if (h > MAX_H) h = MAX_H
  panelHeight.value = h
}

const stopResize = () => {
  isResizing.value = false
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
  window.removeEventListener('mousemove', onResize)
  window.removeEventListener('mouseup', stopResize)
  window.removeEventListener('touchmove', onResize)
  window.removeEventListener('touchend', stopResize)
}
</script>

<style scoped>
.webshare-panel {
  flex-shrink: 0;
  position: relative;
  background: var(--bg-panel, #e8ecf6);
  border-bottom: 1px solid var(--border-subtle, rgba(0, 0, 0, 0.07));
  display: flex;
  flex-direction: column;
}

.resize-handle {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 4px;
  cursor: ns-resize;
  z-index: 10;
}

.resize-handle::after {
  content: '';
  position: absolute;
  bottom: 1px;
  left: calc(50% - 16px);
  width: 32px;
  height: 2px;
  border-radius: 1px;
  background: rgba(0, 0, 0, 0.12);
  opacity: 0;
  transition: opacity 0.2s;
}

.resize-handle:hover::after,
.resize-handle.resizing::after {
  opacity: 1;
  background: rgba(64, 149, 254, 0.6);
}

.webshare-panel-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 16px;
  cursor: pointer;
  user-select: none;
  position: sticky;
  top: 0;
  background: var(--bg-panel, #e8ecf6);
  z-index: 2;
}

.webshare-panel-header:hover {
  background: rgba(0, 0, 0, 0.02);
}

.webshare-panel-icon {
  font-size: 14px;
}

.webshare-panel-title {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary, #4d4d4d);
}

.webshare-panel-count {
  font-size: 12px;
  color: var(--text-dim, #797979);
  margin-left: auto;
}

.webshare-panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0 10px;
  min-height: 0;
}

.webshare-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 24px 12px;
  gap: 8px;
  text-align: center;
}

.webshare-empty-icon {
  font-size: 28px;
  opacity: 0.7;
}

.webshare-empty-text {
  font-size: 14px;
  color: var(--text-dim, #797979);
  line-height: 1.6;
}

.webshare-file-list {
  padding: 0 16px;
}

.webshare-file-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 8px;
  border-radius: 4px;
  text-decoration: none;
  color: inherit;
  font-size: 14px;
  transition: background 0.15s;
}

.webshare-file-row:hover {
  background: rgba(64, 149, 254, 0.08);
}

.webshare-file-row.header-row {
  font-weight: 600;
  color: var(--text-secondary, #616061);
  font-size: 12px;
  padding: 4px 8px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  margin-bottom: 2px;
  pointer-events: none;
}

.webshare-file-row.header-row:hover {
  background: transparent;
}

.file-icon {
  font-size: 15px;
  margin-right: 4px;
}

.file-name-col {
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  display: flex;
  align-items: center;
}

.file-size-col {
  width: 70px;
  text-align: right;
  flex-shrink: 0;
  color: var(--text-secondary, #616061);
  font-size: 12px;
}

.file-time-col {
  width: 130px;
  text-align: right;
  flex-shrink: 0;
  color: var(--text-secondary, #616061);
  font-size: 12px;
}

.refresh-btn {
  width: 22px;
  height: 22px;
  border-radius: 4px;
  background: transparent;
  border: none;
  color: var(--text-dim, #797979);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-left: 8px;
  transition: all 0.15s;
  flex-shrink: 0;
}

.refresh-btn:hover {
  background: rgba(64, 149, 254, 0.1);
  color: #4095fe;
}

.refresh-btn.spinning svg {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@media (max-width: 768px) {
  .webshare-panel-header {
    padding: 6px 12px;
  }
  .webshare-file-list {
    padding: 0 12px;
  }
  .file-time-col {
    width: 110px;
  }
}
</style>
