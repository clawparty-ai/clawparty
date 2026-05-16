<template>
  <div class="wiki-panel" :class="{ fullscreen: isFullscreen }" :style="{ height: isFullscreen ? '100vh' : panelHeight + 'px' }">
    <div class="wiki-panel-header">
      <span class="wiki-panel-icon">📖</span>
      <span class="wiki-panel-title">Wiki</span>
      <div class="wiki-tabs">
        <button
          class="wiki-tab-btn"
          :class="{ active: activeTab === 'pages' }"
          @click="activeTab = 'pages'"
        >
          <span class="tab-icon">📄</span> 文档
        </button>
        <button
          class="wiki-tab-btn"
          :class="{ active: activeTab === 'graph' }"
          @click="activeTab = 'graph'"
        >
          <span class="tab-icon">🔗</span> 关系图
        </button>
      </div>
      <div class="wiki-search" v-if="activeTab === 'pages'">
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索页面..."
          class="wiki-search-input"
          @input="debouncedSearch"
        />
      </div>
      <button
        class="wiki-fullscreen-btn"
        :class="{ active: isFullscreen }"
        @click="toggleFullscreen"
        title="全屏"
      >
        <span>⛶</span>
      </button>
      <button class="refresh-btn" :class="{ spinning: refreshing }" @click.stop="onRefresh" title="刷新 Wiki">
        <svg width="14" height="14" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z" clip-rule="evenodd"/>
        </svg>
      </button>
    </div>
    
    <div v-show="expanded" class="wiki-panel-body">
      <!-- Pages Tab -->
      <div v-if="activeTab === 'pages'" class="wiki-pages-layout">
        <div class="wiki-sidebar">
          <!-- Upload raw files to wiki/raw/ -->
          <div
            class="wiki-upload-pane"
            :class="{ dragover: isDragOver }"
            @dragenter.prevent="onDragEnter"
            @dragover.prevent="onDragOver"
            @dragleave.prevent="onDragLeave"
            @drop.prevent="onDrop"
            @click="triggerFileSelect"
            title="拖拽或点击上传文档到 raw/"
          >
            <span class="wiki-upload-icon">📤</span>
            <span class="wiki-upload-text">上传文档</span>
            <span class="wiki-upload-hint">→ raw/</span>
            <div v-if="uploadStatus === 'uploading'" class="wiki-upload-status">上传中...</div>
            <div v-else-if="uploadStatus === 'success'" class="wiki-upload-status success">✓ 成功</div>
            <div v-else-if="uploadStatus === 'error'" class="wiki-upload-status error">✗ 失败</div>
            <input ref="fileInput" type="file" multiple style="display:none" @change="handleFileChange" />
          </div>
          <div v-if="treeLoading" class="wiki-loading">加载中...</div>
          <div v-else class="wiki-tree">
            <WikiTreeNode
              v-for="item in treeData"
              :key="item.path || item.name"
              :item="item"
              :level="0"
              :activePath="activePagePath"
              @select="handleSelectPage"
            />
          </div>
          <!-- Raw files section -->
          <div class="wiki-raw-section">
            <div class="wiki-raw-header">
              <span class="wiki-raw-icon">📁</span>
              <span class="wiki-raw-title">raw/</span>
              <span v-if="rawFiles.length > 0" class="wiki-raw-count">{{ rawFiles.length }}</span>
            </div>
            <div v-if="rawFilesLoading" class="wiki-raw-loading">加载中...</div>
            <div v-else-if="rawFiles.length === 0" class="wiki-raw-empty">暂无文件</div>
            <div v-else class="wiki-raw-list">
              <div
                v-for="file in rawFiles"
                :key="file.name"
                class="wiki-raw-item"
                :class="{ 'is-dir': file.type === 'dir' }"
                @click="file.type === 'file' && previewRawFile(file.name)"
              >
                <span class="wiki-raw-item-icon">{{ file.type === 'dir' ? '📁' : '📄' }}</span>
                <span class="wiki-raw-item-name" :title="file.name">{{ file.name }}</span>
                <span v-if="file.type === 'file' && file.size" class="wiki-raw-item-size">{{ formatSize(file.size) }}</span>
              </div>
            </div>
          </div>
        </div>
        
        <div class="wiki-viewer" ref="viewerRef">
          <div v-if="!activePage && refreshLogs.length === 0" class="wiki-empty">
            <span class="wiki-empty-icon">📖</span>
            <span class="wiki-empty-text">选择一个页面查看内容</span>
          </div>
          <div v-else-if="refreshLogs.length > 0 && !activePage" class="wiki-log-panel">
            <div class="wiki-log-header">🪵 Wiki 刷新日志</div>
            <div class="wiki-log-body" ref="logBodyRef">
              <div
                v-for="(log, idx) in refreshLogs"
                :key="idx"
                class="wiki-log-line"
                :class="'log-' + log.level"
              >
                <span v-if="log.time" class="wiki-log-time">{{ log.time }}</span>
                <span class="wiki-log-msg">{{ log.msg }}</span>
              </div>
            </div>
          </div>
          <div v-else class="wiki-page-content">
            <div class="wiki-page-title">{{ activePageTitle }}</div>
            <div class="wiki-markdown" v-html="renderedContent"></div>
          </div>
        </div>
      </div>
      
      <!-- Graph Tab -->
      <div v-else-if="activeTab === 'graph'" class="wiki-graph-container">
        <div v-show="graphLoading" class="wiki-loading">加载关系图中...</div>
        <canvas ref="graphCanvas" class="wiki-graph-canvas"></canvas>
        
        <div v-if="graphError" class="wiki-graph-error">{{ graphError }}</div>
        
        <div class="wiki-graph-legend">
          <div class="legend-item"><span class="legend-dot" style="background: #2eb67d;"></span> 实体</div>
          <div class="legend-item"><span class="legend-dot" style="background: #1d9bd1;"></span> 概念</div>
          <div class="legend-item"><span class="legend-dot" style="background: #797979;"></span> 页面</div>
          <div class="legend-item"><span class="legend-dot" style="background: #ecb22e;"></span> 原始资料</div>
        </div>
      </div>
    </div>
    
    <!-- Resize handle -->
    <div class="wiki-resize-handle" @mousedown="startResize"></div>
  </div>
