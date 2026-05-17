<template>
  <div class="radar-panel" :class="{ fullscreen: isFullscreen }" :style="{ height: isFullscreen ? '100vh' : panelHeight + 'px' }">
    <div class="radar-panel-header" @click="toggleExpanded">
      <span class="radar-panel-icon">📡</span>
      <span class="radar-panel-title">雷达</span>
      <div class="radar-tabs">
        <button
          class="radar-tab"
          :class="{ active: activeTab === 'discovery' }"
          @click.stop="activeTab = 'discovery'"
        >发现</button>
        <button
          class="radar-tab"
          :class="{ active: activeTab === 'target' }"
          @click.stop="activeTab = 'target'"
        >目标</button>
      </div>
      <span class="radar-panel-count" v-if="activeTab === 'target' && targets.length > 0">{{ targets.length }}</span>
      <div class="radar-panel-actions">
        <button
          class="radar-fullscreen-btn"
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
    <div v-show="expanded" class="radar-panel-body">
      <!-- Discovery tab placeholder -->
      <div v-if="activeTab === 'discovery'" class="radar-discovery-placeholder">
        <div class="placeholder-icon">🔍</div>
        <div class="placeholder-title">发现面板</div>
        <div class="placeholder-desc">即将推出 — 在此处定义搜索条件并发现新目标</div>
        <div class="placeholder-hint">
          如果您需要创建雷达扫描条件，请在 agent 的 workspace/radar/scans/ 目录中添加 .md 文件。
        </div>
      </div>

      <!-- Target tab -->
      <div v-else class="radar-target-layout">
        <div class="radar-targets-list">
          <div v-if="targets.length === 0 && !loadingTargets" class="radar-empty">
            <span class="radar-empty-icon">🎯</span>
            <span class="radar-empty-text">暂无监控目标</span>
            <span class="radar-empty-hint">外部离线任务可以将收集到的目标写入 workspace/radar/targets/ 目录</span>
          </div>
          <div v-else-if="loadingTargets" class="radar-loading">
            <span class="loading-spinner"></span>
            加载中...
          </div>
          <div
            v-for="target in targets"
            :key="target.name"
            class="radar-target-row"
            :class="{ selected: selectedTarget && selectedTarget.name === target.name }"
            @click="selectTarget(target)"
          >
            <span class="radar-target-status-dot" :class="'status-' + target.status"></span>
            <span class="radar-target-name">{{ target.name }}</span>
            <span class="radar-target-meta">
              <span v-if="target.log_entries > 0" class="radar-target-log-count">{{ target.log_entries }}</span>
            </span>
          </div>
        </div>
        <div class="radar-target-detail">
          <div v-if="!selectedTarget" class="radar-detail-empty">
            <span class="detail-empty-icon">📡</span>
            <span class="detail-empty-text">选择一个目标查看详情</span>
          </div>
          <div v-else-if="loadingDetail" class="radar-loading">
            <span class="loading-spinner"></span>
            加载中...
          </div>
          <div v-else class="radar-detail-content">
            <div class="radar-detail-tabs">
              <button
                class="detail-tab"
                :class="{ active: detailTab === 'info' }"
                @click="detailTab = 'info'"
              >概况</button>
              <button
                class="detail-tab"
                :class="{ active: detailTab === 'log' }"
                @click="detailTab = 'log'"
              >情报日志</button>
            </div>
            <div class="radar-detail-body">
              <div v-if="detailTab === 'info'" class="radar-info-content" v-html="renderedInfo"></div>
              <div v-else class="radar-log-content" v-html="renderedLog"></div>
            </div>
          </div>
        </div>
      </div>
    </div>
    <div
      v-show="expanded"
      class="resize-handle"
      @mousedown="startResize"
      @touchstart="startResize"
    ></div>
  </div>
</template>

<script setup>
import { ref, watch, computed, nextTick, onMounted } from 'vue'
import { marked } from 'marked'
import { radarService } from '../services/radarService'

const props = defineProps({
  agentName: { type: String, required: true },
  expanded: { type: Boolean, default: true },
  initialHeight: { type: Number, default: 180 },
  refreshing: { type: Boolean, default: false }
})

