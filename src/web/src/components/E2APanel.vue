<template>
  <div class="e2a-panel" :class="{ fullscreen: isFullscreen }" :style="isFullscreen ? {} : { height: panelHeight + 'px' }">
    <div class="e2a-panel-header" @click="toggleExpanded">
      <span class="e2a-panel-icon">📊</span>
      <span class="e2a-panel-title">Excel-to-Agent</span>
      <button
        class="mode-btn display-btn"
        :class="{ active: currentMode === 'display' }"
        @click.stop="switchMode('display')"
        title="数据展示"
      >
        <span class="mode-btn-icon">📋</span>
        <span class="mode-btn-text">数据展示</span>
      </button>
      <button
        v-if="false"
        class="mode-btn formula-btn"
        :class="{ active: currentMode === 'formula' }"
        @click.stop="switchMode('formula')"
        title="公式分析"
      >
        <span class="mode-btn-icon">📐</span>
        <span class="mode-btn-text">公式分析</span>
      </button>
      <button
        v-if="false"
        class="mode-btn fill-btn"
        :class="{ active: currentMode === 'fill' }"
        @click.stop="switchMode('fill')"
        title="数据填报"
      >
        <span class="mode-btn-icon">✍️</span>
        <span class="mode-btn-text">数据填报</span>
      </button>
      <div class="e2a-panel-actions">
        <button
          class="e2a-fullscreen-btn"
          :class="{ active: isFullscreen }"
          @click.stop="toggleFullscreen"
          title="全屏"
        >
          <span>⛶</span>
        </button>
        <button class="refresh-btn" :class="{ spinning: refreshing }" @click.stop="onRefresh" title="刷新">
          <svg width="14" height="14" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z" clip-rule="evenodd"/>
          </svg>
        </button>
      </div>
    </div>
    <div v-show="expanded" class="e2a-panel-body">
      <div class="e2a-layout">
        <!-- Left: Data source -->
        <div class="e2a-source-pane">
          <div
            class="e2a-upload-zone"
            :class="{ dragover: isDragOver }"
            @dragenter.prevent="onDragEnter"
            @dragover.prevent="onDragOver"
            @dragleave.prevent="onDragLeave"
            @drop.prevent="onDrop"
            @click="triggerFileSelect"
          >
            <div class="upload-content">
              <span class="upload-icon">📤</span>
              <span class="upload-text">拖拽 Excel 文件到此处</span>
              <span class="upload-hint">或点击选择文件 (.xlsx, .xls)</span>
              <div v-if="uploadStatus === 'uploading'" class="upload-status uploading">
                <span class="upload-spinner"></span>
                解析中...
              </div>
              <div v-else-if="uploadStatus === 'success'" class="upload-status success">✓ 解析成功</div>
              <div v-else-if="uploadStatus === 'error'" class="upload-status error">✗ 解析失败</div>
            </div>
            <input
              ref="fileInput"
              type="file"
              accept=".xlsx,.xls,.csv"
              style="display: none"
              @change="handleFileChange"
            />
          </div>
          <div class="e2a-dataset-list">
            <div v-if="datasets.length === 0" class="e2a-empty">
              <span>暂无数据集</span>
            </div>
            <div v-for="ds in datasets" :key="ds.name" class="dataset-item">
              <div
                class="dataset-header"
                :class="{ selected: selectedDataset === ds.name }"
                @click="toggleDataset(ds)"
              >
                <span class="dataset-arrow">{{ expandedDataset === ds.name ? '▼' : '▶' }}</span>
                <span class="dataset-icon">📊</span>
                <span class="dataset-name">{{ ds.name }}</span>
                <button class="dataset-delete" @click.stop="deleteDataset(ds.name)" title="删除">×</button>
              </div>
              <div v-show="expandedDataset === ds.name" class="dataset-sheets">
                <template v-for="sheet in ds.sheets.filter(s => !s.name.endsWith('-dashboard'))" :key="sheet.name">
                  <div
                    class="sheet-item"
                    :class="{ selected: selectedSheet === sheet.name && selectedDataset === ds.name }"
                    @click.stop="selectSheet(ds.name, sheet.name)"
                  >
                    <span class="sheet-icon">📋</span>
                    <span class="sheet-name">{{ sheet.name }}</span>
                    <span class="sheet-info">{{ sheet.row_count }}行 × {{ sheet.col_count }}列</span>
                  </div>
                </template>
                <template v-for="sheet in ds.sheets.filter(s => s.name.endsWith('-dashboard'))" :key="'dash-'+sheet.name">
                  <div
                    class="sheet-item dashboard-item"
                    :class="{ selected: selectedSheet === sheet.name && selectedDataset === ds.name }"
                    @click.stop="selectSheet(ds.name, sheet.name)"
                  >
                    <span class="sheet-icon">📈</span>
                    <span class="sheet-name">{{ sheet.name.replace('-dashboard', '') }}</span>
                    <span class="sheet-info">Dashboard</span>
                  </div>
                </template>
                <div
                  class="sheet-item special"
                  :class="{ selected: selectedSheet === 'formulas.md' && selectedDataset === ds.name }"
                  @click.stop="selectSheet(ds.name, 'formulas.md')"
                >
                  <span class="sheet-icon">📐</span>
                  <span class="sheet-name">公式清单</span>
                </div>
              </div>
            </div>
          </div>
        </div>
        <!-- Right: Display area -->
        <div class="e2a-display-pane">
          <!-- Mode: Data Analysis -->
          <div v-if="currentMode === 'analyze'" class="display-analyze">
            <template v-if="analyzeContent">
              <template v-if="analysisCharts.length > 0 || analyzeContent === 'loading'">
                <div class="analyze-charts" ref="analyzeChartsEl">
                  <div v-for="(chart, ci) in analysisCharts" :key="ci" class="analysis-chart-card">
                    <div class="analysis-chart-title">{{ chart.title }}</div>
                    <canvas :ref="el => setChartCanvas(ci, el)" class="analysis-chart-canvas"></canvas>
                  </div>
                  <div v-if="analyzeContent === 'loading'" class="analyze-loading">
                    <span class="upload-spinner"></span> 正在生成 Dashboard...
                  </div>
                </div>
                <div class="analyze-divider"></div>
                <div class="analyze-summary markdown-body" v-html="analysisSummary"></div>
              </template>
              <div v-else class="markdown-body" v-html="analyzeContent"></div>
            </template>
            <div v-else class="display-empty">
              <span class="empty-icon">🔍</span>
              <span class="empty-text">点击左侧 Sheet 触发智能分析，将生成 Dashboard + 文字总结</span>
            </div>
          </div>

          <!-- Mode: Data Display -->
          <div v-else-if="currentMode === 'display'" class="display-table">
            <div v-if="csvHeaders.length > 0" class="table-toolbar">
              <span class="toolbar-info">{{ selectedDataset }} / {{ displaySheetName }} — {{ csvRows.length }} 行</span>
              <div class="toolbar-actions">
                <select v-model="chartType" class="chart-select">
                  <option value="">选择图表类型</option>
                  <option value="bar">柱状图</option>
                  <option value="line">折线图</option>
                  <option value="pie">饼图</option>
                </select>
                <button v-if="chartType" class="chart-gen-btn" @click="generateChart">生成图表</button>
              </div>
            </div>
            <div v-if="csvHeaders.length > 0" class="data-table-wrap">
              <canvas v-if="chartData" ref="chartCanvas" class="chart-canvas" width="400" height="250"></canvas>
              <table class="data-table">
                <thead>
                  <tr>
                    <th v-for="(h, i) in csvHeaders" :key="i" @click="sortByColumn(i)">
                      {{ h }} <span class="sort-indicator">{{ sortColumn === i ? (sortAsc ? '▲' : '▼') : '' }}</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, ri) in pagedRows" :key="ri">
                    <td v-for="(cell, ci) in row" :key="ci">{{ cell }}</td>
                  </tr>
                </tbody>
              </table>
              <div class="pagination" v-if="totalPages > 1">
                <button :disabled="currentPage <= 0" @click="currentPage--">上一页</button>
                <span>{{ currentPage + 1 }} / {{ totalPages }}</span>
                <button :disabled="currentPage >= totalPages - 1" @click="currentPage++">下一页</button>
                <span class="page-size-label">每页</span>
                <select v-model.number="pageSize" @change="currentPage = 0">
                  <option :value="20">20</option>
                  <option :value="50">50</option>
                  <option :value="100">100</option>
                </select>
              </div>
            </div>
            <div v-else class="display-empty">
              <span class="empty-icon">📋</span>
              <span class="empty-text">在左侧选择一个 Sheet 以查看数据</span>
            </div>
          </div>

          <!-- Mode: Formula Analysis -->
          <div v-else-if="currentMode === 'formula'" class="display-formula">
            <div v-if="formulaContent" class="markdown-body" v-html="formulaContent"></div>
            <div v-else class="display-empty">
              <span class="empty-icon">📐</span>
              <span class="empty-text">上传 Excel 文件后，公式分析结果将显示在这里</span>
            </div>
          </div>

          <!-- Mode: Data Fill (placeholder) -->
          <div v-else-if="currentMode === 'fill'" class="display-fill">
            <div class="display-empty">
              <span class="empty-icon">✍️</span>
              <span class="empty-text">数据填报功能即将推出</span>
              <span class="empty-hint">通过模板映射和批量填报，将 Excel 数据自动写入目标系统</span>
            </div>
          </div>
        </div>
        <!-- Right: Regenerate panel (only in dashboard view) -->
        <div v-if="currentMode === 'analyze' && analyzeContent && selectedSheet && selectedSheet.endsWith('-dashboard')" class="e2a-regenerate-pane">
          <div class="regenerate-title">重新生成</div>
          <textarea
            v-model="regenerateSuggestion"
            class="regenerate-input"
            placeholder="输入重新生成建议..."
            rows="4"
          ></textarea>
          <button
            class="regenerate-btn"
            :disabled="isRegenerating"
            @click="regenerateDashboard"
          >
            <span v-if="isRegenerating" class="upload-spinner"></span>
            {{ isRegenerating ? '生成中...' : '重新生成' }}
          </button>
        </div>
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
import { ref, watch, computed, nextTick, onMounted, onUnmounted } from 'vue'
import { marked } from 'marked'
import { Chart, registerables } from 'chart.js'
import { e2aService } from '../services/e2aService'

