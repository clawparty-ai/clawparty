<template>
  <div class="webshare-panel" :class="{ fullscreen: isFullscreen }" :style="{ height: isFullscreen ? '100vh' : panelHeight + 'px' }">
    <div class="webshare-panel-header" @click="toggleExpanded">
      <span class="webshare-panel-icon">📁</span>
      <span class="webshare-panel-title">共享文件</span>
      <span class="webshare-panel-breadcrumb" v-if="currentPath">{{ currentPath }}</span>
      <span class="webshare-panel-count" v-if="files.length > 0">{{ files.length }}</span>
      <div class="webshare-panel-actions">
        <button
          class="webshare-fullscreen-btn"
          :class="{ active: isFullscreen }"
          @click.stop="toggleFullscreen"
          title="全屏"
        >
          <span>⛶</span>
        </button>
        <button class="refresh-btn" :class="{ spinning: refreshing }" @click.stop="onRefresh" title="刷新文件列表">
        <svg width="14" height="14" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z" clip-rule="evenodd"/>
        </svg>
      </button>
      </div>
    </div>
    <div v-show="expanded" class="webshare-panel-body">
      <div class="webshare-layout">
        <!-- Left: Upload panel -->
        <div
          class="webshare-upload-pane"
          :class="{ dragover: isDragOver }"
          @dragenter.prevent="onDragEnter"
          @dragover.prevent="onDragOver"
          @dragleave.prevent="onDragLeave"
          @drop.prevent="onDrop"
          @click="triggerFileSelect"
        >
          <div class="upload-content">
            <span class="upload-icon">📤</span>
            <span class="upload-text">拖拽文件到此处上传</span>
            <span class="upload-hint">或点击选择文件</span>
            <div v-if="uploadStatus === 'uploading'" class="upload-status uploading">
              <span class="upload-spinner"></span>
              上传中...
            </div>
            <div v-else-if="uploadStatus === 'success'" class="upload-status success">✓ 上传成功</div>
            <div v-else-if="uploadStatus === 'error'" class="upload-status error">✗ 上传失败</div>
          </div>
          <input
            ref="fileInput"
            type="file"
            multiple
            style="display: none"
            @change="handleFileChange"
          />
        </div>
        <!-- Middle: File list -->
        <div class="webshare-list-pane">
          <div v-if="files.length === 0" class="webshare-empty">
            <span class="webshare-empty-icon">📂</span>
            <span class="webshare-empty-text">暂无共享文件<br>在 agent workspace/web/ 目录下添加文件</span>
          </div>
          <template v-else>
          <div class="breadcrumb-bar">
            <span class="breadcrumb-root" @click="goRoot">web</span>
            <template v-for="(seg, idx) in pathSegments" :key="idx">
              <span class="breadcrumb-sep">/</span>
              <span
                class="breadcrumb-seg"
                :class="{ active: idx === pathSegments.length - 1 }"
                @click="goUpTo(idx)"
              >{{ seg }}</span>
            </template>
          </div>
          <div class="webshare-file-list">
            <div class="webshare-file-row header-row">
              <span class="file-name-col header-sortable" :class="{ active: sortKey === 'name' }" @click="handleSort('name')">
                文件名<span class="sort-arrow">{{ sortIndicator('name') }}</span>
              </span>
              <span class="file-type-col header-sortable" :class="{ active: sortKey === 'type' }" @click="handleSort('type')">
                类型<span class="sort-arrow">{{ sortIndicator('type') }}</span>
              </span>
              <span class="file-size-col header-sortable" :class="{ active: sortKey === 'size' }" @click="handleSort('size')">
                大小<span class="sort-arrow">{{ sortIndicator('size') }}</span>
              </span>
              <span class="file-time-col header-sortable" :class="{ active: sortKey === 'mtime' }" @click="handleSort('mtime')">
                时间<span class="sort-arrow">{{ sortIndicator('mtime') }}</span>
              </span>
              <span class="file-actions-col">操作</span>
            </div>
            <div
              v-if="currentPath"
              class="webshare-file-row parent-dir-row"
              @click="goParent()"
              title="返回上级目录"
            >
              <span class="file-name-col">
                <span class="file-icon">⬆️</span>
                <span class="parent-dir-text">..</span>
              </span>
              <span class="file-size-col">-</span>
              <span class="file-type-col">-</span>
              <span class="file-time-col">-</span>
              <span class="file-actions-col"></span>
            </div>
            <div v-for="file in sortedFiles" :key="file.name + file.type">
              <div
                v-if="file.type === 'dir'"
                class="webshare-file-row dir-row"
                @click="enterDir(file.name)"
                :title="'进入 ' + file.name"
              >
                <span class="file-name-col">
                  <span class="file-icon">📁</span>
                  {{ file.name }}
                </span>
                  <span class="file-type-col">文件夹</span>
                  <span class="file-size-col">-</span>
                  <span class="file-time-col">-</span>
                  <span class="file-actions-col"></span>
                </div>
                 <div v-else class="webshare-file-row" @click="previewFile(file.name)">
                   <span class="file-name-col" :title="file.name">
                     <span class="file-icon">{{ fileIcon(file.name) }}</span>
                     {{ file.name }}
                   </span>
                   <span class="file-type-col">{{ extLabel(file.name) }}</span>
                 <span class="file-size-col">{{ formatSize(file.size) }}</span>
                <span class="file-time-col">{{ formatTime(file.mtime) }}</span>
                 <span class="file-actions-col">
                   <button class="action-btn copy-btn" title="复制文件URL" @click.stop="copyFileUrl(file.name)">复制</button>
                   <button class="action-btn preview-btn" title="预览" @click.stop="previewFile(file.name)">预览</button>
                   <button class="action-btn open-btn" title="在新标签页打开" @click.stop="openFile(file.name)">打开</button>
                </span>
              </div>
            </div>
          </div>
          </template>
        </div>
        <!-- Right: Preview panel -->
        <div class="webshare-preview-pane">
          <template v-if="previewUrl">
            <div class="preview-header">
              <span class="preview-title" :title="previewName">{{ previewName }}</span>
              <button class="preview-full-btn" @click="fullPreview = true" title="全屏预览">
                <svg width="14" height="14" viewBox="0 0 20 20" fill="currentColor">
                  <path fill-rule="evenodd" d="M3 4a1 1 0 011-1h4a1 1 0 010 2H6.414l2.293 2.293a1 1 0 01-1.414 1.414L5 6.414V8a1 1 0 01-2 0V4zm9 1a1 1 0 110-2h4a1 1 0 011 1v4a1 1 0 11-2 0V6.414l-2.293 2.293a1 1 0 11-1.414-1.414L13.586 5H12zm-9 7a1 1 0 112 0v1.586l2.293-2.293a1 1 0 011.414 1.414L5.414 15H7a1 1 0 110 2H3a1 1 0 01-1-1v-4zm13.707 4.707a1 1 0 010-1.414L14.586 15H13a1 1 0 110-2h4a1 1 0 011 1v4a1 1 0 11-2 0v-1.586l-2.293 2.293a1 1 0 01-1.414-1.414z" clip-rule="evenodd"/>
                </svg>
              </button>
              <button class="preview-close" @click="closePreview">✕</button>
            </div>
            <div class="preview-body">
              <video v-if="isVideoPreview" :src="previewUrl" class="preview-video" controls />
              <img v-else-if="isImagePreview" :src="previewUrl" class="preview-image" :alt="previewName" />
              <iframe v-else-if="isHtmlPreview" :src="previewUrl" class="preview-iframe" sandbox="allow-scripts allow-same-origin"></iframe>
              <iframe v-else-if="isHtmlPreview" :src="previewUrl" class="preview-iframe" sandbox="allow-scripts allow-same-origin"></iframe>
              <div v-else-if="isMarkdownPreview" class="preview-markdown" v-html="markdownContent"></div>
              <pre v-else-if="isTextPreview" class="preview-text">{{ textContent }}</pre>
              <div v-else class="preview-placeholder">
                <span class="preview-icon">{{ fileIcon(previewName) }}</span>
                <span class="preview-hint">暂不支持此文件预览</span>
                <a :href="previewUrl" target="_blank" rel="noopener noreferrer" class="preview-open-link">在新标签页打开</a>
              </div>
            </div>
          </template>
          <div v-else class="preview-empty">
            <span class="preview-empty-icon">📄</span>
            <span class="preview-empty-text">选择文件以预览</span>
          </div>
        </div>
      </div>
    </div>
    <!-- Fullscreen preview overlay -->
    <Teleport to="body">
      <div v-if="fullPreview" class="fullscreen-preview" @keyup.escape="fullPreview = false" tabindex="-1">
        <div class="fullscreen-preview-header">
          <span class="fullscreen-preview-title">{{ previewName }}</span>
          <a :href="previewUrl" target="_blank" rel="noopener noreferrer" class="fullscreen-open-link">在新标签页打开</a>
          <button class="fullscreen-close-btn" @click="fullPreview = false">✕ 退出全屏</button>
        </div>
        <div class="fullscreen-preview-body">
          <video v-if="isVideoPreview" :src="previewUrl" class="fullscreen-preview-video" controls />
          <img v-else-if="isImagePreview" :src="previewUrl" class="fullscreen-preview-image" :alt="previewName" />
          <iframe v-else-if="isHtmlPreview" :src="previewUrl" class="fullscreen-preview-iframe"></iframe>
          <div v-else-if="isMarkdownPreview" class="preview-markdown fullscreen" v-html="markdownContent"></div>
          <pre v-else-if="isTextPreview" class="preview-text fullscreen">{{ textContent }}</pre>
          <div v-else class="fullscreen-preview-placeholder">
            <span class="preview-icon">{{ fileIcon(previewName) }}</span>
            <span class="preview-hint">暂不支持此文件预览</span>
          </div>
        </div>
      </div>
    </Teleport>
    <div
      class="resize-handle"
      :class="{ resizing: isResizing }"
      @mousedown="startResize"
      @touchstart="startResize"
    ></div>
  </div>