</template>

<script setup>
import { ref, watch, computed, onMounted, nextTick } from 'vue'
import { marked } from 'marked'
import { wikiService } from '../services/chatService'
import WikiTreeNode from './WikiTreeNode.vue'

const props = defineProps({
  agentName: {
    type: String,
    required: true
  },
  expanded: {
    type: Boolean,
    default: true
  },
  initialHeight: {
    type: Number,
    default: 200
  },
  refreshing: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['toggle', 'refresh'])

const panelHeight = ref(props.initialHeight)
const activeTab = ref('pages')
const treeData = ref([])
const treeLoading = ref(false)
const activePage = ref(null)
const activePagePath = ref('')
const activePageTitle = ref('')
const pageContent = ref('')
const searchQuery = ref('')
const searchTimeout = ref(null)
const isFullscreen = ref(false)
const viewerRef = ref(null)
const graphCanvas = ref(null)
const graphLoading = ref(false)
const graphError = ref('')

// Raw files state
const rawFiles = ref([])
const rawFilesLoading = ref(false)

// Refresh logs state
const refreshLogs = ref([])
const logBodyRef = ref(null)

const addRefreshLog = (level, msg) => {
  const t = new Date()
  const hh = t.getHours().toString().padStart(2, '0')
  const mm = t.getMinutes().toString().padStart(2, '0')
  const ss = t.getSeconds().toString().padStart(2, '0')
  refreshLogs.value.push({ time: hh + ':' + mm + ':' + ss, level, msg })
  if (refreshLogs.value.length > 80) refreshLogs.value.shift()
  nextTick(() => {
    if (logBodyRef.value) logBodyRef.value.scrollTop = logBodyRef.value.scrollHeight
  })
}

// Upload state
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
      await wikiService.uploadRaw(props.agentName, uint8, file.name)
      
      // Auto-convert non-markdown files to markdown via LLM
      if (!file.name.toLowerCase().endsWith('.md')) {
        try {
          await wikiService.convert(props.agentName, file.name)
          console.log('[Wiki] Converted to markdown:', file.name)
        } catch (convertErr) {
          console.warn('[Wiki] Auto-convert failed (file kept in raw/):', file.name, convertErr)
        }
      }
    } catch (err) {
      console.error('[Wiki] Upload failed:', file.name, err)
      hasError = true
    }
  }
  uploadStatus.value = hasError ? 'error' : 'success'
  loadTree() // Refresh tree after upload
  setTimeout(() => {
    uploadStatus.value = ''
  }, 2000)
}