try {
  Chart.register(...registerables)
} catch (e) {
  console.warn('[E2A] Chart.js registration failed:', e)
}

const props = defineProps({
  agentName: { type: String, required: true },
  datasets: { type: Array, default: () => [] },
  expanded: { type: Boolean, default: true },
  initialHeight: { type: Number, default: 180 },
  refreshing: { type: Boolean, default: false },
  isFullscreen: { type: Boolean, default: false },
  messages: { type: Array, default: () => [] }
})

const emit = defineEmits(['toggle', 'refresh', 'uploaded', 'toggleFullscreen', 'send'])

console.log('[E2APanel] component loaded')

const currentMode = ref('display')
const expandedDataset = ref(null)
const selectedDataset = ref(null)
const selectedSheet = ref(null)
const formulaContent = ref('')
const csvHeaders = ref([])
const csvRows = ref([])
const displaySheetName = ref('')
const currentPage = ref(0)
const pageSize = ref(50)
const sortColumn = ref(-1)
const sortAsc = ref(true)
const chartType = ref('')
const chartData = ref(null)
let chartInstance = null

const internalDatasets = ref([])

watch(() => props.datasets, (val) => {
  internalDatasets.value = val || []
})

const isDragOver = ref(false)
const uploadStatus = ref('')
const fileInput = ref(null)
let dragCounter = 0