</template>

<script setup>
import { ref, watch, computed, onUnmounted } from 'vue'
import { marked } from 'marked'
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
  },
  isFullscreen: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['toggle', 'refresh', 'uploaded', 'toggleFullscreen'])

const currentPath = ref('')
const previewUrl = ref('')
const previewName = ref('')
const fullPreview = ref(false)

const toggleFullscreen = () => {
  emit('toggleFullscreen')
}
const markdownContent = ref('')
const textContent = ref('')

// Upload drag & drop state
const isDragOver = ref(false)
const uploadStatus = ref('')
const fileInput = ref(null)
let dragCounter = 0

const onDragEnter = (e) => {
  dragCounter++
  isDragOver.value = true
}

const onDragOver = (e) => {
  isDragOver.value = true
}

const onDragLeave = (e) => {
  dragCounter--
  if (dragCounter <= 0) {
    isDragOver.value = false
    dragCounter = 0
  }
}

const onDrop = (e) => {
  dragCounter = 0
  isDragOver.value = false
  const files = e.dataTransfer?.files
  if (files && files.length > 0) {
    uploadFiles(files)
  }
}

const triggerFileSelect = () => {
  fileInput.value?.click()
}

const handleFileChange = (e) => {
  const files = e.target.files
  if (files && files.length > 0) {
    uploadFiles(files)
  }
  e.target.value = ''
}