const emit = defineEmits(['toggle', 'refresh'])

const isFullscreen = ref(false)
const panelHeight = ref(props.initialHeight)
const activeTab = ref('target')
const detailTab = ref('info')
const targets = ref([])
const selectedTarget = ref(null)
const targetInfo = ref('')
const targetLog = ref('')
const loadingTargets = ref(false)
const loadingDetail = ref(false)

const renderedInfo = computed(() => {
  if (!targetInfo.value) return ''
  return marked.parse(targetInfo.value)
})

const renderedLog = computed(() => {
  if (!targetLog.value) return ''
  return marked.parse(targetLog.value)
})

const loadTargets = async () => {
  if (!props.agentName) return
  loadingTargets.value = true
  try {
    const res = await radarService.getTargets(props.agentName)
    if (res.data && res.data.targets) {
      targets.value = res.data.targets
    }
  } catch (e) {
    console.error('[Radar] Failed to load targets:', e)
  } finally {
    loadingTargets.value = false
  }
}

const selectTarget = async (target) => {
  selectedTarget.value = target
  detailTab.value = 'info'
  loadingDetail.value = true
  try {
    const [infoRes, logRes] = await Promise.all([
      radarService.getTargetInfo(props.agentName, target.name),
      radarService.getTargetLog(props.agentName, target.name)
    ])
    targetInfo.value = infoRes.data || ''
    targetLog.value = logRes.data || ''
  } catch (e) {
    console.error('[Radar] Failed to load target detail:', e)
    targetInfo.value = ''
    targetLog.value = ''
  } finally {
    loadingDetail.value = false
  }
}

const onRefresh = () => {
  emit('refresh')
  loadTargets()
}

const toggleExpanded = () => {
  emit('toggle')
}

const toggleFullscreen = () => {
  isFullscreen.value = !isFullscreen.value
}

const startResize = (e) => {
  const startY = e.clientY || e.touches[0].clientY
  const startH = panelHeight.value
  const handleMouseMove = (ev) => {
    const dy = (ev.clientY || ev.touches[0].clientY) - startY
    panelHeight.value = Math.max(100, startH + dy)
  }
  const handleMouseUp = () => {
    document.removeEventListener('mousemove', handleMouseMove)
    document.removeEventListener('mouseup', handleMouseUp)
    document.removeEventListener('touchmove', handleMouseMove)
    document.removeEventListener('touchend', handleMouseUp)
  }
  document.addEventListener('mousemove', handleMouseMove)
  document.addEventListener('mouseup', handleMouseUp)
  document.addEventListener('touchmove', handleMouseMove)
  document.addEventListener('touchend', handleMouseUp)
}

onMounted(async () => {
  try {
    await radarService.initRadar(props.agentName)
  } catch (e) {
    console.error('[Radar] Init failed:', e)
  }
  await loadTargets()
})
</script>

<style scoped>
.radar-panel {
  display: flex;
  flex-direction: column;
  background: var(--bg-chat);
  border-bottom: 1px solid var(--border-subtle);
  position: relative;
  min-height: 40px;
  overflow: hidden;
}
.radar-panel.fullscreen {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1000;
  height: 100vh !important;
}

.radar-panel-header {
  display: flex;
  align-items: center;
  padding: 0 16px;
  height: 40px;
  gap: 8px;
  cursor: pointer;
  user-select: none;
  flex-shrink: 0;
}
.radar-panel-icon {
  font-size: 14px;
}
.radar-panel-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}
.radar-tabs {
  display: flex;
  gap: 4px;
  margin-left: 12px;
}
.radar-tab {
  padding: 2px 10px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
}
.radar-tab:hover {
  background: var(--bg-hover);
}
.radar-tab.active {
  background: rgba(64, 149, 254, 0.15);
  color: #4095fe;
  font-weight: 600;
}
.radar-panel-count {
  font-size: 11px;
  color: var(--text-dim);
  background: var(--bg-hover);
  padding: 0 6px;
  border-radius: 8px;
  line-height: 18px;
}
.radar-panel-actions {
  margin-left: auto;
  display: flex;
  gap: 4px;
}
.radar-fullscreen-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
}
.radar-fullscreen-btn:hover {
  background: var(--bg-hover);
}
.radar-fullscreen-btn.active {
  background: rgba(64, 149, 254, 0.15);
  color: #4095fe;
}
.refresh-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.refresh-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.refresh-btn.spinning svg {
  animation: spin 1s linear infinite;
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.radar-panel-body {
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

/* Discovery tab placeholder */
.radar-discovery-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  padding: 40px 20px;
  text-align: center;
}
.placeholder-icon {
  font-size: 48px;
  margin-bottom: 16px;
}
.placeholder-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 8px;
}
.placeholder-desc {
  font-size: 14px;
  color: var(--text-secondary);
  margin-bottom: 16px;
}
.placeholder-hint {
  font-size: 12px;
  color: var(--text-dim);
  max-width: 400px;
  line-height: 1.6;
}