const onDragEnter = () => { dragCounter++; isDragOver.value = true }
const onDragOver = () => { isDragOver.value = true }
const onDragLeave = () => { dragCounter--; if (dragCounter <= 0) { isDragOver.value = false; dragCounter = 0 } }
const onDrop = (e) => {
  dragCounter = 0; isDragOver.value = false
  const files = e.dataTransfer?.files
  if (files && files.length > 0) uploadFile(files[0])
}
const triggerFileSelect = () => { fileInput.value?.click() }
const handleFileChange = (e) => {
  const files = e.target.files
  if (files && files.length > 0) uploadFile(files[0])
  e.target.value = ''
}

const uploadFile = async (file) => {
  uploadStatus.value = 'uploading'
  try {
    const arrayBuffer = await file.arrayBuffer()
    const uint8 = new Uint8Array(arrayBuffer)
    await e2aService.upload(props.agentName, uint8, file.name)
    uploadStatus.value = 'success'
    emit('uploaded')
    setTimeout(() => { uploadStatus.value = '' }, 2000)
  } catch (err) {
    console.error('[E2A] Upload failed:', err)
    uploadStatus.value = 'error'
    setTimeout(() => { uploadStatus.value = '' }, 2000)
  }
}

const switchMode = (mode) => {
  currentMode.value = mode
  if (mode === 'formula' && selectedDataset.value) {
    loadFormulaContent()
  }
}

const toggleDataset = (ds) => {
  if (expandedDataset.value === ds.name) {
    expandedDataset.value = null
  } else {
    expandedDataset.value = ds.name
  }
}

const selectSheet = async (dsName, sheetName) => {
  selectedDataset.value = dsName
  selectedSheet.value = sheetName
  if (sheetName.endsWith('-dashboard')) {
    await loadDashboard(dsName, sheetName)
  } else if (sheetName === 'formulas.md') {
    currentMode.value = 'formula'
    await loadFormulaContent()
  } else {
    // Check if a dashboard already exists
    const dashName = sheetName + '-dashboard'
    const ds = internalDatasets.value.find(d => d.name === dsName)
    const hasDashboard = ds && ds.sheets && ds.sheets.some(s => s.name === dashName)
    if (hasDashboard) {
      // Show markdown content directly
      await loadSheetMarkdown(dsName, sheetName)
    } else {
      // Trigger analysis in background, show markdown while waiting
      await loadSheetMarkdown(dsName, sheetName)
      await analyzeSheet(dsName, sheetName)
    }
  }
}

const loadDashboard = async (dsName, dashboardName) => {
  try {
    const filename = dashboardName.endsWith('.md') ? dashboardName : dashboardName + '.md'
    const res = await e2aService.getFile(props.agentName, dsName, filename)
    const content = typeof res === 'string' ? res : res?.data || ''
    if (!content) {
      analyzeContent.value = '<p style="color:red;">Dashboard 文件为空</p>'
      return
    }
    const { charts, text } = parseAnalysisResult(content)
    analyzeContent.value = content
    analysisCharts.value = charts
    analysisSummary.value = marked.parse(text)
    currentMode.value = 'analyze'
    await nextTick()
    if (charts.length > 0) {
      renderAnalysisCharts()
    }
  } catch (e) {
    console.error('[E2A] Failed to load dashboard:', e)
    analyzeContent.value = '<p style="color:red;">加载 Dashboard 失败</p>'
  }
}