const uploadFiles = async (files) => {
  uploadStatus.value = 'uploading'
  let hasError = false
  for (let i = 0; i < files.length; i++) {
    const file = files[i]
    try {
      const arrayBuffer = await file.arrayBuffer()
      const uint8 = new Uint8Array(arrayBuffer)
      await webshareService.uploadAgentWebshareFile(props.agentName, uint8, file.name, currentPath.value)
    } catch (err) {
      console.error('[WebShare] Upload failed:', file.name, err)
      hasError = true
    }
  }
  uploadStatus.value = hasError ? 'error' : 'success'
  emit('uploaded')
  setTimeout(() => {
    uploadStatus.value = ''
  }, 2000)
}

// Sorting state: 'name' | 'type' | 'size' | 'mtime'
const sortKey = ref('type')
const sortDir = ref('asc')

const handleSort = (key) => {
  if (sortKey.value === key) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortKey.value = key
    sortDir.value = 'asc'
  }
}

const sortIndicator = (key) => {
  if (sortKey.value !== key) return ''
  return sortDir.value === 'asc' ? ' ▲' : ' ▼'
}

const sortedFiles = computed(() => {
  const list = [...props.files]
  const key = sortKey.value
  const dir = sortDir.value
  if (!key) return list

  list.sort((a, b) => {
    let va, vb
    if (key === 'name') {
      va = a.name || ''
      vb = b.name || ''
    } else if (key === 'type') {
      const ext = (name) => {
        const idx = (name || '').lastIndexOf('.')
        return idx > 0 ? name.slice(idx).toLowerCase() : ''
      }
      va = a.type === 'dir' ? '\x00' + (a.name || '').toLowerCase() : ext(a.name) + '\x00' + (a.name || '').toLowerCase()
      vb = b.type === 'dir' ? '\x00' + (b.name || '').toLowerCase() : ext(b.name) + '\x00' + (b.name || '').toLowerCase()
    } else if (key === 'size') {
      va = a.size || 0
      vb = b.size || 0
    } else if (key === 'mtime') {
      va = a.mtime || 0
      vb = b.mtime || 0
    } else {
      return 0
    }

    if (typeof va === 'string' && typeof vb === 'string') {
      const cmp = va.localeCompare(vb)
      return dir === 'asc' ? cmp : -cmp
    }
    if (va < vb) return dir === 'asc' ? -1 : 1
    if (va > vb) return dir === 'asc' ? 1 : -1
    return 0
  })
  return list
})