/* Target layout */
.radar-target-layout {
  display: flex;
  height: 100%;
}
.radar-targets-list {
  width: 220px;
  min-width: 180px;
  overflow-y: auto;
  border-right: 1px solid var(--border-subtle);
  padding: 8px 0;
}
.radar-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 32px 16px;
  text-align: center;
}
.radar-empty-icon {
  font-size: 32px;
  margin-bottom: 8px;
}
.radar-empty-text {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}
.radar-empty-hint {
  font-size: 11px;
  color: var(--text-dim);
  line-height: 1.5;
}
.radar-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 32px;
  color: var(--text-secondary);
  font-size: 13px;
}
.loading-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid var(--border-subtle);
  border-top-color: #4095fe;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}
.radar-target-row {
  display: flex;
  align-items: center;
  padding: 8px 16px;
  cursor: pointer;
  gap: 8px;
}
.radar-target-row:hover {
  background: var(--bg-hover);
}
.radar-target-row.selected {
  background: rgba(64, 149, 254, 0.1);
}
.radar-target-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.radar-target-status-dot.status-active {
  background: #22c55e;
}
.radar-target-status-dot.status-paused {
  background: #f59e0b;
}
.radar-target-status-dot.status-archived {
  background: #9ca3af;
}
.radar-target-name {
  font-size: 13px;
  color: var(--text-primary);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.radar-target-meta {
  display: flex;
  align-items: center;
  gap: 4px;
}
.radar-target-log-count {
  font-size: 11px;
  color: var(--text-dim);
  background: var(--bg-hover);
  padding: 0 6px;
  border-radius: 8px;
  line-height: 16px;
}

/* Target detail area */
.radar-target-detail {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.radar-detail-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 8px;
  color: var(--text-dim);
}
.detail-empty-icon {
  font-size: 32px;
}
.detail-empty-text {
  font-size: 13px;
}
.radar-detail-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border-subtle);
  padding: 0 16px;
  flex-shrink: 0;
}
.detail-tab {
  padding: 8px 16px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
}
.detail-tab:hover {
  color: var(--text-primary);
}
.detail-tab.active {
  color: #4095fe;
  font-weight: 600;
  border-bottom-color: #4095fe;
}
.radar-detail-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}
.radar-info-content,
.radar-log-content {
  font-size: 13px;
  line-height: 1.7;
  color: var(--text-primary);
}
.radar-info-content h1,
.radar-log-content h1 {
  font-size: 18px;
  margin: 0 0 12px;
}
.radar-info-content h2,
.radar-log-content h2 {
  font-size: 15px;
  margin: 16px 0 8px;
}
.radar-info-content p,
.radar-log-content p {
  margin: 0 0 8px;
}
.radar-info-content ul,
.radar-log-content ul {
  margin: 0 0 8px;
  padding-left: 20px;
}

.resize-handle {
  height: 4px;
  cursor: ns-resize;
  flex-shrink: 0;
  position: relative;
}
.resize-handle::after {
  content: '';
  position: absolute;
  top: 1px;
  left: 50%;
  transform: translateX(-50%);
  width: 40px;
  height: 2px;
  background: var(--border-subtle);
  border-radius: 1px;
}
.resize-handle:hover::after {
  background: #4095fe;
}
</style>
