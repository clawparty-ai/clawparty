<template>
  <div class="task-panel" :style="{ height: panelHeight + 'px' }">
    <div class="task-panel-header" @click="toggleExpanded">
      <span class="task-panel-icon">🎯</span>
      <span class="task-panel-title">任务</span>
      <span class="task-panel-stats">
        <span class="stat running" v-if="taskStats.running > 0">{{ taskStats.running }} 执行中</span>
        <span class="stat completed" v-if="taskStats.completed > 0">{{ taskStats.completed }} 完成</span>
        <span class="stat pending" v-if="taskStats.pending > 0">{{ taskStats.pending }} 待办</span>
        <span class="stat pending" v-else-if="taskStats.total === 0">0 待办</span>
        <span class="stat failed" v-if="taskStats.failed > 0">{{ taskStats.failed }} 失败</span>
        <span class="stat confirm" v-if="taskStats.pendingConfirm > 0">{{ taskStats.pendingConfirm }} 待确认</span>
      </span>
      <span class="timeout-select-wrap">
        <select v-model="refreshTimeout" class="timeout-select" title="超时时间" :disabled="refreshing">
          <option :value="30">30s</option>
          <option :value="60">60s</option>
          <option :value="120">120s</option>
          <option :value="180">180s</option>
          <option :value="300">300s</option>
        </select>
      </span>
      <button class="refresh-btn" :class="{ spinning: refreshing }" @click.stop="onRefresh" title="刷新任务">
        <svg width="14" height="14" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z" clip-rule="evenodd"/>
        </svg>
      </button>
    </div>
    <div v-show="expanded" class="task-panel-container">
      <!-- Left: Task list -->
      <div class="task-list-body">
        <div v-if="flattenedTasks.length === 0" class="task-empty">
          <span class="task-empty-icon">✨</span>
          <span class="task-empty-text">暂无任务，跟我说「帮我做一件事」来开始吧</span>
        </div>
        <div
          v-for="task in flattenedTasks"
          :key="task.task_id"
          v-else
          class="task-row"
          :class="['indent-' + Math.min(task.depth, 3), 'status-' + task.status, { 'pending-confirm': task._pendingChange, 'pending-create': task._isPendingCreate }]"
        >
          <div class="task-indent-guide">
            <div v-for="d in task.depth" :key="d" class="indent-line"></div>
          </div>
          <div class="task-branch" v-if="task.depth > 0">
            <div class="branch-line"></div>
          </div>
          <div class="task-indicator">
            <span class="status-dot" :class="'status-' + task.status"></span>
          </div>
          <div class="task-content">
            <div class="task-line">
              <span class="task-created">{{ formatDate(task.created_at) }}</span>
              <span class="task-title" :title="task.ai_description || task.description">
                <span v-if="task._isPendingCreate" class="new-badge">新</span>
                {{ task.short_title || task.title }}
              </span>
              <span class="task-duration" v-if="task.created_at">{{ formatDuration(task) }}</span>
              <span class="task-priority" v-if="task.priority !== 'normal'" :class="'priority-' + task.priority">{{ formatPriority(task.priority) }}</span>
            </div>
            <div class="task-meta">
              <div class="task-progress-bar">
                <div class="task-progress-fill" :class="'fill-' + task.status" :style="{ width: task.progress + '%' }"></div>
              </div>
              <span class="task-progress-text">{{ task.progress }}%</span>
            </div>
            <div v-if="task._pendingChange" class="pending-reason">
              <span class="reason-text">{{ task._pendingChange.reason }}</span>
              <button class="confirm-btn" @click.stop="confirmChange(task._pendingChange)">✓ 确认</button>
            </div>
          </div>
        </div>
      </div>

      <!-- Right: Refresh log panel -->
      <div v-if="refreshLogs.length > 0" class="log-panel">
        <div class="log-header">🪵 任务分析</div>
        <div class="log-body" ref="logBodyRef">
          <div
            v-for="(log, idx) in refreshLogs"
            :key="idx"
            class="log-line"
            :class="'log-' + log.level"
          >
            <span v-if="log.time" class="log-time">{{ log.time }}</span>
            <span class="log-msg">{{ log.msg }}</span>
          </div>
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
import { computed, ref, watch, nextTick } from 'vue'