const pathSegments = computed(() => {
  if (!currentPath.value) return []
  return currentPath.value.split('/').filter(Boolean)
})

const isImagePreview = computed(() => {
  const name = previewName.value.toLowerCase()
  return name.endsWith('.png') || name.endsWith('.jpg') || name.endsWith('.jpeg') || name.endsWith('.gif') || name.endsWith('.webp') || name.endsWith('.svg') || name.endsWith('.ico')
})

const isVideoPreview = computed(() => {
  const name = previewName.value.toLowerCase()
  return name.endsWith('.mp4') || name.endsWith('.webm') || name.endsWith('.ogg') || name.endsWith('.mov')
})

const isHtmlPreview = computed(() => {
  const name = previewName.value.toLowerCase()
  return name.endsWith('.html') || name.endsWith('.htm')
})

const isMarkdownPreview = computed(() => {
  const name = previewName.value.toLowerCase()
  return name.endsWith('.md') || name.endsWith('.markdown')
})

const isTextPreview = computed(() => {
  const name = previewName.value.toLowerCase()
  return name.endsWith('.txt') || name.endsWith('.text') || name.endsWith('.log')
})

const loadTextContent = async () => {
  if (!isTextPreview.value) {
    textContent.value = ''
    return
  }
  try {
    const res = await webshareService.getAgentWebshareFileContent(props.agentName, previewName.value, currentPath.value)
    var text
    if (typeof res === 'string') {
      text = res
    } else if (res instanceof ArrayBuffer) {
      const decoder = new TextDecoder('utf-8')
      text = decoder.decode(res)
    } else {
      text = res?.data instanceof ArrayBuffer ? new TextDecoder('utf-8').decode(res.data) : (res?.data || res)
    }
    textContent.value = text
  } catch (err) {
    console.error('[WebShare] Failed to load text:', err)
    textContent.value = '加载失败'
  }
}

const loadMarkdownContent = async () => {
  if (!isMarkdownPreview.value) {
    markdownContent.value = ''
    return
  }
  try {
    const res = await webshareService.getAgentWebshareFileContent(props.agentName, previewName.value, currentPath.value)
    var text
    if (typeof res === 'string') {
      text = res
    } else if (res instanceof ArrayBuffer) {
      const decoder = new TextDecoder('utf-8')
      text = decoder.decode(res)
    } else {
      text = res?.data instanceof ArrayBuffer ? new TextDecoder('utf-8').decode(res.data) : (res?.data || res)
    }
    markdownContent.value = marked(text)
  } catch (err) {
    console.error('[WebShare] Failed to load markdown:', err)
    markdownContent.value = '<p style="color:red;">加载失败</p>'
  }
}

const toggleExpanded = () => {
  if (!props.expanded) {
    emit('toggle')
  } else if (currentPath.value) {
    goRoot()
  } else {
    emit('toggle')
  }
}

const onRefresh = () => {
  emit('refresh', currentPath.value)
}