// Custom wiki link renderer for marked
const wikiLinkRenderer = {
  name: 'wikiLink',
  level: 'inline',
  start(src) { return src.match(/\[\[/)?.index },
  tokenizer(src, tokens) {
    const match = src.match(/^\[\[([^\]]+)\]\]/)
    if (match) {
      return {
        type: 'wikiLink',
        raw: match[0],
        text: match[1].trim(),
        tokens: []
      }
    }
  },
  renderer(token) {
    return `<a href="#" class="wiki-link" data-page="${token.text}" onclick="event.preventDefault(); return false;">${token.text}</a>`
  }
}

marked.use({ extensions: [wikiLinkRenderer] })

const renderedContent = computed(() => {
  if (!pageContent.value) return ''
  return marked.parse(pageContent.value)
})

// Load wiki tree
const loadTree = async () => {
  treeLoading.value = true
  try {
    const res = await wikiService.getTree(props.agentName)
    if (res.data && res.data.files) {
      treeData.value = buildTree(res.data.files, res.data.path || '')
    }
  } catch (e) {
    console.error('[Wiki] Failed to load tree:', e)
  } finally {
    treeLoading.value = false
  }
  // Also load raw files
  loadRawFiles()
}

const loadRawFiles = async () => {
  rawFilesLoading.value = true
  try {
    const res = await wikiService.getTree(props.agentName, 'raw')
    if (res.data && res.data.files) {
      rawFiles.value = res.data.files
    } else {
      rawFiles.value = []
    }
  } catch (e) {
    console.error('[Wiki] Failed to load raw files:', e)
    rawFiles.value = []
  } finally {
    rawFilesLoading.value = false
  }
}

const buildTree = (files, basePath) => {
  // Group files by directory
  const root = []
  const dirs = {}
  
  for (const file of files) {
    if (file.type === 'dir') {
      dirs[file.name] = {
        name: file.name,
        type: 'dir',
        path: basePath ? basePath + '/' + file.name : file.name,
        children: []
      }
      root.push(dirs[file.name])
    } else {
      root.push({
        name: file.name,
        type: 'file',
        path: basePath ? basePath + '/' + file.name : file.name
      })
    }
  }
  
  return root
}

const handleSelectPage = async (item) => {
  if (item.type === 'dir') return
  activePage.value = item
  activePagePath.value = item.path
  activePageTitle.value = item.name.replace('.md', '')
  
  try {
    const pathParts = item.path.split('/')
    const filename = pathParts.pop()
    const dirPath = pathParts.join('/')
    const res = await wikiService.getPage(props.agentName, filename, dirPath || undefined)
    pageContent.value = res.data || ''
    
    nextTick(() => {
      setupWikiLinks()
    })
  } catch (e) {
    console.error('[Wiki] Failed to load page:', e)
    pageContent.value = '加载失败'
  }
}

const setupWikiLinks = () => {
  if (!viewerRef.value) return
  const links = viewerRef.value.querySelectorAll('.wiki-link')
  links.forEach(link => {
    link.addEventListener('click', (e) => {
      e.preventDefault()
      const pageName = link.getAttribute('data-page')
      if (pageName) {
        // Try to find and open the page
        findAndOpenPage(pageName)
      }
    })
  })
}

const findAndOpenPage = async (pageName) => {
  // Search for page in tree
  const searchInTree = (items) => {
    for (const item of items) {
      if (item.type === 'file' && item.name.replace('.md', '') === pageName) {
        return item
      }
      if (item.children) {
        const found = searchInTree(item.children)
        if (found) return found
      }
    }
    return null
  }
  
  const found = searchInTree(treeData.value)
  if (found) {
    await handleSelectPage(found)
  }
}

const debouncedSearch = () => {
  if (searchTimeout.value) clearTimeout(searchTimeout.value)
  searchTimeout.value = setTimeout(() => {
    performSearch()
  }, 300)
}

const performSearch = async () => {
  if (!searchQuery.value.trim()) {
    await loadTree()
    return
  }
  
  try {
    const res = await wikiService.search(props.agentName, searchQuery.value)
    if (res.data && res.data.results) {
      treeData.value = res.data.results.map(r => ({
        name: r.name,
        type: 'file',
        path: r.path,
        title: r.title,
        preview: r.preview
      }))
    }
  } catch (e) {
    console.error('[Wiki] Search failed:', e)
  }
}

const toggleFullscreen = () => {
  isFullscreen.value = !isFullscreen.value
}

const onRefresh = async () => {
  emit('refresh')
  addRefreshLog('info', '开始刷新 Wiki...')
  try {
    const res = await wikiService.refresh(props.agentName)
    const data = res.data || {}
    const converted = data.converted || []
    const failed = data.failed || []
    const ingestedPages = data.ingested_pages || 0
    const totalLinks = data.total_links || 0
    
    if (converted.length > 0) {
      addRefreshLog('info', '已转换 ' + converted.length + ' 个文件')
      for (const f of converted) {
        addRefreshLog('info', '  ✓ ' + f)
      }
    }
    if (failed.length > 0) {
      addRefreshLog('error', '转换失败 ' + failed.length + ' 个文件')
      for (const f of failed) {
        addRefreshLog('error', '  ✗ ' + f)
      }
    }
    
    // Always show ingest summary (even if some converts failed)
    addRefreshLog('info', '已摄取 ' + ingestedPages + ' 个页面, ' + totalLinks + ' 个链接')
    
    if (converted.length === 0 && failed.length === 0) {
      addRefreshLog('info', '没有需要转换的新文件')
    }
  } catch (e) {
    addRefreshLog('error', '刷新失败: ' + (e?.message || e))
    console.error('[Wiki] Refresh failed:', e)
  }
  loadTree()
}

const formatSize = (bytes) => {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB'
}

const previewRawFile = async (filename) => {
  try {
    const res = await wikiService.getPage(props.agentName, filename, 'raw')
    pageContent.value = res.data || ''
    activePageTitle.value = filename
    activePagePath.value = 'raw/' + filename
    activePage.value = { name: filename, type: 'file', path: 'raw/' + filename }
    nextTick(() => setupWikiLinks())
  } catch (e) {
    console.error('[Wiki] Failed to preview raw file:', e)
  }
}

// Resize handling
let isResizing = false
let startY = 0
let startHeight = 0

const startResize = (e) => {
  isResizing = true
  startY = e.clientY
  startHeight = panelHeight.value
  document.addEventListener('mousemove', onResizeMove)
  document.addEventListener('mouseup', stopResize)
}

const onResizeMove = (e) => {
  if (!isResizing) return
  const delta = e.clientY - startY
  panelHeight.value = Math.max(60, Math.min(600, startHeight + delta))
}

const stopResize = () => {
  isResizing = false
  document.removeEventListener('mousemove', onResizeMove)
  document.removeEventListener('mouseup', stopResize)
}

// Graph rendering with d3-force
let simulation = null

const renderGraph = async () => {
  graphLoading.value = true
  graphError.value = ''
  
  try {
    const res = await wikiService.getGraph(props.agentName)
    const { nodes, links } = res.data || { nodes: [], links: [] }
    console.log('[Wiki] Graph data:', nodes.length, 'nodes,', links.length, 'links')
    
    if (!nodes || nodes.length === 0) {
      graphError.value = '暂无关系数据'
      graphLoading.value = false
      return
    }
    
    // Hide loading, show canvas
    graphLoading.value = false
    await nextTick()
    
    const canvas = graphCanvas.value
    if (!canvas) {
      console.error('[Wiki] Canvas ref not found')
      return
    }
    
    const ctx = canvas.getContext('2d')
    const rect = canvas.parentElement.getBoundingClientRect()
    console.log('[Wiki] Canvas parent size:', rect.width, 'x', rect.height)
    
    if (rect.width === 0 || rect.height === 0) {
      console.error('[Wiki] Canvas parent has zero size')
      return
    }
    
    canvas.width = rect.width
    canvas.height = rect.height - 40 // Leave space for legend
    
    const width = canvas.width
    const height = canvas.height
    
    // Color mapping
    const colors = {
      entity: '#2eb67d',
      concept: '#1d9bd1',
      page: '#797979',
      raw: '#ecb22e'
    }
    
    // Initialize node positions
    nodes.forEach(node => {
      node.x = width / 2 + (Math.random() - 0.5) * 200
      node.y = height / 2 + (Math.random() - 0.5) * 200
      node.vx = 0
      node.vy = 0
    })
    
    // Simple force simulation
    const k = Math.sqrt((width * height) / nodes.length) * 0.8
    const maxIterations = 300
    
    for (let i = 0; i < maxIterations; i++) {
      // Repulsion
      for (let a = 0; a < nodes.length; a++) {
        for (let b = a + 1; b < nodes.length; b++) {
          const dx = nodes[b].x - nodes[a].x
          const dy = nodes[b].y - nodes[a].y
          let dist = Math.sqrt(dx * dx + dy * dy)
          if (dist < 1) dist = 1
          const force = (k * k) / dist
          const fx = (dx / dist) * force
          const fy = (dy / dist) * force
          nodes[a].vx -= fx
          nodes[a].vy -= fy
          nodes[b].vx += fx
          nodes[b].vy += fy
        }
      }
      
      // Attraction (links)
      for (const link of links) {
        const source = nodes[link.source]
        const target = nodes[link.target]
        if (!source || !target) continue
        const dx = target.x - source.x
        const dy = target.y - source.y
        let dist = Math.sqrt(dx * dx + dy * dy)
        if (dist < 1) dist = 1
        const force = (dist * dist) / k
        const fx = (dx / dist) * force * 0.05
        const fy = (dy / dist) * force * 0.05
        source.vx += fx
        source.vy += fy
        target.vx -= fx
        target.vy -= fy
      }
      
      // Center gravity
      for (const node of nodes) {
        const dx = width / 2 - node.x
        const dy = height / 2 - node.y
        node.vx += dx * 0.01
        node.vy += dy * 0.01
      }
      
      // Update positions
      for (const node of nodes) {
        node.vx *= 0.5 // Damping
        node.vy *= 0.5
        node.x += node.vx
        node.y += node.vy
        
        // Keep in bounds
        const margin = 20
        node.x = Math.max(margin, Math.min(width - margin, node.x))
        node.y = Math.max(margin, Math.min(height - margin, node.y))
      }
    }
    
    // Draw
    ctx.clearRect(0, 0, width, height)
    
    // Draw links
    ctx.strokeStyle = 'rgba(0, 0, 0, 0.15)'
    ctx.lineWidth = 1
    for (const link of links) {
      const source = nodes[link.source]
      const target = nodes[link.target]
      if (!source || !target) continue
      ctx.beginPath()
      ctx.moveTo(source.x, source.y)
      ctx.lineTo(target.x, target.y)
      ctx.stroke()
    }
    
    // Draw nodes
    for (const node of nodes) {
      const radius = node.category === 'entity' ? 8 : 6
      ctx.beginPath()
      ctx.arc(node.x, node.y, radius, 0, 2 * Math.PI)
      ctx.fillStyle = colors[node.category] || colors.page
      ctx.fill()
      ctx.strokeStyle = '#fff'
      ctx.lineWidth = 2
      ctx.stroke()
      
      // Draw label
      ctx.fillStyle = '#4d4d4d'
      ctx.font = '11px sans-serif'
      ctx.textAlign = 'center'
      ctx.fillText(node.name, node.x, node.y + radius + 14)
    }
    
  } catch (e) {
    console.error('[Wiki] Graph render failed:', e)
    graphError.value = '渲染失败'
  } finally {
    graphLoading.value = false
  }
}

// Watch for tab changes
watch(activeTab, (newTab) => {
  if (newTab === 'graph') {
    renderGraph()
  }
})

// Load tree on mount
onMounted(() => {
  loadTree()
})

// Watch for agent changes
watch(() => props.agentName, () => {
  loadTree()
  activePage.value = null
  activePagePath.value = ''
  pageContent.value = ''
})
</script>

<style scoped>
.wiki-panel {
  display: flex;
  flex-direction: column;
  border-bottom: 1px solid var(--border-subtle);
  background: var(--bg-secondary);
  position: relative;
}

.wiki-panel-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.wiki-panel-icon {
  font-size: 14px;
}

.wiki-panel-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.wiki-tabs {
  display: flex;
  gap: 4px;
}

.wiki-tab-btn {
  padding: 4px 10px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 4px;
}

.wiki-tab-btn:hover {
  background: var(--bg-hover);
}

.wiki-tab-btn.active {
  background: rgba(64, 149, 254, 0.15);
  color: #4095fe;
}

.tab-icon {
  font-size: 11px;
}

.wiki-search {
  flex: 1;
  max-width: 200px;
}

.wiki-search-input {
  width: 100%;
  padding: 4px 10px;
  border: 1px solid var(--border-light);
  border-radius: 4px;
  font-size: 12px;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.wiki-search-input:focus {
  outline: none;
  border-color: var(--slack-blue);
}

.wiki-fullscreen-btn {
  width: 28px;
  height: 28px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
}

.wiki-fullscreen-btn:hover,
.wiki-fullscreen-btn.active {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.refresh-btn {
  width: 28px;
  height: 28px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s;
}

.refresh-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.refresh-btn.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.wiki-panel-body {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.wiki-panel.fullscreen {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
  background: var(--bg-secondary);
}

.wiki-pages-layout {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.wiki-sidebar {
  width: 200px;
  border-right: 1px solid var(--border-subtle);
  overflow-y: auto;
  flex-shrink: 0;
  padding: 8px;
}

.wiki-upload-pane {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  padding: 8px 4px;
  margin-bottom: 8px;
  border: 2px dashed var(--border-light);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  background: var(--bg-primary);
  min-height: 48px;
}

.wiki-upload-pane:hover,
.wiki-upload-pane.dragover {
  border-color: var(--slack-blue);
  background: rgba(64, 149, 254, 0.05);
}

.wiki-upload-icon {
  font-size: 14px;
}

.wiki-upload-text {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
}

.wiki-upload-hint {
  font-size: 10px;
  color: var(--text-tertiary);
}

.wiki-upload-status {
  font-size: 10px;
  margin-top: 2px;
}

.wiki-upload-status.success {
  color: #2eb67d;
}

.wiki-upload-status.error {
  color: #e01e5a;
}

.wiki-tree {
  font-size: 12px;
}

.wiki-raw-section {
  margin-top: 12px;
  border-top: 1px solid var(--border-subtle);
  padding-top: 8px;
}

.wiki-raw-header {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-bottom: 6px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
}

.wiki-raw-icon {
  font-size: 12px;
}

.wiki-raw-title {
  flex: 1;
}

.wiki-raw-count {
  background: var(--bg-hover);
  padding: 1px 6px;
  border-radius: 10px;
  font-size: 10px;
  color: var(--text-tertiary);
}

.wiki-raw-loading,
.wiki-raw-empty {
  font-size: 11px;
  color: var(--text-tertiary);
  padding: 4px 0;
}

.wiki-raw-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.wiki-raw-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 11px;
  color: var(--text-primary);
  transition: background 0.15s;
}

.wiki-raw-item:hover {
  background: var(--bg-hover);
}

.wiki-raw-item.is-dir {
  cursor: default;
}

.wiki-raw-item-icon {
  font-size: 12px;
  flex-shrink: 0;
}

.wiki-raw-item-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.wiki-raw-item-size {
  font-size: 10px;
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.wiki-viewer {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.wiki-log-panel {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  border-left: 1px solid var(--border-subtle);
}

.wiki-log-header {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  padding: 6px 10px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.wiki-log-body {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px;
  font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  font-size: 11px;
  line-height: 1.5;
}

.wiki-log-line {
  white-space: pre-wrap;
  word-break: break-word;
  margin-bottom: 1px;
}

.wiki-log-time {
  color: var(--text-tertiary);
  margin-right: 6px;
}

.wiki-log-line.log-info  { color: #337ab7; }
.wiki-log-line.log-warn  { color: #f0ad4e; }
.wiki-log-line.log-error { color: #d9534f; }
.wiki-log-line.log-debug { color: #888888; }

.wiki-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-secondary);
  gap: 8px;
}

.wiki-empty-icon {
  font-size: 32px;
  opacity: 0.5;
}

.wiki-empty-text {
  font-size: 13px;
}

.wiki-page-title {
  font-size: 18px;
  font-weight: 700;
  margin-bottom: 16px;
  color: var(--text-primary);
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-subtle);
}

.wiki-markdown :deep(h1),
.wiki-markdown :deep(h2),
.wiki-markdown :deep(h3),
.wiki-markdown :deep(h4) {
  margin-top: 16px;
  margin-bottom: 8px;
  color: var(--text-primary);
}

.wiki-markdown :deep(h1) { font-size: 1.5em; }
.wiki-markdown :deep(h2) { font-size: 1.3em; }
.wiki-markdown :deep(h3) { font-size: 1.15em; }

.wiki-markdown :deep(p) {
  margin-bottom: 10px;
  line-height: 1.6;
  color: var(--text-primary);
}

.wiki-markdown :deep(code) {
  background: rgba(0, 0, 0, 0.05);
  padding: 2px 5px;
  border-radius: 3px;
  font-size: 13px;
  font-family: monospace;
}

.wiki-markdown :deep(pre) {
  background: #f5f5f5;
  padding: 12px;
  border-radius: 6px;
  overflow-x: auto;
  margin: 10px 0;
}

.wiki-markdown :deep(pre code) {
  background: none;
  padding: 0;
}

.wiki-markdown :deep(ul),
.wiki-markdown :deep(ol) {
  margin-bottom: 10px;
  padding-left: 24px;
}

.wiki-markdown :deep(li) {
  margin-bottom: 4px;
}

.wiki-markdown :deep(blockquote) {
  border-left: 3px solid var(--slack-blue);
  padding-left: 12px;
  margin: 10px 0;
  color: var(--text-secondary);
}

.wiki-markdown :deep(a) {
  color: var(--text-link);
  text-decoration: none;
}

.wiki-markdown :deep(a:hover) {
  text-decoration: underline;
}

.wiki-markdown :deep(.wiki-link) {
  color: #1d9bd1;
  text-decoration: none;
  cursor: pointer;
}

.wiki-markdown :deep(.wiki-link:hover) {
  text-decoration: underline;
}

.wiki-markdown :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 10px 0;
}

.wiki-markdown :deep(th),
.wiki-markdown :deep(td) {
  border: 1px solid var(--border-light);
  padding: 6px 10px;
  text-align: left;
  font-size: 13px;
}

.wiki-markdown :deep(th) {
  background: var(--bg-hover);
  font-weight: 600;
}

.wiki-markdown :deep(img) {
  max-width: 100%;
  border-radius: 4px;
}

.wiki-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-secondary);
  font-size: 13px;
}

.wiki-resize-handle {
  height: 4px;
  cursor: ns-resize;
  background: transparent;
  flex-shrink: 0;
}

.wiki-resize-handle:hover {
  background: var(--slack-blue);
}

.wiki-graph-container {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.wiki-graph-canvas {
  width: 100%;
  height: 100%;
}

.wiki-graph-error {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  color: var(--text-secondary);
  font-size: 13px;
}

.wiki-graph-legend {
  position: absolute;
  bottom: 8px;
  left: 8px;
  display: flex;
  gap: 12px;
  background: rgba(255, 255, 255, 0.9);
  padding: 6px 10px;
  border-radius: 4px;
  font-size: 11px;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}
</style>
