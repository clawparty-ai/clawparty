<template>
  <div class="task-kanban">
    <div class="kanban-header">
      <span class="kanban-title">📊 {{ kanbanConfig.name || '任务看板' }}</span>
      <div class="kanban-actions">
        <button class="action-btn" @click="openFullscreen" title="全屏">
          ⛶ 全屏
        </button>
        <button class="close-btn" @click="$emit('close')">×</button>
      </div>
    </div>
    <div class="kanban-body">
      <!-- Stats Cards -->
      <div class="stats-cards">
        <div class="stat-card">
          <div class="stat-value">{{ stats.total }}</div>
          <div class="stat-label">总任务</div>
        </div>
        <div class="stat-card running">
          <div class="stat-value">{{ stats.running }}</div>
          <div class="stat-label">执行中</div>
        </div>
        <div class="stat-card completed">
          <div class="stat-value">{{ stats.completed }}</div>
          <div class="stat-label">已完成</div>
        </div>
        <div class="stat-card failed">
          <div class="stat-value">{{ stats.failed }}</div>
          <div class="stat-label">失败</div>
        </div>
      </div>

      <!-- Charts Grid -->
      <div class="charts-container">
        <div 
          v-for="chart in enabledCharts" 
          :key="chart.id"
          class="chart-item"
          :class="'chart-' + chart.type"
        >
          <div class="chart-header">
            <span class="chart-title">{{ chart.title }}</span>
          </div>
          <div class="chart-wrap">
            <canvas :ref="el => setChartRef(el, chart.id)"></canvas>
          </div>
        </div>
      </div>

      <!-- Timeout Tasks -->
      <div class="timeout-section" v-if="timeoutTasks.length > 0">
        <div class="section-title">⏰ 超时任务 ({{ timeoutTasks.length }})</div>
        <div class="timeout-list">
          <div v-for="task in timeoutTasks" :key="task.task_id" class="timeout-item">
            <span class="timeout-dot"></span>
            <span class="task-title">{{ task.short_title || task.title }}</span>
            <span class="task-time">{{ formatDuration(task) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Fullscreen Editor Overlay -->
  <Teleport to="body">
    <div v-if="isFullscreen" class="fullscreen-kanban" @keyup.escape="closeFullscreen" tabindex="-1">
      <div class="fullscreen-header">
        <span class="fullscreen-title">📊 {{ kanbanConfig.name || '任务看板' }}</span>
        <div class="fullscreen-actions">
          <button 
            class="mode-toggle" 
            :class="{ active: isEditMode }"
            @click="isEditMode = !isEditMode"
          >
            {{ isEditMode ? '👁 预览' : '✏️ 编辑' }}
          </button>
          <button class="fullscreen-close-btn" @click="closeFullscreen">✕ 退出全屏</button>
        </div>
      </div>

      <div class="fullscreen-body">
        <!-- Edit Mode Sidebar -->
        <div v-if="isEditMode" class="editor-sidebar">
          <div class="editor-section">
            <div class="editor-section-title">看板设置</div>
            <div class="editor-field">
              <label>看板名称</label>
              <input v-model="editableConfig.name" type="text" placeholder="看板名称">
            </div>
            <div class="editor-field">
              <label>看板提示词</label>
              <textarea v-model="editableConfig.prompt" rows="3" placeholder="描述看板用途的提示词..."></textarea>
            </div>
          </div>

          <div class="editor-section">
            <div class="editor-section-title">图表配置</div>
            <div v-for="(chart, idx) in editableConfig.charts" :key="chart.id" class="chart-config-item">
              <div class="chart-config-header">
                <input v-model="chart.title" type="text" class="chart-title-input" placeholder="图表标题">
                <button class="remove-chart-btn" @click="removeChart(idx)" title="删除">🗑</button>
              </div>
              <div class="chart-config-fields">
                <select v-model="chart.type" class="chart-type-select">
                  <option value="doughnut">环形图</option>
                  <option value="pie">饼图</option>
                  <option value="line">折线图</option>
                  <option value="bar">柱状图</option>
                  <option value="card">数字卡片</option>
                  <option value="gauge">仪表盘</option>
                </select>
                <div class="chart-field">
                  <label>图表提示词</label>
                  <textarea v-model="chart.prompt" rows="2" placeholder="描述此图表的AI提示词..."></textarea>
                </div>
              </div>
            </div>
            
            <button class="add-chart-btn" @click="addChart">+ 添加图表</button>
          </div>

          <div class="editor-actions">
            <button class="save-btn" @click="saveConfig" :disabled="saving">
              {{ saving ? '保存中...' : '💾 保存配置' }}
            </button>
          </div>
        </div>

        <!-- Preview Area -->
        <div class="preview-area" :class="{ 'with-sidebar': isEditMode }">
          <div class="stats-cards">
            <div class="stat-card">
              <div class="stat-value">{{ stats.total }}</div>
              <div class="stat-label">总任务</div>
            </div>
            <div class="stat-card running">
              <div class="stat-value">{{ stats.running }}</div>
              <div class="stat-label">执行中</div>
            </div>
            <div class="stat-card completed">
              <div class="stat-value">{{ stats.completed }}</div>
              <div class="stat-label">已完成</div>
            </div>
            <div class="stat-card failed">
              <div class="stat-value">{{ stats.failed }}</div>
              <div class="stat-label">失败</div>
            </div>
          </div>

          <div class="fullscreen-charts-container">
            <div 
              v-for="chart in enabledCharts" 
              :key="chart.id"
              class="chart-item"
              :class="'chart-' + chart.type"
            >
              <div class="chart-header">
                <span class="chart-title">{{ chart.title }}</span>
                <button v-if="chart.prompt" class="ai-generate-btn" @click="generateChartAI(chart)" title="AI生成">
                  🤖 AI
                </button>
              </div>
              <div class="chart-wrap">
                <canvas :ref="el => setFullscreenChartRef(el, chart.id)"></canvas>
              </div>
            </div>
          </div>

          <div class="timeout-section" v-if="timeoutTasks.length > 0">
            <div class="section-title">⏰ 超时任务 ({{ timeoutTasks.length }})</div>
            <div class="timeout-list">
              <div v-for="task in timeoutTasks" :key="task.task_id" class="timeout-item">
                <span class="timeout-dot"></span>
                <span class="task-title">{{ task.short_title || task.title }}</span>
                <span class="task-time">{{ formatDuration(task) }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { Chart, DoughnutController, PieController, LineController, BarController, ArcElement, LineElement, BarElement, PointElement, CategoryScale, LinearScale, Tooltip, Legend, Filler } from 'chart.js'

Chart.register(DoughnutController, PieController, LineController, BarController, ArcElement, LineElement, BarElement, PointElement, CategoryScale, LinearScale, Tooltip, Legend, Filler)

const props = defineProps({
  tasks: {
    type: Array,
    default: () => []
  },
  agentName: {
    type: String,
    default: ''
  },
  kanbanConfig: {
    type: Object,
    default: () => ({
      name: '默认看板',
      prompt: '',
      config: {
        charts: [
          { id: 'status', type: 'doughnut', title: '状态分布', enabled: true, prompt: '' },
          { id: 'trend', type: 'line', title: '近7天趋势', enabled: true, prompt: '' },
          { id: 'agent', type: 'bar', title: 'Agent分布', enabled: true, prompt: '' },
          { id: 'duration', type: 'bar', title: '耗时统计', enabled: true, prompt: '' }
        ]
      }
    })
  }
})

const emit = defineEmits(['close', 'update:kanbanConfig', 'generateChart'])

// Refs
const chartRefs = ref({})
const fullscreenChartRefs = ref({})
const chartInstances = ref({})
const fullscreenChartInstances = ref({})
const isFullscreen = ref(false)
const isEditMode = ref(false)
const saving = ref(false)
const editableConfig = ref({
  name: '',
  prompt: '',
  charts: []
})

const TIMEOUT_THRESHOLD = 30 * 60 * 1000

// Computed
const stats = computed(() => {
  let total = 0, pending = 0, running = 0, completed = 0, failed = 0
  function count(taskList) {
    for (let i = 0; i < taskList.length; i++) {
      const task = taskList[i]
      total++
      if (task.status === 'pending') pending++
      else if (task.status === 'running') running++
      else if (task.status === 'completed') completed++
      else if (task.status === 'failed') failed++
      if (task.subtasks && task.subtasks.length > 0) {
        count(task.subtasks)
      }
    }
  }
  count(props.tasks)
  return { total, pending, running, completed, failed }
})

const flattenedTasks = computed(() => {
  const result = []
  function flatten(taskList, depth) {
    for (let i = 0; i < taskList.length; i++) {
      const task = taskList[i]
      result.push({ ...task, depth })
      if (task.subtasks && task.subtasks.length > 0) {
        flatten(task.subtasks, depth + 1)
      }
    }
  }
  flatten(props.tasks, 0)
  return result
})

const timeoutTasks = computed(() => {
  const now = Date.now()
  return flattenedTasks.value.filter(task => {
    if (task.status !== 'running') return false
    const startTime = (task.started_at || task.created_at) * 1000
    return (now - startTime) > TIMEOUT_THRESHOLD
  }).sort((a, b) => {
    const aTime = (a.started_at || a.created_at) * 1000
    const bTime = (b.started_at || b.created_at) * 1000
    return aTime - bTime
  })
})

const enabledCharts = computed(() => {
  const config = props.kanbanConfig?.config || { charts: [] }
  return (config.charts || []).filter(c => c.enabled !== false)
})

// Methods
const setChartRef = (el, id) => {
  if (el) chartRefs.value[id] = el
}

const setFullscreenChartRef = (el, id) => {
  if (el) fullscreenChartRefs.value[id] = el
}

const formatDuration = (task) => {
  if (!task.created_at) return ''
  const start = task.started_at ? task.started_at * 1000 : task.created_at * 1000
  const end = Date.now()
  const ms = end - start
  if (ms < 0) return ''
  const seconds = Math.floor(ms / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)
  if (days > 0) return days + '天' + (hours % 24) + '小时'
  if (hours > 0) return hours + '小时' + (minutes % 60) + '分'
  if (minutes > 0) return minutes + '分' + (seconds % 60) + '秒'
  return seconds + '秒'
}

// Chart rendering
const renderChart = (canvas, chartDef, isFullscreen) => {
  if (!canvas) return
  
  const ctx = canvas.getContext('2d')
  const type = chartDef.type
  
  // Destroy existing chart
  const instances = isFullscreen ? fullscreenChartInstances : chartInstances
  if (instances.value[chartDef.id]) {
    instances.value[chartDef.id].destroy()
  }
  
  let chart = null
  
  if (type === 'doughnut' || type === 'pie') {
    chart = new Chart(ctx, {
      type: type,
      data: {
        labels: ['待办', '执行中', '完成', '失败'],
        datasets: [{
          data: [stats.value.pending, stats.value.running, stats.value.completed, stats.value.failed],
          backgroundColor: ['#9e9e9e', '#4095fe', '#2eb67d', '#e01e5a'],
          borderWidth: 2,
          borderColor: '#ffffff'
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        cutout: type === 'doughnut' ? '60%' : 0,
        plugins: {
          legend: {
            position: 'bottom',
            labels: { font: { size: 10 }, boxWidth: 10, padding: 4 }
          }
        }
      }
    })
  } else if (type === 'line') {
    const days = []
    const created = []
    const completed = []
    const now = new Date()
    for (let i = 6; i >= 0; i--) {
      const date = new Date(now)
      date.setDate(date.getDate() - i)
      const dayStart = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime() / 1000
      const dayEnd = dayStart + 86400
      const mm = (date.getMonth() + 1).toString().padStart(2, '0')
      const dd = date.getDate().toString().padStart(2, '0')
      days.push(mm + '/' + dd)
      let createdCount = 0
      let completedCount = 0
      for (let j = 0; j < flattenedTasks.value.length; j++) {
        const task = flattenedTasks.value[j]
        if (task.created_at >= dayStart && task.created_at < dayEnd) createdCount++
        if (task.completed_at && task.completed_at >= dayStart && task.completed_at < dayEnd) completedCount++
      }
      created.push(createdCount)
      completed.push(completedCount)
    }
    chart = new Chart(ctx, {
      type: 'line',
      data: {
        labels: days,
        datasets: [
          {
            label: '创建',
            data: created,
            borderColor: '#4095fe',
            backgroundColor: 'rgba(64, 149, 254, 0.15)',
            fill: true,
            tension: 0.4,
            pointRadius: 3
          },
          {
            label: '完成',
            data: completed,
            borderColor: '#2eb67d',
            backgroundColor: 'rgba(46, 182, 125, 0.15)',
            fill: true,
            tension: 0.4,
            pointRadius: 3
          }
        ]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { position: 'bottom', labels: { font: { size: 10 }, boxWidth: 10, padding: 4 } }
        }
      }
    })
  } else if (type === 'bar') {
    if (chartDef.id === 'agent') {
      // Agent distribution
      const agentMap = {}
      for (let i = 0; i < flattenedTasks.value.length; i++) {
        const agent = flattenedTasks.value[i].agent_name || 'unknown'
        agentMap[agent] = (agentMap[agent] || 0) + 1
      }
      const names = Object.keys(agentMap).map(v => v.length > 8 ? v.substring(0, 8) + '..' : v)
      const values = Object.values(agentMap)
      const colors = ['#4095fe', '#2eb67d', '#f5a623', '#e01e5a', '#9b59b6', '#1abc9c']
      chart = new Chart(ctx, {
        type: 'bar',
        data: {
          labels: names,
          datasets: [{
            data: values,
            backgroundColor: names.map((_, i) => colors[i % colors.length]),
            barThickness: 14
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          indexAxis: 'y',
          plugins: { legend: { display: false } }
        }
      })
    } else if (chartDef.id === 'duration') {
      // Duration stats
      const durations = []
      for (let i = 0; i < flattenedTasks.value.length; i++) {
        const task = flattenedTasks.value[i]
        if (task.status === 'completed' && task.completed_at && task.started_at) {
          const duration = (task.completed_at - task.started_at) / 60
          if (duration > 0) {
            durations.push({
              title: task.short_title || task.title || '任务',
              duration: duration
            })
          }
        }
      }
      durations.sort((a, b) => b.duration - a.duration)
      const top5 = durations.slice(0, 5)
      const titles = top5.map(t => t.title.length > 6 ? t.title.substring(0, 6) + '...' : t.title)
      const values = top5.map(t => Math.round(t.duration * 10) / 10)
      chart = new Chart(ctx, {
        type: 'bar',
        data: {
          labels: titles,
          datasets: [{
            data: values,
            backgroundColor: '#4095fe',
            borderRadius: 4
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: { legend: { display: false } }
        }
      })
    }
  } else if (type === 'card') {
    // Number card - render as custom canvas
    ctx.clearRect(0, 0, canvas.width, canvas.height)
    ctx.font = 'bold 32px sans-serif'
    ctx.fillStyle = '#333'
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    let value = 0
    if (chartDef.id === 'status') value = stats.value.total
    else if (chartDef.id === 'completed') value = stats.value.completed
    else value = stats.value.running
    ctx.fillText(value.toString(), canvas.width / 2, canvas.height / 2 - 10)
    ctx.font = '14px sans-serif'
    ctx.fillStyle = '#666'
    ctx.fillText(chartDef.title, canvas.width / 2, canvas.height / 2 + 20)
  } else if (type === 'gauge') {
    // Gauge chart
    const percentage = stats.value.total > 0 ? Math.round((stats.value.completed / stats.value.total) * 100) : 0
    ctx.clearRect(0, 0, canvas.width, canvas.height)
    const centerX = canvas.width / 2
    const centerY = canvas.height / 2 + 10
    const radius = Math.min(centerX, centerY) - 20
    
    // Background arc
    ctx.beginPath()
    ctx.arc(centerX, centerY, radius, Math.PI, 0)
    ctx.strokeStyle = '#e0e0e0'
    ctx.lineWidth = 12
    ctx.stroke()
    
    // Value arc
    const endAngle = Math.PI + (Math.PI * percentage / 100)
    ctx.beginPath()
    ctx.arc(centerX, centerY, radius, Math.PI, endAngle)
    ctx.strokeStyle = percentage >= 80 ? '#2eb67d' : percentage >= 50 ? '#f5a623' : '#e01e5a'
    ctx.lineWidth = 12
    ctx.stroke()
    
    // Text
    ctx.font = 'bold 28px sans-serif'
    ctx.fillStyle = '#333'
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(percentage + '%', centerX, centerY - 10)
    ctx.font = '12px sans-serif'
    ctx.fillStyle = '#666'
    ctx.fillText(chartDef.title, centerX, centerY + 20)
  }
  
  if (chart) {
    instances.value[chartDef.id] = chart
  }
}

const renderAllCharts = (isFullscreen) => {
  nextTick(() => {
    const charts = enabledCharts.value
    const refs = isFullscreen ? fullscreenChartRefs.value : chartRefs.value
    for (let i = 0; i < charts.length; i++) {
      renderChart(refs[charts[i].id], charts[i], isFullscreen)
    }
  })
}

// Fullscreen
const openFullscreen = () => {
  isFullscreen.value = true
  isEditMode.value = false
  editableConfig.value = {
    name: props.kanbanConfig.name || '默认看板',
    prompt: props.kanbanConfig.prompt || '',
    charts: JSON.parse(JSON.stringify(props.kanbanConfig.config?.charts || []))
  }
  nextTick(() => {
    renderAllCharts(true)
  })
}

const closeFullscreen = () => {
  isFullscreen.value = false
  isEditMode.value = false
  // Destroy fullscreen charts
  for (const id in fullscreenChartInstances.value) {
    if (fullscreenChartInstances.value[id]) {
      fullscreenChartInstances.value[id].destroy()
    }
  }
  fullscreenChartInstances.value = {}
}

// Edit mode
const addChart = () => {
  editableConfig.value.charts.push({
    id: 'chart-' + Date.now(),
    type: 'doughnut',
    title: '新图表',
    enabled: true,
    prompt: ''
  })
}

const removeChart = (idx) => {
  editableConfig.value.charts.splice(idx, 1)
}

const saveConfig = () => {
  saving.value = true
  emit('update:kanbanConfig', {
    ...props.kanbanConfig,
    name: editableConfig.value.name,
    prompt: editableConfig.value.prompt,
    config: {
      charts: editableConfig.value.charts
    }
  })
  setTimeout(() => {
    saving.value = false
  }, 500)
}

const generateChartAI = (chart) => {
  emit('generateChart', {
    chartId: chart.id,
    prompt: chart.prompt,
    title: chart.title
  })
}

// Watch
watch(() => props.tasks, () => {
  renderAllCharts(false)
  if (isFullscreen.value) {
    renderAllCharts(true)
  }
}, { deep: true })

watch(() => props.kanbanConfig, () => {
  renderAllCharts(false)
  if (isFullscreen.value) {
    renderAllCharts(true)
  }
}, { deep: true })

// Lifecycle
onMounted(() => {
  renderAllCharts(false)
})

onBeforeUnmount(() => {
  for (const id in chartInstances.value) {
    if (chartInstances.value[id]) {
      chartInstances.value[id].destroy()
    }
  }
  for (const id in fullscreenChartInstances.value) {
    if (fullscreenChartInstances.value[id]) {
      fullscreenChartInstances.value[id].destroy()
    }
  }
})
</script>

<style scoped>
.task-kanban {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-left: 1px solid rgba(0, 0, 0, 0.08);
  background: #ffffff;
  max-width: 55%;
}

.kanban-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: #f9fafc;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
}

.kanban-title {
  font-size: 12px;
  font-weight: 600;
  color: #757575;
}

.kanban-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.action-btn {
  padding: 2px 8px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  background: #fff;
  color: #666;
  border-radius: 4px;
  cursor: pointer;
  font-size: 11px;
  transition: all 0.15s;
}

.action-btn:hover {
  background: rgba(64, 149, 254, 0.1);
  border-color: rgba(64, 149, 254, 0.3);
  color: #4095fe;
}

.close-btn {
  width: 20px;
  height: 20px;
  border: none;
  background: transparent;
  color: #999;
  cursor: pointer;
  font-size: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: all 0.15s;
}

.close-btn:hover {
  background: rgba(0, 0, 0, 0.06);
  color: #666;
}

.kanban-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 10px;
}

.stats-cards {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
  margin-bottom: 8px;
}

.stat-card {
  background: #f5f7fa;
  border-radius: 6px;
  padding: 8px 6px;
  text-align: center;
}

.stat-card.running { background: rgba(64, 149, 254, 0.08); }
.stat-card.completed { background: rgba(46, 182, 125, 0.08); }
.stat-card.failed { background: rgba(224, 30, 90, 0.08); }

.stat-value {
  font-size: 20px;
  font-weight: 700;
  color: #333;
  line-height: 1.2;
}

.stat-card.running .stat-value { color: #4095fe; }
.stat-card.completed .stat-value { color: #2eb67d; }
.stat-card.failed .stat-value { color: #e01e5a; }

.stat-label {
  font-size: 11px;
  color: #757575;
  margin-top: 2px;
}

.charts-container {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
  margin-bottom: 8px;
}

.chart-item {
  background: #f9fafc;
  border-radius: 6px;
  padding: 6px;
  border: 1px solid rgba(0, 0, 0, 0.04);
}

.chart-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
  padding-left: 4px;
}

.chart-title {
  font-size: 11px;
  font-weight: 600;
  color: #757575;
}

.ai-generate-btn {
  padding: 1px 6px;
  border: 1px solid rgba(64, 149, 254, 0.3);
  background: rgba(64, 149, 254, 0.08);
  color: #4095fe;
  border-radius: 3px;
  cursor: pointer;
  font-size: 10px;
  transition: all 0.15s;
}

.ai-generate-btn:hover {
  background: rgba(64, 149, 254, 0.15);
}

.chart-wrap {
  width: 100%;
  height: 140px;
  position: relative;
}

.chart-card .chart-wrap {
  height: 100px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.chart-gauge .chart-wrap {
  height: 160px;
}

.timeout-section {
  background: #f9fafc;
  border-radius: 6px;
  padding: 8px;
  border: 1px solid rgba(0, 0, 0, 0.04);
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: #757575;
  margin-bottom: 6px;
}

.timeout-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.timeout-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  background: rgba(224, 30, 90, 0.04);
  border-radius: 4px;
  font-size: 12px;
}

.timeout-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #e01e5a;
  flex-shrink: 0;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.timeout-item .task-title {
  flex: 1;
  color: #4d4d4d;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.timeout-item .task-time {
  color: #e01e5a;
  font-weight: 500;
  flex-shrink: 0;
  font-size: 11px;
}

/* Fullscreen Styles */
.fullscreen-kanban {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: #f5f7fa;
  z-index: 9999;
  display: flex;
  flex-direction: column;
}

.fullscreen-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  background: #ffffff;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
  flex-shrink: 0;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
}

.fullscreen-title {
  font-size: 16px;
  font-weight: 600;
  color: #333;
}

.fullscreen-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.mode-toggle {
  padding: 4px 12px;
  border: 1px solid rgba(0, 0, 0, 0.15);
  background: #fff;
  color: #666;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.15s;
}

.mode-toggle:hover {
  background: rgba(64, 149, 254, 0.08);
  border-color: rgba(64, 149, 254, 0.3);
}

.mode-toggle.active {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  border-color: transparent;
}

.fullscreen-close-btn {
  padding: 4px 12px;
  border: 1px solid rgba(224, 30, 90, 0.3);
  background: rgba(224, 30, 90, 0.08);
  color: #e01e5a;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.15s;
}

.fullscreen-close-btn:hover {
  background: #e01e5a;
  color: #fff;
}

.fullscreen-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

/* Editor Sidebar */
.editor-sidebar {
  width: 320px;
  background: #ffffff;
  border-right: 1px solid rgba(0, 0, 0, 0.08);
  overflow-y: auto;
  padding: 16px;
  flex-shrink: 0;
}

.editor-section {
  margin-bottom: 20px;
}

.editor-section-title {
  font-size: 13px;
  font-weight: 600;
  color: #333;
  margin-bottom: 10px;
  padding-bottom: 6px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
}

.editor-field {
  margin-bottom: 12px;
}

.editor-field label {
  display: block;
  font-size: 12px;
  color: #666;
  margin-bottom: 4px;
}

.editor-field input,
.editor-field textarea {
  width: 100%;
  padding: 6px 8px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 4px;
  font-size: 13px;
  box-sizing: border-box;
}

.editor-field input:focus,
.editor-field textarea:focus {
  outline: none;
  border-color: #4095fe;
}

.chart-config-item {
  background: #f9fafc;
  border-radius: 6px;
  padding: 10px;
  margin-bottom: 10px;
  border: 1px solid rgba(0, 0, 0, 0.04);
}

.chart-config-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
}

.chart-title-input {
  flex: 1;
  padding: 4px 6px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 4px;
  font-size: 12px;
}

.remove-chart-btn {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: 14px;
  border-radius: 4px;
  transition: all 0.15s;
}

.remove-chart-btn:hover {
  background: rgba(224, 30, 90, 0.1);
}

.chart-config-fields {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.chart-type-select {
  padding: 4px 6px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 4px;
  font-size: 12px;
  background: #fff;
}

.chart-field label {
  display: block;
  font-size: 11px;
  color: #666;
  margin-bottom: 3px;
}

.chart-field textarea {
  width: 100%;
  padding: 4px 6px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 4px;
  font-size: 11px;
  box-sizing: border-box;
  resize: vertical;
}

.add-chart-btn {
  width: 100%;
  padding: 8px;
  border: 1px dashed rgba(0, 0, 0, 0.15);
  background: transparent;
  color: #666;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;
}

.add-chart-btn:hover {
  border-color: #4095fe;
  color: #4095fe;
  background: rgba(64, 149, 254, 0.04);
}

.editor-actions {
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
}

.save-btn {
  width: 100%;
  padding: 8px;
  border: none;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.15s;
}

.save-btn:hover {
  opacity: 0.9;
}

.save-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Preview Area */
.preview-area {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
}

.preview-area.with-sidebar {
  padding-left: 16px;
}

.fullscreen-charts-container {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin-bottom: 12px;
}

.fullscreen-charts-container .chart-item {
  background: #ffffff;
  border-radius: 8px;
  padding: 12px;
  border: 1px solid rgba(0, 0, 0, 0.06);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
}

.fullscreen-charts-container .chart-wrap {
  height: 200px;
}

.fullscreen-charts-container .chart-card .chart-wrap {
  height: 150px;
}

.fullscreen-charts-container .chart-gauge .chart-wrap {
  height: 220px;
}

.fullscreen-charts-container .chart-title {
  font-size: 13px;
}

.fullscreen-charts-container .ai-generate-btn {
  font-size: 11px;
  padding: 2px 8px;
}

@media (max-width: 768px) {
  .fullscreen-charts-container {
    grid-template-columns: repeat(2, 1fr);
  }
  .editor-sidebar {
    width: 280px;
  }
}
</style>