const fileUrl = (filename) => {
  return webshareService.getAgentWebshareFileUrl(props.agentName, filename, currentPath.value)
}

const enterDir = (dirName) => {
  currentPath.value = currentPath.value ? currentPath.value + '/' + dirName : dirName
  emit('refresh', currentPath.value)
  closePreview()
}

const goRoot = () => {
  if (!currentPath.value) return
  currentPath.value = ''
  emit('refresh', '')
  closePreview()
}

const goUpTo = (idx) => {
  const segs = pathSegments.value
  if (idx >= segs.length - 1) return
  currentPath.value = segs.slice(0, idx + 1).join('/')
  emit('refresh', currentPath.value)
  closePreview()
}

const goParent = () => {
  const segs = pathSegments.value
  if (segs.length <= 1) {
    currentPath.value = ''
  } else {
    currentPath.value = segs.slice(0, segs.length - 1).join('/')
  }
  emit('refresh', currentPath.value)
  closePreview()
}

const copyFileUrl = async (filename) => {
  const url = fileUrl(filename)
  try {
    await navigator.clipboard.writeText(url)
  } catch (e) {
    console.error('[WebShare] 复制失败:', e)
  }
}

const previewFile = async (filename) => {
  previewName.value = filename
  previewUrl.value = fileUrl(filename)
  if (isMarkdownPreview.value) {
    await loadMarkdownContent()
  } else if (isTextPreview.value) {
    await loadTextContent()
  } else {
    markdownContent.value = ''
    textContent.value = ''
  }
}

const openFile = (filename) => {
  const url = fileUrl(filename)
  window.open(url, '_blank', 'noopener,noreferrer')
}

const closePreview = () => {
  previewUrl.value = ''
  previewName.value = ''
  fullPreview.value = false
}

watch(fullPreview, (val) => {
  if (val) {
    document.body.classList.add('has-fullscreen-preview')
  } else {
    document.body.classList.remove('has-fullscreen-preview')
  }
})

onUnmounted(() => {
  document.body.classList.remove('has-fullscreen-preview')
})

// Auto-preview index.html when directory changes and no file is selected
watch(() => props.files, (newFiles) => {
  if (!previewUrl.value && newFiles && newFiles.length > 0) {
    const indexFile = newFiles.find(f => f.name === 'index.html')
    if (indexFile) {
      previewFile('index.html')
    }
  }
})

const fileIcon = (name) => {
  const lower = name.toLowerCase()
  if (lower.endsWith('.html') || lower.endsWith('.htm')) return '🌐'
  if (lower.endsWith('.css')) return '🎨'
  if (lower.endsWith('.js') || lower.endsWith('.mjs')) return '⚡'
  if (lower.endsWith('.json')) return '📋'
  if (lower.endsWith('.png') || lower.endsWith('.jpg') || lower.endsWith('.jpeg') || lower.endsWith('.gif') || lower.endsWith('.webp') || lower.endsWith('.svg')) return '🖼️'
  if (lower.endsWith('.pdf')) return '📄'
  if (lower.endsWith('.md')) return '📝'
  if (lower.endsWith('.txt') || lower.endsWith('.text') || lower.endsWith('.log')) return '📃'
  if (lower.endsWith('.mp4') || lower.endsWith('.webm')) return '🎬'
  if (lower.endsWith('.mp3') || lower.endsWith('.wav') || lower.endsWith('.ogg')) return '🎵'
  return '📄'
}