const regenerateDashboard = async () => {
  if (!selectedSheet.value || !selectedDataset.value) return
  
  const dashboardName = selectedSheet.value
  const sheetName = dashboardName.replace('-dashboard', '')
  
  try {
    const safeName = sheetName.replace(/[/\\:*?"<>|]/g, '_')
    const res = await e2aService.getFile(props.agentName, selectedDataset.value, safeName + '.md')
    const mdContent = typeof res === 'string' ? res : res?.data || ''
    if (!mdContent) return
    
    const suggestion = regenerateSuggestion.value.trim()
    const prompt = `基于以下数据和用户的补充建议，重新生成一个"描述+多个 Chart"的 Dashboard。

**重要：不要创建任何文件，直接在聊天回复中返回分析内容。**

用户的重新生成建议：${suggestion || '请优化图表展示和文字描述'}

要求：
1. 每个图表使用 \`\`\`chart 代码块包裹，内容为 JSON 格式：{"type":"bar|line|pie|doughnut","title":"图表标题","labels":["标签1","标签2"],"data":[数值1,数值2]}
2. Chart 代码块放在前面，文字总结放在后面
3. 图表和总结都用中文

原始数据（Markdown）：
${mdContent.substring(0, 8000)}`

    isRegenerating.value = true
    analyzedSheet.value = sheetName
    analysisStartMsgIdx.value = props.messages.length
    analysisCharts.value = []
    analysisSummary.value = ''
    analyzeContent.value = 'loading'
    regenerateSuggestion.value = ''

    emit('send', prompt)
  } catch (e) {
    console.error('[E2A] Failed to regenerate:', e)
    isRegenerating.value = false
    analyzeContent.value = '<p style="color:red;">重新生成失败</p>'
  }
}

const loadSheetMarkdown = async (dsName, sheetName) => {
  try {
    const safeName = sheetName.replace(/[/\\:*?"<>|]/g, '_')
    const res = await e2aService.getFile(props.agentName, dsName, safeName + '.md')
    analysisCharts.value = []
    analysisSummary.value = ''
    analyzeContent.value = marked.parse(typeof res === 'string' ? res : res?.data || '')
    currentMode.value = 'analyze'
    await nextTick()
  } catch (e) {
    console.error('[E2A] Failed to load sheet md:', e)
    analyzeContent.value = '<p style="color:red;">加载失败</p>'
    currentMode.value = 'analyze'
  }
}

const analyzeSheet = async (dsName, sheetName) => {
  try {
    const safeName = sheetName.replace(/[/\\:*?"<>|]/g, '_')
    const res = await e2aService.getFile(props.agentName, dsName, safeName + '.md')
    const mdContent = typeof res === 'string' ? res : res?.data || ''
    if (!mdContent) {
      analyzeContent.value = ''
      return
    }

    const prompt = `请分析以下 Excel 表格数据「${sheetName}」，对这些数据的类型和功能进行判断，然后用自然语言进行概括和描述，并提供分析和展示的建议，形成一个"描述+多个 Chart"的 Dashboard。

**重要：不要创建任何文件，直接在聊天回复中返回分析内容。**

要求：
1. 每个图表使用 \`\`\`chart 代码块包裹，内容为 JSON 格式：{"type":"bar|line|pie|doughnut","title":"图表标题","labels":["标签1","标签2"],"data":[数值1,数值2]}
2. Chart 代码块放在前面，文字总结放在后面
3. 图表和总结都用中文

数据（Markdown）：
${mdContent.substring(0, 8000)}`

    analyzedSheet.value = sheetName
    analysisStartMsgIdx.value = props.messages.length
    analysisCharts.value = []
    analysisSummary.value = ''

    emit('send', prompt)
  } catch (e) {
    console.error('[E2A] Failed to analyze sheet:', e)
    analysisSummary.value = '<p style="color:red;">分析请求失败</p>'
    analyzeContent.value = ''
  }
}

const analyzedSheet = ref('')
const analysisStartMsgIdx = ref(0)
const analyzeContent = ref('')
const analysisCharts = ref([])
const analysisSummary = ref('')
const analyzeChartsEl = ref(null)
const chartCanvasRefs = ref({})
let chartInstances = []
const regenerateSuggestion = ref('')
const isRegenerating = ref(false)

const setChartCanvas = (idx, el) => {
  if (el) chartCanvasRefs.value[idx] = el
}

const parseAnalysisResult = (text) => {
  // Try to extract actual dashboard content from tool output
  let content = text
  const contentMatch = content.match(/<content>([\s\S]*?)<\/content>/)
  if (contentMatch) {
    content = contentMatch[1]
  }
  
  const charts = []
  // Try ```chart blocks first, then ```json blocks that look like charts
  for (const pattern of [/```chart\s*\n([\s\S]*?)```/g, /```json\s*\n([\s\S]*?)```/g]) {
    let match
    while ((match = pattern.exec(content)) !== null) {
      try {
        // Strip line number prefixes like "8: " from tool output
        const cleanJson = match[1].trim().replace(/^\d+:\s*/gm, '').trim()
        const config = JSON.parse(cleanJson)
        if (config.type && (config.data || config.datasets)) {
          charts.push(config)
        }
      } catch (e) {
        // invalid JSON, keep in text
      }
    }
    if (charts.length > 0) break
  }
  
  // Clean line numbers from text for display
  let cleanText = content
  if (contentMatch) {
    cleanText = cleanText.replace(/^\d+:\s*/gm, '')
    cleanText = cleanText.replace(/^Done:.*$/gm, '')
    cleanText = cleanText.replace(/<path>.*?<\/path>/gs, '')
    cleanText = cleanText.replace(/<type>.*?<\/type>/gs, '')
    cleanText = cleanText.replace(/<content>/g, '')
    cleanText = cleanText.replace(/<\/content>/g, '')
    cleanText = cleanText.replace(/\(End of file.*\)/g, '')
  }
  
  return { charts, text: cleanText.trim() }
}

const renderAnalysisCharts = async () => {
  // Destroy old charts
  chartInstances.forEach(c => c.destroy())
  chartInstances = []
  
  await nextTick()
  for (let i = 0; i < analysisCharts.value.length; i++) {
    const canvas = chartCanvasRefs.value[i]
    if (!canvas) continue
    const config = analysisCharts.value[i]
    const chart = new Chart(canvas, {
      type: config.type || 'bar',
      data: {
        labels: config.labels || [],
        datasets: config.datasets || [{
          label: config.label || '数据',
          data: config.data || [],
          backgroundColor: config.type === 'pie' || config.type === 'doughnut'
            ? (config.labels || []).map((_, j) => `hsl(${j * 360 / (config.labels || [1]).length}, 70%, 60%)`)
            : 'rgba(64, 149, 254, 0.6)',
          borderColor: 'rgba(64, 149, 254, 1)',
          borderWidth: 1
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { display: true, position: 'bottom' },
          title: { display: false }
        }
      }
    })
    chartInstances.push(chart)
  }
}

// Watch for new messages as analysis result
watch(() => props.messages, async (msgs) => {
  if (!analyzedSheet.value) return
  const newMsgs = msgs.slice(analysisStartMsgIdx.value)
  const assistantMsgs = newMsgs.filter(m => !m.isSent && !m.isTyping && m.text)
  if (assistantMsgs.length > 0) {
    const last = assistantMsgs[assistantMsgs.length - 1]
    const { charts, text } = parseAnalysisResult(last.text || '')
    analyzeContent.value = last.text || ''
    analysisCharts.value = charts
    analysisSummary.value = marked.parse(text)
    isRegenerating.value = false
    renderAnalysisCharts()
    
    // Save dashboard as a markdown file
    const sheetName = analyzedSheet.value
    const dsName = selectedDataset.value
    analyzedSheet.value = ''
    if (dsName && sheetName) {
      try {
        const safeName = sheetName.replace(/[/\\:*?"<>|]/g, '_')
        await e2aService.saveFile(props.agentName, dsName, safeName + '-dashboard.md', last.text || '')
        emit('refresh')
      } catch (e) {
        console.error('[E2A] Failed to save dashboard:', e)
      }
    }
  }
}, { deep: true })

const loadOverview = async (dsName) => {
  try {
    const res = await e2aService.getFile(props.agentName, dsName, 'overview.md')
    analyzeContent.value = marked.parse(typeof res === 'string' ? res : res?.data || '')
    await nextTick()
  } catch (e) {
    console.error('[E2A] Failed to load overview:', e)
    analyzeContent.value = '<p style="color:red;">加载失败</p>'
  }
}

const loadCSV = async (dsName, sheetName) => {
  try {
    const safeName = sheetName.replace(/[/\\:*?"<>|]/g, '_')
    const res = await e2aService.getFile(props.agentName, dsName, safeName + '.csv')
    const csv = typeof res === 'string' ? res : res?.data || ''
    const lines = csv.split('\n').filter(l => l.trim())
    if (lines.length > 0) {
      csvHeaders.value = parseCSVLine(lines[0])
      csvRows.value = lines.slice(1).map(parseCSVLine)
    }
    displaySheetName.value = sheetName
    currentPage.value = 0
    chartData.value = null
    if (chartInstance) { chartInstance.destroy(); chartInstance = null }
  } catch (e) {
    console.error('[E2A] Failed to load CSV:', e)
    csvHeaders.value = []
    csvRows.value = []
  }
}

const parseCSVLine = (line) => {
  const result = []
  let current = ''
  let inQuotes = false
  for (let i = 0; i < line.length; i++) {
    const ch = line[i]
    if (inQuotes) {
      if (ch === '"') {
        if (i + 1 < line.length && line[i + 1] === '"') {
          current += '"'; i++
        } else {
          inQuotes = false
        }
      } else {
        current += ch
      }
    } else {
      if (ch === '"') {
        inQuotes = true
      } else if (ch === ',') {
        result.push(current); current = ''
      } else {
        current += ch
      }
    }
  }
  result.push(current)
  return result
}

const loadFormulaContent = async () => {
  if (!selectedDataset.value) return
  try {
    const res = await e2aService.getFile(props.agentName, selectedDataset.value, 'formulas.md')
    formulaContent.value = marked.parse(typeof res === 'string' ? res : res?.data || '')
    await nextTick()
  } catch (e) {
    console.error('[E2A] Failed to load formulas:', e)
    formulaContent.value = '<p style="color:red;">加载失败</p>'
  }
}

const deleteDataset = async (dsName) => {
  if (!confirm(`确定要删除数据集 "${dsName}" 吗？`)) return
  try {
    await e2aService.deleteDataset(props.agentName, dsName)
    if (selectedDataset.value === dsName) {
      selectedDataset.value = null
      selectedSheet.value = null
      csvHeaders.value = []
      csvRows.value = []
      analyzeContent.value = ''
      formulaContent.value = ''
    }
    if (expandedDataset.value === dsName) expandedDataset.value = null
    emit('refresh')
  } catch (e) {
    console.error('[E2A] Failed to delete dataset:', e)
  }
}

const sortedRows = computed(() => {
  if (sortColumn.value < 0) return csvRows.value
  const rows = [...csvRows.value]
  const col = sortColumn.value
  rows.sort((a, b) => {
    const va = a[col] || ''
    const vb = b[col] || ''
    const na = parseFloat(va), nb = parseFloat(vb)
    if (!isNaN(na) && !isNaN(nb)) {
      return sortAsc.value ? na - nb : nb - na
    }
    return sortAsc.value ? va.localeCompare(vb) : vb.localeCompare(va)
  })
  return rows
})

const pagedRows = computed(() => {
  const start = currentPage.value * pageSize.value
  return sortedRows.value.slice(start, start + pageSize.value)
})

const totalPages = computed(() => Math.ceil(csvRows.value.length / pageSize.value) || 1)

const sortByColumn = (col) => {
  if (sortColumn.value === col) {
    sortAsc.value = !sortAsc.value
  } else {
    sortColumn.value = col
    sortAsc.value = true
  }
}

const chartCanvas = ref(null)

const generateChart = () => {
  if (!chartType.value || csvHeaders.value.length < 2 || csvRows.value.length === 0) return
  const labelCol = 0
  const dataCol = 1
  const labels = csvRows.value.map(r => r[labelCol] || '').slice(0, 20)
  const values = csvRows.value.map(r => parseFloat(r[dataCol]) || 0).slice(0, 20)

  if (chartInstance) { chartInstance.destroy(); chartInstance = null }

  nextTick(() => {
    if (!chartCanvas.value) return
    chartInstance = new Chart(chartCanvas.value, {
      type: chartType.value,
      data: {
        labels,
        datasets: [{
          label: csvHeaders.value[dataCol] || '数据',
          data: values,
          backgroundColor: chartType.value === 'pie'
            ? labels.map((_, i) => `hsl(${i * 360 / labels.length}, 70%, 60%)`)
            : 'rgba(64, 149, 254, 0.6)',
          borderColor: 'rgba(64, 149, 254, 1)',
          borderWidth: 1
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: { legend: { display: true, position: 'bottom' } }
      }
    })
    chartData.value = true
  })
}

const toggleExpanded = () => { emit('toggle') }
const onRefresh = () => { emit('refresh') }
const toggleFullscreen = () => { emit('toggleFullscreen') }

const panelHeight = ref(props.initialHeight)
watch(() => props.initialHeight, (newH) => { panelHeight.value = newH })
const isResizing = ref(false)
let startY = 0, startH = 0

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
  if (h < 60) h = 60
  if (h > 500) h = 500
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

onMounted(() => {
  console.log('[E2APanel] mounted, height:', panelHeight.value)
})

onUnmounted(() => {
  if (chartInstance) { chartInstance.destroy(); chartInstance = null }
  chartInstances.forEach(c => c.destroy())
  chartInstances = []
})
</script>

<style scoped>
.e2a-panel {
  flex-shrink: 0;
  position: relative;
  background: var(--bg-panel, #e8ecf6);
  border-bottom: 1px solid var(--border-subtle, rgba(0, 0, 0, 0.07));
  display: flex;
  flex-direction: column;
  box-shadow: -4px 0 8px rgba(0, 0, 0, 0.05), 0 4px 12px rgba(0, 0, 0, 0.08);
  z-index: 5;
}

.e2a-panel.fullscreen {
  position: fixed;
  top: var(--header-height, 56px);
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
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

.e2a-panel-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 16px;
  cursor: pointer;
  user-select: none;
  background: var(--bg-panel, #e8ecf6);
  flex-wrap: wrap;
}

.e2a-panel-icon { font-size: 14px; }
.e2a-panel-title {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary, #4d4d4d);
  margin-right: 4px;
}

/* Mode buttons */
.mode-btn {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  transition: all 0.2s;
  border: 1px solid transparent;
  background: transparent;
}

.analyze-btn { color: #4095fe; }
.analyze-btn:hover { background: rgba(64, 149, 254, 0.1); }
.analyze-btn.active { background: rgba(64, 149, 254, 0.15); border-color: rgba(64, 149, 254, 0.4); }

.display-btn { color: #2eb67d; }
.display-btn:hover { background: rgba(46, 182, 125, 0.1); }
.display-btn.active { background: rgba(46, 182, 125, 0.15); border-color: rgba(46, 182, 125, 0.4); }

.formula-btn { color: #e01e5a; }
.formula-btn:hover { background: rgba(224, 30, 90, 0.1); }
.formula-btn.active { background: rgba(224, 30, 90, 0.15); border-color: rgba(224, 30, 90, 0.4); }

.fill-btn { color: #f5a623; }
.fill-btn:hover { background: rgba(245, 166, 35, 0.1); }
.fill-btn.active { background: rgba(245, 166, 35, 0.15); border-color: rgba(245, 166, 35, 0.4); }

.mode-btn-icon { font-size: 11px; }
.mode-btn-text { font-size: 11px; }

.e2a-panel-actions {
  display: flex;
  gap: 4px;
  margin-left: auto;
}

.e2a-fullscreen-btn {
  width: 22px; height: 22px;
  border-radius: 4px; border: none;
  background: transparent;
  color: var(--text-dim, #797979);
  cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  font-size: 12px;
  transition: all 0.15s;
}

.e2a-fullscreen-btn:hover,
.e2a-fullscreen-btn.active {
  background: rgba(64, 149, 254, 0.1);
  color: #4095fe;
}

.refresh-btn {
  width: 22px; height: 22px;
  border-radius: 4px; border: none;
  background: transparent;
  color: var(--text-dim, #797979);
  cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  transition: all 0.15s;
}

.refresh-btn:hover { background: rgba(64, 149, 254, 0.1); color: #4095fe; }
.refresh-btn.spinning svg { animation: spin 1s linear infinite; }

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.e2a-panel-body {
  flex: 1;
  overflow: hidden;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.e2a-layout {
  display: flex;
  flex-direction: row;
  flex: 1;
  overflow: hidden;
}

/* Left: source pane */
.e2a-source-pane {
  width: 220px;
  min-width: 160px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid rgba(0, 0, 0, 0.08);
  overflow: hidden;
}

.e2a-upload-zone {
  padding: 12px 8px;
  cursor: pointer;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  background: rgba(255, 255, 255, 0.4);
  transition: background 0.2s;
}

.e2a-upload-zone:hover { background: rgba(64, 149, 254, 0.06); }
.e2a-upload-zone.dragover { background: rgba(64, 149, 254, 0.15); }

.upload-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  text-align: center;
  pointer-events: none;
}

.upload-icon { font-size: 24px; opacity: 0.7; }
.upload-text { font-size: 12px; color: var(--text-secondary, #616061); font-weight: 500; }
.upload-hint { font-size: 10px; color: var(--text-dim, #797979); }

.upload-status {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  margin-top: 2px;
  display: flex;
  align-items: center;
  gap: 4px;
  font-weight: 500;
}

.upload-status.uploading { color: #4095fe; background: rgba(64, 149, 254, 0.1); }
.upload-status.success { color: #2eb67d; background: rgba(46, 182, 125, 0.1); }
.upload-status.error { color: #e01e5a; background: rgba(224, 30, 90, 0.1); }

.upload-spinner {
  display: inline-block;
  width: 10px; height: 10px;
  border: 2px solid rgba(64, 149, 254, 0.3);
  border-top-color: #4095fe;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.e2a-dataset-list {
  flex: 1;
  overflow-y: auto;
}

.e2a-empty {
  padding: 16px;
  text-align: center;
  color: var(--text-dim, #797979);
  font-size: 13px;
}

.dataset-item { border-bottom: 1px solid rgba(0, 0, 0, 0.04); }

.dataset-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  cursor: pointer;
  font-size: 13px;
  transition: background 0.15s;
}

.dataset-header:hover { background: rgba(0, 0, 0, 0.03); }
.dataset-header.selected { background: rgba(64, 149, 254, 0.08); }

.dataset-arrow { font-size: 10px; color: var(--text-dim); width: 12px; }
.dataset-icon { font-size: 12px; }
.dataset-name { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--text-primary); font-weight: 500; }

.dataset-delete {
  width: 18px; height: 18px;
  border: none; background: transparent;
  color: var(--text-dim); cursor: pointer;
  font-size: 14px; border-radius: 3px;
  display: flex; align-items: center; justify-content: center;
}

.dataset-delete:hover { background: rgba(224, 30, 90, 0.1); color: #e01e5a; }

.dataset-sheets { padding-left: 18px; }

.sheet-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  cursor: pointer;
  font-size: 12px;
  transition: background 0.15s;
}

.sheet-item:hover { background: rgba(0, 0, 0, 0.03); }
.sheet-item.selected { background: rgba(64, 149, 254, 0.1); color: #4095fe; }
.sheet-item.special { border-top: 1px dashed rgba(0, 0, 0, 0.06); margin-top: 2px; padding-top: 5px; }
.sheet-item.dashboard-item { border-top: 1px dashed rgba(64, 149, 254, 0.2); margin-top: 2px; padding-top: 5px; }
.sheet-item.dashboard-item .sheet-icon { color: #4095fe; }

.sheet-icon { font-size: 11px; }
.sheet-name { flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.sheet-info { font-size: 10px; color: var(--text-dim); flex-shrink: 0; }

/* Right: display pane */
.e2a-display-pane {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  background: #ffffff;
  min-width: 0;
}

/* Right: regenerate panel */
.e2a-regenerate-pane {
  width: 200px;
  min-width: 160px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
  border-left: 1px solid rgba(0, 0, 0, 0.08);
  background: #fafafa;
}

.regenerate-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.regenerate-input {
  height: 80px;
  min-height: 60px;
  max-height: 120px;
  padding: 8px;
  font-size: 12px;
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: 4px;
  resize: vertical;
  outline: none;
  font-family: inherit;
}

.regenerate-input:focus {
  border-color: #4095fe;
}

.regenerate-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 500;
  border: none;
  border-radius: 4px;
  background: linear-gradient(135deg, #4095fe, #667eea);
  color: #fff;
  cursor: pointer;
  transition: all 0.15s;
}

.regenerate-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.regenerate-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.display-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-dim, #797979);
}

.empty-icon { font-size: 40px; opacity: 0.4; }
.empty-text { font-size: 14px; }
.empty-hint { font-size: 12px; opacity: 0.6; }

/* Markdown rendering */
.markdown-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  font-size: 13px;
  line-height: 1.6;
  color: #333;
  min-height: 0;
}

.markdown-body :deep(h1) { font-size: 1.4em; margin: 12px 0 8px; }
.markdown-body :deep(h2) { font-size: 1.2em; margin: 10px 0 6px; }
.markdown-body :deep(h3) { font-size: 1.1em; margin: 8px 0 4px; }
.markdown-body :deep(table) { width: 100%; border-collapse: collapse; margin: 8px 0; font-size: 12px; }
.markdown-body :deep(th), .markdown-body :deep(td) { border: 1px solid #ddd; padding: 6px 8px; text-align: left; }
.markdown-body :deep(th) { background: #f5f5f5; font-weight: 600; }
.markdown-body :deep(code) { background: #f0f0f0; padding: 2px 5px; border-radius: 3px; font-size: 0.9em; }
.markdown-body :deep(blockquote) { border-left: 3px solid #4095fe; padding-left: 10px; color: #666; margin: 8px 0; }

/* Data table */
.display-analyze,
.display-formula,
.display-fill,
.display-table {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Analyze split layout */
.analyze-charts {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  padding: 12px;
  align-content: flex-start;
}

.analysis-chart-card {
  flex: 1 1 calc(50% - 6px);
  min-width: 250px;
  min-height: 260px;
  max-height: 360px;
  background: #fafafa;
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 6px;
  padding: 8px;
  display: flex;
  flex-direction: column;
}

.analysis-chart-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
  flex-shrink: 0;
}

.analysis-chart-canvas {
  flex: 1;
  min-height: 0;
  max-height: 300px;
}

.analyze-divider {
  height: 2px;
  background: rgba(0, 0, 0, 0.06);
  margin: 0 12px;
  flex-shrink: 0;
}

.analyze-summary {
  flex: 0 1 40%;
  min-height: 80px;
  overflow-y: auto !important;
  padding: 12px 16px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
  font-size: 13px;
  line-height: 1.6;
  color: #333;
}

.analyze-loading {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px;
  color: var(--text-dim);
  font-size: 13px;
}

.table-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
  font-size: 12px;
}

.toolbar-info { color: var(--text-secondary); }
.toolbar-actions { display: flex; gap: 6px; align-items: center; }
.chart-select { font-size: 12px; padding: 2px 6px; border-radius: 4px; border: 1px solid #ddd; }
.chart-gen-btn { font-size: 11px; padding: 2px 8px; border-radius: 4px; border: 1px solid #4095fe; background: rgba(64, 149, 254, 0.1); color: #4095fe; cursor: pointer; }
.chart-gen-btn:hover { background: #4095fe; color: #fff; }

.chart-canvas { max-height: 250px; margin: 8px 12px; border: 1px solid rgba(0, 0, 0, 0.06); border-radius: 4px; }

.data-table-wrap {
  flex: 1;
  overflow: auto;
  padding: 0 12px 8px;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

.data-table th {
  position: sticky;
  top: 0;
  background: #f5f5f5;
  padding: 5px 8px;
  border: 1px solid #ddd;
  font-weight: 600;
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
}

.data-table th:hover { background: #e8e8e8; }
.sort-indicator { font-size: 10px; margin-left: 2px; color: #4095fe; }

.data-table td {
  padding: 4px 8px;
  border: 1px solid #eee;
  max-width: 300px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 8px;
  font-size: 12px;
}

.pagination button {
  padding: 2px 10px;
  border-radius: 4px;
  border: 1px solid #ddd;
  background: #fff;
  cursor: pointer;
  font-size: 12px;
}

.pagination button:hover:not(:disabled) { background: #f0f0f0; }
.pagination button:disabled { opacity: 0.4; cursor: default; }
.page-size-label { margin-left: 8px; color: var(--text-dim); }
.pagination select { font-size: 12px; padding: 2px 4px; border-radius: 4px; border: 1px solid #ddd; }

@media (max-width: 768px) {
  .e2a-panel-header { padding: 6px 12px; }
  .e2a-source-pane { width: 160px; }
}
</style>