const props = defineProps({
  agentName: {
    type: String,
    required: true
  },
  tasks: {
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
  pendingChanges: {
    type: Array,
    default: () => []
  },
  refreshLogs: {
    type: Array,
    default: () => []
  }
})

const emit = defineEmits(['toggle', 'refresh', 'confirmChange', 'refreshTimeoutChange'])

const logBodyRef = ref(null)
const refreshTimeout = ref(120)

watch(() => refreshTimeout.value, (newVal) => {
  emit('refreshTimeoutChange', newVal)
})

watch(() => props.refreshLogs.length, () => {
  nextTick(() => {
    const el = logBodyRef.value
    if (el) el.scrollTop = el.scrollHeight
  })
})

const toggleExpanded = () => {
  emit('toggle')
}

const onRefresh = () => {
  emit('refresh')
}

const isPendingConfirm = (taskId) => {
  for (const c of props.pendingChanges) {
    if (c.taskId === taskId) return c
  }
  return null
}

const confirmChange = (change) => {
  emit('confirmChange', change)
}

const formatPriority = (priority) => {
  const map = { low: '低', normal: '中', high: '高', urgent: '紧急' }
  return map[priority] || priority
}

const formatDate = (ts) => {
  if (!ts) return ''
  const d = new Date(ts * 1000)
  const mm = (d.getMonth() + 1).toString().padStart(2, '0')
  const dd = d.getDate().toString().padStart(2, '0')
  const hh = d.getHours().toString().padStart(2, '0')
  const min = d.getMinutes().toString().padStart(2, '0')
  return mm + '-' + dd + ' ' + hh + ':' + min
}

const formatDuration = (task) => {
  if (!task.created_at) return ''
  const start = task.started_at ? task.started_at * 1000 : task.created_at * 1000
  const end = task.completed_at ? task.completed_at * 1000 : (task.status === 'running' ? Date.now() : 0)
  if (!end) return ''
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

const flattenedTasks = computed(() => {
  function cloneWithPending(taskList) {
    const list = []
    for (let i = 0; i < taskList.length; i++) {
      const task = { ...taskList[i] }
      const pendingChange = isPendingConfirm(task.task_id)
      if (pendingChange) task._pendingChange = pendingChange
      if (task.subtasks && task.subtasks.length > 0) {
        task.subtasks = cloneWithPending(task.subtasks)
      }
      list.push(task)
    }
    return list
  }

  const cloned = cloneWithPending(props.tasks)
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
  flatten(cloned, 0)

  for (const c of props.pendingChanges) {
    if (c.type === 'create') {
      result.push({
        task_id: c.taskId,
        title: c.data.title,
        short_title: null,
        description: c.data.description,
        status: c.data.status || 'pending',
        progress: c.data.progress !== undefined ? c.data.progress : 0,
        priority: 'normal',
        depth: 0,
        _isPendingCreate: true,
        _pendingChange: c,
      })
    }
  }

  return result
})

const taskStats = computed(() => {
  let total = 0
  let pending = 0
  let running = 0
  let completed = 0
  let failed = 0

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
  return { total, pending, running, completed, failed, pendingConfirm: props.pendingChanges.length }
})
</script>

<style scoped>
.task-panel {
  flex-shrink: 0;
  position: relative;
  background: var(--bg-panel, #e8ecf6);
  border-bottom: 1px solid var(--border-subtle, rgba(0, 0, 0, 0.07));
  display: flex;
  flex-direction: column;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
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

.task-panel-header {
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

.task-panel-header:hover {
  background: rgba(0, 0, 0, 0.02);
}

.task-panel-icon {
  font-size: 14px;
}

.task-panel-title {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary, #4d4d4d);
}

.task-panel-stats {
  display: flex;
  gap: 8px;
  margin-left: auto;
}

.stat {
  font-size: 12px;
  padding: 1px 6px;
  border-radius: 10px;
  font-weight: 500;
}

.stat.running { background: rgba(64, 149, 254, 0.12); color: #4095fe; }
.stat.completed { background: rgba(46, 182, 125, 0.12); color: #2eb67d; }
.stat.pending { background: rgba(158, 158, 158, 0.12); color: #757575; }
.stat.failed { background: rgba(224, 30, 90, 0.12); color: #e01e5a; }

.task-panel-container {
  display: flex;
  flex: 1;
  flex-direction: row;
  overflow: hidden;
  min-height: 0;
}

.task-list-body {
  flex: 1;
  overflow-y: auto;
  padding: 4px 16px 10px;
  min-height: 0;
}

.log-panel {
  flex: 1; 
  overflow: hidden;
  display: flex;
  flex-direction: column;
  border-left: 1px solid rgba(0, 0, 0, 0.08);
  background: #ffffff;
  max-width: 50%;
}

.log-header {
  font-size: 11px;
  font-weight: 600;
  color: #757575;
  padding: 6px 10px;
  background: #f9fafc;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
}

.log-body {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px;
  font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  font-size: 11px;
  line-height: 1.5;
}

.log-line {
  white-space: pre-wrap;
  word-break: break-word;
  margin-bottom: 1px;
}

.log-time {
  color: #b0b0b0;
  margin-right: 6px;
}

.log-info  { color: #337ab7; }
.log-warn  { color: #f0ad4e; }
.log-error { color: #d9534f; }
.log-debug { color: #888888; }

.task-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 16px 12px;
  gap: 6px;
  text-align: center;
}

.task-empty-icon {
  font-size: 20px;
  opacity: 0.7;
}

.task-empty-text {
  font-size: 15px;
  color: var(--text-dim, #797979);
  line-height: 1.5;
}

.task-row {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  padding: 4px 0;
  position: relative;
}

.task-indent-guide {
  display: flex;
  flex-shrink: 0;
}

.indent-line {
  width: 16px;
  border-right: 1px solid rgba(0, 0, 0, 0.08);
  height: 100%;
  min-height: 28px;
}

.task-branch {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.branch-line {
  width: 12px;
  height: 1px;
  background: rgba(0, 0, 0, 0.1);
  margin-top: 6px;
}

.task-indicator {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  padding-top: 4px;
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  display: inline-block;
}

.status-dot.status-pending { background: #9e9e9e; }
.status-dot.status-running { background: #4095fe; animation: pulse 2s infinite; }
.status-dot.status-completed { background: #2eb67d; }
.status-dot.status-failed { background: #e01e5a; }

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.task-content {
  flex: 1;
  min-width: 0;
}

.task-line {
  display: flex;
  align-items: center;
  gap: 6px;
}

.task-created {
  font-size: 12px;
  color: var(--text-muted, #999);
  flex-shrink: 0;
}

.task-duration {
  font-size: 12px;
  color: var(--text-muted, #999);
  flex-shrink: 0;
  margin-left: auto;
}

.task-title {
  font-size: 15px;
  color: var(--text-primary, #4d4d4d);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 500;
}

.task-priority {
  font-size: 12px;
  padding: 0 4px;
  border-radius: 3px;
  font-weight: 500;
  flex-shrink: 0;
}

.priority-low { background: rgba(100, 149, 237, 0.15); color: #6495ed; }
.priority-normal { background: rgba(158, 158, 158, 0.15); color: #757575; }
.priority-high { background: rgba(255, 165, 0, 0.15); color: #ff8c00; }
.priority-urgent { background: rgba(224, 30, 90, 0.15); color: #e01e5a; }

.task-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 2px;
}

.task-progress-bar {
  flex: 1;
  height: 3px;
  background: rgba(0, 0, 0, 0.06);
  border-radius: 2px;
  overflow: hidden;
}

.task-progress-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.3s ease;
}

.fill-pending { background: #bdbdbd; }
.fill-running { background: #4095fe; }
.fill-completed { background: #2eb67d; }
.fill-failed { background: #e01e5a; }

.task-progress-text {
  font-size: 12px;
  color: var(--text-dim, #797979);
  flex-shrink: 0;
  min-width: 28px;
  text-align: right;
}

@media (max-width: 768px) {
  .task-panel-header {
    padding: 6px 12px;
  }
  .task-list-body {
    padding: 4px 12px 8px;
  }
  .task-stats {
    gap: 4px;
  }
  .stat {
    font-size: 10px;
    padding: 1px 4px;
  }
}

/* Refresh button */
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

/* Timeout select */
.timeout-select-wrap {
  position: relative;
  flex-shrink: 0;
}

.timeout-select {
  font-size: 10px;
  padding: 2px 14px 2px 4px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 4px;
  background: #fff;
  color: #757575;
  cursor: pointer;
  outline: none;
  appearance: none;
  -webkit-appearance: none;
}

.timeout-select:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.timeout-select-wrap::after {
  content: '';
  position: absolute;
  right: 4px;
  top: 50%;
  transform: translateY(-50%);
  width: 0;
  height: 0;
  border-left: 3px solid transparent;
  border-right: 3px solid transparent;
  border-top: 3px solid #999;
  pointer-events: none;
}

/* Pending confirm stat badge */
.stat.confirm {
  background: rgba(255, 152, 0, 0.15);
  color: #f57c00;
}

/* Pending confirm flashing highlight */
.task-row.pending-confirm {
  animation: flashBorder 1.5s ease-in-out infinite;
  border-radius: 4px;
  padding: 4px 6px;
  margin: 0 -6px;
}

@keyframes flashBorder {
  0%, 100% {
    background: transparent;
    box-shadow: inset 0 0 0 1px transparent;
  }
  50% {
    background: rgba(64, 149, 254, 0.06);
    box-shadow: inset 0 0 0 1px rgba(64, 149, 254, 0.3);
  }
}

/* Pending create (new task) */
.task-row.pending-create {
  animation: flashBorder 1.5s ease-in-out infinite;
  border-radius: 4px;
  padding: 4px 6px;
  margin: 0 -6px;
}

.new-badge {
  display: inline-block;
  font-size: 11px;
  padding: 0 4px;
  border-radius: 3px;
  background: rgba(64, 149, 254, 0.2);
  color: #4095fe;
  margin-right: 4px;
  font-weight: 600;
}

/* Pending reason + confirm button */
.pending-reason {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 3px;
}

.reason-text {
  font-size: 12px;
  color: #f57c00;
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.confirm-btn {
  font-size: 12px;
  padding: 1px 6px;
  border-radius: 3px;
  border: none;
  background: rgba(46, 182, 125, 0.15);
  color: #2eb67d;
  cursor: pointer;
  font-weight: 600;
  flex-shrink: 0;
  transition: all 0.15s;
}

.confirm-btn:hover {
  background: #2eb67d;
  color: #fff;
}
</style>