const extLabel = (name) => {
  const idx = (name || '').lastIndexOf('.')
  if (idx > 0) return name.slice(idx).toLowerCase()
  return '文件'
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
  box-shadow: -4px 0 8px rgba(0, 0, 0, 0.05), 0 4px 12px rgba(0, 0, 0, 0.08);
  z-index: 5;
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

.webshare-panel.fullscreen {
  position: fixed;
  top: var(--header-height, 56px);
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
}

.webshare-panel-actions {
  display: flex;
  gap: 4px;
  margin-left: auto;
}

.webshare-fullscreen-btn {
  width: 22px;
  height: 22px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--text-dim, #797979);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  transition: all 0.15s;
  flex-shrink: 0;
}

.webshare-fullscreen-btn:hover,
.webshare-fullscreen-btn.active {
  background: rgba(64, 149, 254, 0.1);
  color: #4095fe;
}

.webshare-panel-icon {
  font-size: 14px;
}

.webshare-panel-title {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary, #4d4d4d);
}

.webshare-panel-breadcrumb {
  font-size: 12px;
  color: var(--text-dim, #797979);
  margin-left: 4px;
  max-width: 200px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.webshare-panel-count {
  font-size: 12px;
  color: var(--text-dim, #797979);
}

.webshare-panel-body {
  flex: 1;
  overflow: hidden;
  min-height: 0;
  display: flex;
  flex-direction: column;
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

/* Left-right layout */
.webshare-layout {
  display: flex;
  flex-direction: row;
  flex: 1;
  overflow: hidden;
}

.webshare-upload-pane {
  width: 160px;
  min-width: 120px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-right: 1px solid rgba(0, 0, 0, 0.08);
  background: rgba(255, 255, 255, 0.5);
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
  padding: 8px;
}

.webshare-upload-pane:hover {
  background: rgba(64, 149, 254, 0.06);
}

.webshare-upload-pane.dragover {
  background: rgba(64, 149, 254, 0.15);
  border-color: #4095fe;
}

.upload-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  text-align: center;
  pointer-events: none;
}

.upload-icon {
  font-size: 28px;
  opacity: 0.7;
}

.upload-text {
  font-size: 13px;
  color: var(--text-secondary, #616061);
  font-weight: 500;
  line-height: 1.4;
}

.upload-hint {
  font-size: 11px;
  color: var(--text-dim, #797979);
}

.upload-status {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  margin-top: 4px;
  display: flex;
  align-items: center;
  gap: 4px;
  font-weight: 500;
}

.upload-status.uploading {
  color: #4095fe;
  background: rgba(64, 149, 254, 0.1);
}

.upload-status.success {
  color: #2eb67d;
  background: rgba(46, 182, 125, 0.1);
}

.upload-status.error {
  color: #e01e5a;
  background: rgba(224, 30, 90, 0.1);
}

.upload-spinner {
  display: inline-block;
  width: 10px;
  height: 10px;
  border: 2px solid rgba(64, 149, 254, 0.3);
  border-top-color: #4095fe;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.webshare-list-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
}

.webshare-preview-pane {
  width: 80%;
  max-width: 800px;
  min-width: 180px;
  display: flex;
  flex-direction: column;
  border-left: 1px solid rgba(0, 0, 0, 0.08);
  background: #ffffff;
}

.preview-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
}

.preview-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary, #616061);
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.preview-close {
  width: 20px;
  height: 20px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--text-dim, #797979);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  flex-shrink: 0;
}

.preview-close:hover {
  background: rgba(224, 30, 90, 0.1);
  color: #e01e5a;
}

.preview-body {
  flex: 1;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px;
}

.preview-image {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: 4px;
}

.preview-video {
  max-width: 100%;
  max-height: 100%;
  border-radius: 4px;
  outline: none;
}

.preview-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: var(--text-dim, #797979);
  text-align: center;
}

.preview-icon {
  font-size: 32px;
  opacity: 0.5;
}

.preview-hint {
  font-size: 13px;
}

.preview-open-link {
  font-size: 12px;
  color: #4095fe;
  text-decoration: none;
}

.preview-open-link:hover {
  text-decoration: underline;
}

.preview-iframe {
  width: 100%;
  height: 100%;
  border: none;
  background: #fff;
  border-radius: 4px;
}

.preview-markdown {
  width: 100%;
  height: 100%;
  overflow: auto;
  padding: 12px;
  font-size: 14px;
  line-height: 1.6;
  color: #333;
  box-sizing: border-box;
}

.preview-markdown.fullscreen {
  max-width: 900px;
  margin: 0 auto;
  padding: 24px;
  font-size: 15px;
}

.preview-markdown :deep(h1),
.preview-markdown :deep(h2),
.preview-markdown :deep(h3),
.preview-markdown :deep(h4) {
  margin: 16px 0 8px;
  font-weight: 600;
  color: #1a1a1a;
}

.preview-markdown :deep(h1) { font-size: 1.5em; }
.preview-markdown :deep(h2) { font-size: 1.3em; }
.preview-markdown :deep(h3) { font-size: 1.15em; }

.preview-markdown :deep(p) {
  margin: 8px 0;
}

.preview-markdown :deep(code) {
  background: #f0f0f0;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace;
  font-size: 0.9em;
}

.preview-markdown :deep(pre) {
  background: #f5f5f5;
  padding: 12px;
  border-radius: 6px;
  overflow-x: auto;
  margin: 12px 0;
}

.preview-markdown :deep(pre code) {
  background: none;
  padding: 0;
}

.preview-markdown :deep(ul),
.preview-markdown :deep(ol) {
  margin: 8px 0;
  padding-left: 24px;
}

.preview-markdown :deep(li) {
  margin: 4px 0;
}

.preview-markdown :deep(blockquote) {
  border-left: 4px solid #ddd;
  padding-left: 12px;
  margin: 12px 0;
  color: #666;
}

.preview-markdown :deep(a) {
  color: #4095fe;
  text-decoration: none;
}

.preview-markdown :deep(a:hover) {
  text-decoration: underline;
}

.preview-markdown :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: 12px 0;
}

.preview-markdown :deep(th),
.preview-markdown :deep(td) {
  border: 1px solid #ddd;
  padding: 8px 12px;
  text-align: left;
}

.preview-markdown :deep(th) {
  background: #f5f5f5;
  font-weight: 600;
}

.preview-markdown :deep(img) {
  max-width: 100%;
  border-radius: 4px;
}

.preview-text {
  width: 100%;
  height: 100%;
  overflow: auto;
  padding: 12px;
  margin: 0;
  font-size: 13px;
  line-height: 1.6;
  color: #333;
  background: #fafafa;
  border: none;
  box-sizing: border-box;
  white-space: pre-wrap;
  word-wrap: break-word;
  font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace;
}

.preview-text.fullscreen {
  max-width: 900px;
  margin: 0 auto;
  padding: 24px;
  font-size: 14px;
  background: #fff;
}

.preview-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  color: var(--text-dim, #797979);
}

.preview-empty-icon {
  font-size: 28px;
  opacity: 0.4;
}

.preview-empty-text {
  font-size: 13px;
}

/* Breadcrumb */
.breadcrumb-bar {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px 16px 2px;
  font-size: 12px;
  color: var(--text-secondary, #616061);
  flex-shrink: 0;
}

.breadcrumb-root {
  cursor: pointer;
  color: #4095fe;
  padding: 2px 4px;
  border-radius: 3px;
  transition: background 0.15s;
}

.breadcrumb-root:hover {
  background: rgba(64, 149, 254, 0.1);
}

.breadcrumb-sep {
  color: var(--text-dim, #797979);
  padding: 0 2px;
}

.breadcrumb-seg {
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 3px;
  transition: background 0.15s;
}

.breadcrumb-seg:hover {
  background: rgba(64, 149, 254, 0.1);
  color: #4095fe;
}

.breadcrumb-seg.active {
  color: var(--text-primary, #4d4d4d);
  cursor: default;
}

.breadcrumb-seg.active:hover {
  background: transparent;
  color: var(--text-primary, #4d4d4d);
}

/* File list */
.webshare-file-list {
  padding: 0 16px 8px;
  overflow-y: auto;
  flex: 1;
}

.webshare-file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  border-radius: 4px;
  text-decoration: none;
  color: inherit;
  font-size: 13px;
  transition: background 0.15s;
  cursor: pointer;
}

.webshare-file-row:hover {
  background: rgba(64, 149, 254, 0.08);
}

.webshare-file-row.header-row {
  font-weight: 600;
  color: var(--text-secondary, #616061);
  font-size: 11px;
  padding: 3px 8px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  margin-bottom: 2px;
  pointer-events: auto;
  cursor: default;
}

.webshare-file-row.header-row .header-sortable {
  cursor: pointer;
  user-select: none;
  border-radius: 3px;
  padding: 2px 4px;
  margin: -2px -4px;
  transition: background 0.15s, color 0.15s;
}

.webshare-file-row.header-row .header-sortable:hover {
  background: rgba(64, 149, 254, 0.1);
  color: #4095fe;
}

.webshare-file-row.header-row .header-sortable.active {
  color: #4095fe;
}

.sort-arrow {
  font-size: 10px;
  display: inline-block;
  margin-left: 2px;
}

.webshare-file-row.dir-row {
  color: var(--text-secondary, #616061);
  font-weight: 500;
  cursor: pointer;
}

.webshare-file-row.dir-row:hover {
  background: rgba(0, 0, 0, 0.04);
}

.webshare-file-row.parent-dir-row {
  color: var(--text-secondary, #616061);
  font-weight: 500;
  cursor: pointer;
}

.webshare-file-row.parent-dir-row:hover {
  background: rgba(64, 149, 254, 0.1);
}

.parent-dir-text {
  font-weight: 600;
  color: #4095fe;
}

.webshare-file-row.header-row:hover {
  background: transparent;
}

.file-icon {
  font-size: 14px;
  margin-right: 3px;
}

.file-name-col {
  flex: 1;
  min-width: 0;
  max-width: 55%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  display: flex;
  align-items: center;
}

.file-size-col {
  width: 58px;
  text-align: right;
  flex-shrink: 0;
  color: var(--text-secondary, #616061);
  font-size: 11px;
}

.file-type-col {
  width: 52px;
  text-align: right;
  flex-shrink: 0;
  color: var(--text-secondary, #616061);
  font-size: 11px;
}

.file-time-col {
  width: 110px;
  text-align: right;
  flex-shrink: 0;
  color: var(--text-secondary, #616061);
  font-size: 11px;
}

.file-actions-col {
  width: 144px;
  text-align: right;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 3px;
}

.action-btn {
  font-size: 11px;
  padding: 1px 5px;
  border-radius: 3px;
  border: none;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.15s;
}

.copy-btn {
  background: rgba(64, 149, 254, 0.12);
  color: #4095fe;
}

.copy-btn:hover {
  background: #4095fe;
  color: #fff;
}

.preview-btn {
  background: rgba(46, 182, 125, 0.12);
  color: #2eb67d;
}

.preview-btn:hover {
  background: #2eb67d;
  color: #fff;
}

.open-btn {
  background: rgba(96, 105, 124, 0.12);
  color: #60697c;
}

.open-btn:hover {
  background: #60697c;
  color: #fff;
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

/* Full preview button in preview header */
.preview-full-btn {
  width: 20px;
  height: 20px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--text-dim, #797979);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.15s;
}

.preview-full-btn:hover {
  background: rgba(64, 149, 254, 0.1);
  color: #4095fe;
}

/* Fullscreen preview overlay */
.fullscreen-preview {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 9999;
  background: #fff;
  display: flex;
  flex-direction: column;
}

.fullscreen-preview-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.1);
  background: #f9fafc;
  flex-shrink: 0;
}

.fullscreen-preview-title {
  font-size: 14px;
  font-weight: 600;
  color: #333;
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.fullscreen-open-link {
  font-size: 12px;
  color: #4095fe;
  text-decoration: none;
  flex-shrink: 0;
}

.fullscreen-open-link:hover {
  text-decoration: underline;
}

.fullscreen-close-btn {
  font-size: 13px;
  padding: 4px 12px;
  border-radius: 6px;
  border: 1px solid rgba(0, 0, 0, 0.15);
  background: #fff;
  color: #333;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s;
}

.fullscreen-close-btn:hover {
  background: #f0f0f0;
}

.fullscreen-preview-body {
  flex: 1;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.fullscreen-preview-image {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.fullscreen-preview-video {
  max-width: 100%;
  max-height: 100%;
  border-radius: 4px;
  outline: none;
}

.fullscreen-preview-iframe {
  width: 100%;
  height: 100%;
  border: none;
}

.fullscreen-preview-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: var(--text-dim, #797979);
}

@media (max-width: 768px) {
  .webshare-panel-header {
    padding: 6px 12px;
  }
  .webshare-file-list {
    padding: 0 12px 8px;
  }
  .webshare-preview-pane {
    display: none;
  }
}

@media print {
  .fullscreen-preview {
    position: static !important;
    height: auto !important;
    overflow: visible !important;
  }

  .fullscreen-preview-body {
    overflow: visible !important;
    display: block !important;
  }

  .fullscreen-preview-header {
    display: none !important;
  }

  .preview-markdown.fullscreen {
    max-width: 100% !important;
    padding: 0 !important;
  }
}
</style>
