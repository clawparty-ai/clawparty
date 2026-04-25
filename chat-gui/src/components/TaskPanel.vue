<template>
  <div class="task-panel">
    <div class="task-panel-header" @click="toggleExpanded">
      <span class="task-panel-icon">🎯</span>
      <span class="task-panel-title">任务</span>
      <span class="task-panel-stats">
        <span class="stat running" v-if="taskStats.running > 0">{{ taskStats.running }} 执行中</span>
        <span class="stat completed" v-if="taskStats.completed > 0">{{ taskStats.completed }} 完成</span>
        <span class="stat pending" v-if="taskStats.pending > 0">{{ taskStats.pending }} 待办</span>
        <span class="stat pending" v-else-if="taskStats.total === 0">0 待办</span>
        <span class="stat failed" v-if="taskStats.failed > 0">{{ taskStats.failed }} 失败</span>
      </span>
      <span class="task-panel-toggle">{{ expanded ? '▼' : '▶' }}</span>
    </div>
    <div v-show="expanded" class="task-panel-body">
      <div v-if="tasks.length === 0" class="task-empty">
        <span class="task-empty-icon">✨</span>
        <span class="task-empty-text">暂无任务，跟我说「帮我做一件事」来开始吧</span>
      </div>
      <div
        v-for="task in flattenedTasks"
        :key="task.task_id"
        v-else
        class="task-row"
        :class="['indent-' + Math.min(task.depth, 3), 'status-' + task.status]"
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
            <span class="task-title" :title="task.description">{{ task.title }}</span>
            <span class="task-priority" v-if="task.priority !== 'normal'" :class="'priority-' + task.priority">{{ formatPriority(task.priority) }}</span>
          </div>
          <div class="task-meta">
            <div class="task-progress-bar">
              <div class="task-progress-fill" :class="'fill-' + task.status" :style="{ width: task.progress + '%' }"></div>
            </div>
            <span class="task-progress-text">{{ task.progress }}%</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

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
  }
})

const emit = defineEmits(['toggle'])

const toggleExpanded = () => {
  emit('toggle')
}

const formatPriority = (priority) => {
  const map = { low: '低', normal: '中', high: '高', urgent: '紧急' }
  return map[priority] || priority
}

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
  return { total, pending, running, completed, failed }
})
</script>

<style scoped>
.task-panel {
  flex-shrink: 0;
  background: var(--bg-secondary, #f3f6fc);
  border-bottom: 1px solid var(--border-subtle, rgba(0, 0, 0, 0.07));
  max-height: 300px;
  overflow-y: auto;
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
  background: var(--bg-secondary, #f3f6fc);
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
  font-size: 13px;
  color: var(--text-primary, #4d4d4d);
}

.task-panel-stats {
  display: flex;
  gap: 8px;
  margin-left: auto;
}

.stat {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 10px;
  font-weight: 500;
}

.stat.running { background: rgba(64, 149, 254, 0.12); color: #4095fe; }
.stat.completed { background: rgba(46, 182, 125, 0.12); color: #2eb67d; }
.stat.pending { background: rgba(158, 158, 158, 0.12); color: #757575; }
.stat.failed { background: rgba(224, 30, 90, 0.12); color: #e01e5a; }

.task-panel-toggle {
  font-size: 10px;
  color: var(--text-dim, #797979);
  margin-left: 4px;
}

.task-panel-body {
  padding: 4px 16px 10px;
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

.task-title {
  font-size: 12px;
  color: var(--text-primary, #4d4d4d);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 500;
}

.task-priority {
  font-size: 10px;
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
  font-size: 10px;
  color: var(--text-dim, #797979);
  flex-shrink: 0;
  min-width: 24px;
  text-align: right;
}

@media (max-width: 768px) {
  .task-panel-header {
    padding: 6px 12px;
  }
  .task-panel-body {
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
  font-size: 12px;
  color: var(--text-dim, #797979);
  line-height: 1.5;
}

</style>
