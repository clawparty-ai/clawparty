<template>
  <div class="radar-panel" :class="{ fullscreen: isFullscreen }" :style="{ height: isFullscreen ? '100vh' : panelHeight + 'px' }">
    <div class="radar-panel-header" @click="toggleExpanded">
      <span class="radar-panel-icon">📡</span>
      <span class="radar-panel-title">雷达</span>
      <div class="radar-sub-btns">
        <button
          class="radar-sub-btn"
          :class="{ active: activeSubPanel === 'targets' }"
          @click.stop="activeSubPanel = 'targets'"
        ><span>🎯</span> 目标</button>
        <button
          class="radar-sub-btn"
          :class="{ active: activeSubPanel === 'probes' }"
          @click.stop="activeSubPanel = 'probes'"
        ><span>🔍</span> 探测</button>
        <button
          class="radar-sub-btn"
          :class="{ active: activeSubPanel === 'logs' }"
          @click.stop="activeSubPanel = 'logs'"
        ><span>🪵</span> 日志</button>
      </div>
      <span class="radar-panel-stats" v-if="activeSubPanel === 'targets' && targets.length > 0">{{ targets.length }} 目标</span>
      <span class="radar-panel-stats" v-else-if="activeSubPanel === 'probes' && probes.length > 0">{{ probes.length }} 探测</span>
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
      <!-- Targets / Probes: three-column layout -->
      <div v-if="activeSubPanel === 'targets' || activeSubPanel === 'probes'" class="radar-three-col">
        <div class="radar-left-col">
          <div class="radar-left-header">
            <span class="radar-left-title">{{ activeSubPanel === 'targets' ? '目标列表' : '探测列表' }}</span>
          </div>
          <div v-if="loadingData" class="radar-loading">
            <span class="loading-spinner"></span> 加载中...
          </div>
          <template v-else>
            <div
              v-for="item in (activeSubPanel === 'targets' ? targets : probes)"
              :key="item.name"
              class="radar-list-row"
              :class="{ selected: selectedItem && selectedItem.name === item.name }"
              @click="selectItem(item)"
            >
              <span class="radar-status-dot" :class="'status-' + (item.status || 'active')"></span>
              <div class="radar-list-info">
                <span class="radar-list-name">{{ item.name }}</span>
                <span class="radar-list-sub" v-if="activeSubPanel === 'targets'">{{ item.specLabel || '' }}</span>
                <span class="radar-list-sub" v-else>{{ item.channelLabel || '' }}</span>
              </div>
            </div>
            <button
              class="radar-new-btn"
              @click="handleCreate"
            >➕ 新建{{ activeSubPanel === 'targets' ? '目标' : '探测' }}</button>
          </template>
        </div>

        <div class="radar-center-col">
          <div v-if="!selectedItem" class="radar-empty">
            <span class="radar-empty-icon">📡</span>
            <span class="radar-empty-text">选择一个{{ activeSubPanel === 'targets' ? '目标' : '探测' }}查看详情</span>
          </div>
          <div v-else class="radar-detail-area">
            <div class="radar-detail-section">
              <div class="radar-detail-title">{{ activeSubPanel === 'targets' ? '目标详情' : '探测详情' }}</div>
              <div class="radar-detail-row"><span class="rd-label">名称</span><span class="rd-value">{{ selectedItem.name }}</span></div>
              <div class="radar-detail-row" v-if="selectedItem.description"><span class="rd-label">描述</span><span class="rd-value">{{ selectedItem.description }}</span></div>

              <!-- Target spec -->
              <template v-if="activeSubPanel === 'targets' && selectedItem.spec && selectedItem.spec.length > 0">
                <div class="rd-sub-section">规格</div>
                <div class="radar-detail-row" v-for="(s, i) in selectedItem.spec" :key="'spec-'+i">
                  <span class="rd-label rd-label-indent">{{ s.key || s[Object.keys(s)[0]] || '' }}</span>
                  <span class="rd-value">{{ s.value || s[Object.keys(s)[1]] || '' }}</span>
                </div>
              </template>

              <!-- Target channels -->
              <template v-if="activeSubPanel === 'targets' && selectedItem.channels && selectedItem.channels.length > 0">
                <div class="rd-sub-section">渠道</div>
                <div class="radar-channel-row" v-for="(ch, i) in selectedItem.channels" :key="'ch-'+i">
                  <span class="channel-tag">{{ ch.type || '' }}</span>
                  <span class="channel-loc">{{ ch.location || '' }}</span>
                </div>
              </template>

              <!-- Probe channel -->
              <template v-if="activeSubPanel === 'probes'">
                <div class="radar-detail-row"><span class="rd-label">渠道</span><span class="rd-value">{{ selectedItem.channel_type || '' }}</span></div>
                <div class="radar-detail-row" v-if="selectedItem.channel_location"><span class="rd-label">位置</span><span class="rd-value channel-loc">{{ selectedItem.channel_location }}</span></div>
                <div class="radar-detail-row" v-if="selectedItem.method"><span class="rd-label">方法</span><span class="rd-value">{{ selectedItem.method }}</span></div>
                <div class="radar-detail-row" v-if="selectedItem.schedule"><span class="rd-label">周期</span><span class="rd-value">{{ selectedItem.schedule }}</span></div>
              </template>

              <!-- Common fields -->
              <div class="radar-detail-row" v-if="selectedItem.source_probe"><span class="rd-label">来源探测</span><span class="rd-value">{{ selectedItem.source_probe }}</span></div>
              <div class="radar-detail-row"><span class="rd-label">状态</span><span class="rd-value"><span class="radar-status-dot" :class="'status-' + (selectedItem.status || 'active')"></span> {{ selectedItem.status || 'active' }}</span></div>
              <div class="radar-detail-row" v-if="selectedItem.last_scan"><span class="rd-label">最近扫描</span><span class="rd-value">{{ selectedItem.last_scan }}</span></div>
              <div class="radar-detail-row" v-if="selectedItem.last_run"><span class="rd-label">最近执行</span><span class="rd-value">{{ selectedItem.last_run }}</span></div>
            </div>

            <div class="radar-detail-section">
              <div class="radar-detail-title">🪵 最近{{ activeSubPanel === 'targets' ? '扫描' : '探测' }}</div>
              <div v-if="recentLogs.length === 0" class="rd-empty-log">暂无日志</div>
              <div v-for="(log, i) in recentLogs" :key="i" class="radar-log-line">
                <span class="log-time">{{ log.time?.slice(11, 16) || '' }}</span>
                <span class="log-msg">{{ log.summary || log.name }}</span>
              </div>
            </div>
          </div>
        </div>

        <div class="radar-right-col" ref="radarColRef">
          <canvas ref="radarCanvasRef" class="radar-canvas"></canvas>
          <div class="radar-status-overlay">
            <span v-if="activeSubPanel === 'targets'">{{ targets.length }} 目标</span>
            <span v-else>{{ probes.length }} 探测</span>
            <span class="radar-status-dot" :class="scanning ? 'status-running' : 'status-active'"></span>
          </div>
        </div>
      </div>

      <!-- Logs: two-column layout -->
      <div v-else class="radar-two-col">
        <div class="radar-left-col">
          <div class="radar-left-header">
            <span class="radar-left-title">日志文件</span>
            <select v-model="logFilter" class="radar-log-filter">
              <option value="all">全部</option>
              <option value="probe">探测日志</option>
              <option value="scan">扫描日志</option>
            </select>
          </div>
          <div v-if="loadingLogs" class="radar-loading">
            <span class="loading-spinner"></span> 加载中...
          </div>
          <div v-else-if="filteredLogs.length === 0" class="radar-empty">
            <span class="radar-empty-icon">🪵</span>
            <span class="radar-empty-text">暂无日志</span>
          </div>
          <div
            v-for="log in filteredLogs"
            :key="log.name"
            class="radar-list-row"
            :class="{ selected: selectedLog && selectedLog.name === log.name }"
            @click="selectLog(log)"
          >
            <span class="log-type-icon">{{ log.log_type === 'probe' ? '🔍' : '📡' }}</span>
            <div class="radar-list-info">
              <span class="radar-list-name">{{ log.name }}</span>
              <span class="radar-list-sub">{{ log.time }}</span>
            </div>
          </div>
        </div>
        <div class="radar-center-col">
          <div v-if="!selectedLog" class="radar-empty">
            <span class="radar-empty-icon">📄</span>
            <span class="radar-empty-text">选择一个日志文件查看内容</span>
          </div>
          <div v-else-if="loadingLogContent" class="radar-loading">
            <span class="loading-spinner"></span> 加载中...
          </div>
          <div v-else class="radar-log-viewer"><pre>{{ logContent }}</pre></div>
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
import { ref, watch, computed, nextTick, onMounted, onBeforeUnmount } from 'vue'
import { radarService } from '../services/radarService'

const props = defineProps({
  agentName: { type: String, required: true },
  expanded: { type: Boolean, default: true },
  initialHeight: { type: Number, default: 180 },
  refreshing: { type: Boolean, default: false }
})

const emit = defineEmits(['toggle', 'refresh', 'createTarget', 'createProbe'])

const isFullscreen = ref(false)
const panelHeight = ref(props.initialHeight)
const activeSubPanel = ref('targets')

const targets = ref([])
const probes = ref([])
const selectedItem = ref(null)
const loadingData = ref(false)

const logs = ref([])
const selectedLog = ref(null)
const logContent = ref('')
const loadingLogs = ref(false)
const loadingLogContent = ref(false)
const logFilter = ref('all')

const radarCanvasRef = ref(null)
const radarColRef = ref(null)
let radarAnimCleanup = null
let radarResizeObserver = null

// ── Data loading ──────────────────────────────────────

async function loadTargets() {
  if (!props.agentName) return
  try {
    var res = await radarService.getTargetsJson(props.agentName)
    if (res.data && res.data.targets) {
      targets.value = res.data.targets
    }
  } catch (e) {
    console.error('[Radar] Failed to load targets:', e)
  }
}

async function loadProbes() {
  if (!props.agentName) return
  try {
    var res = await radarService.getProbes(props.agentName)
    if (res.data) {
      probes.value = parseListMd(res.data, 'probes')
    }
  } catch (e) {
    console.error('[Radar] Failed to load probes:', e)
  }
}

async function loadLogs() {
  if (!props.agentName) return
  loadingLogs.value = true
  try {
    var res = await radarService.getLogs(props.agentName)
    if (res.data && res.data.logs) {
      logs.value = res.data.logs
    } else {
      logs.value = []
    }
  } catch (e) {
    console.error('[Radar] Failed to load logs:', e)
    logs.value = []
  } finally {
    loadingLogs.value = false
  }
}

var filteredLogs = computed(function () {
  if (logFilter.value === 'all') return logs.value
  return logs.value.filter(function (l) { return l.log_type === logFilter.value })
})

async function selectLog(log) {
  selectedLog.value = log
  loadingLogContent.value = true
  logContent.value = ''
  try {
    var res = await radarService.getLog(props.agentName, log.name)
    logContent.value = typeof res.data === 'string' ? res.data : (res.data || '')
  } catch (e) {
    console.error('[Radar] Failed to load log:', e)
    logContent.value = '(failed to load log)'
  } finally {
    loadingLogContent.value = false
  }
}

var scanning = computed(function () {
  return false // placeholder — future: check if any probe/target is in scan
})

// ── Selection ─────────────────────────────────────────

function selectItem(item) {
  selectedItem.value = item
  loadRecentLogs()
}

var recentLogs = ref([])

async function loadRecentLogs() {
  if (!selectedItem.value || !props.agentName) return
  recentLogs.value = []
  try {
    if (logs.value.length === 0) await loadLogs()
    var prefix = activeSubPanel.value === 'targets' ? 'scan-' : 'probe-'
    var relevant = logs.value.filter(function (l) {
      return l.name && l.name.indexOf(prefix) === 0
    }).slice(0, 5)
    var results = []
    for (var i = 0; i < relevant.length; i++) {
      try {
        var res = await radarService.getLog(props.agentName, relevant[i].name)
        var text = typeof res.data === 'string' ? res.data : ''
        var firstLine = text.split('\n').filter(Boolean)[0] || ''
        results.push({ name: relevant[i].name, time: relevant[i].time, summary: firstLine.slice(0, 80) })
      } catch (e) { /* skip */ }
    }
    recentLogs.value = results
  } catch (e) {
    console.error('[Radar] Failed to load recent logs:', e)
  }
}

// ── Create action ─────────────────────────────────────

function handleCreate() {
  if (activeSubPanel.value === 'targets') {
    emit('createTarget', '帮我创建一个雷达目标:\n\n名称: [目标名称]\n描述: [目标描述]\n规格: [详细规格]\n渠道: [监测渠道]\n\n请根据以上信息补充完整目标详情，并更新 targets.md。')
  } else {
    emit('createProbe', '帮我创建一个雷达探测:\n\n名称: [探测名称]\n描述: [探测描述]\n渠道类型: [website/rss/api/...]\n渠道位置: [URL或路径]\n探测方法: [keyword_match/...]\n执行周期: [daily/weekly/...]\n\n请根据以上信息补充完整探测详情，并更新 probes.md。')
  }
}

// ── Radar Canvas Animation ────────────────────────────

function setupRadarAnimation() {
  var canvas = radarCanvasRef.value
  var col = radarColRef.value
  if (!canvas || !col) return

  var ctx = canvas.getContext('2d')
  var angle = 0
  var animId = null
  var currentItems = activeSubPanel.value === 'targets' ? targets.value : probes.value

  function resize() {
    var rect = col.getBoundingClientRect()
    var size = Math.min(rect.width, rect.height || rect.width)
    var dpr = window.devicePixelRatio || 1
    canvas.width = size * dpr
    canvas.height = size * dpr
    canvas.style.width = size + 'px'
    canvas.style.height = size + 'px'
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  }

  // Fixed dot positions based on item count
  function getDotPositions(cx, cy, r, count) {
    var positions = []
    for (var i = 0; i < count; i++) {
      var a = 2 * Math.PI * i / count + 0.2 * i
      var d = r * (0.25 + ((i % 5) + 1) * 0.08)
      positions.push({ angle: a, dist: d })
    }
    return positions
  }

  function draw() {
    var w = canvas.width / (window.devicePixelRatio || 1)
    var h = canvas.height / (window.devicePixelRatio || 1)
    var cx = w / 2
    var cy = h / 2
    var r = Math.min(w, h) / 2 - 16

    ctx.clearRect(0, 0, w, h)

    // Background
    ctx.fillStyle = '#0a1628'
    ctx.fillRect(0, 0, w, h)

    // Border glow
    ctx.strokeStyle = 'rgba(0, 255, 65, 0.3)'
    ctx.lineWidth = 1
    ctx.beginPath()
    ctx.arc(cx, cy, r, 0, Math.PI * 2)
    ctx.stroke()

    // Concentric circles
    ctx.strokeStyle = 'rgba(0, 255, 65, 0.12)'
    ctx.lineWidth = 0.5
    for (var ci = 1; ci <= 3; ci++) {
      ctx.beginPath()
      ctx.arc(cx, cy, r * ci / 3, 0, Math.PI * 2)
      ctx.stroke()
    }

    // Crosshairs
    ctx.strokeStyle = 'rgba(0, 255, 65, 0.12)'
    ctx.beginPath()
    ctx.moveTo(cx - r, cy)
    ctx.lineTo(cx + r, cy)
    ctx.moveTo(cx, cy - r)
    ctx.lineTo(cx, cy + r)
    ctx.stroke()

    // Scan cone
    ctx.beginPath()
    ctx.moveTo(cx, cy)
    ctx.arc(cx, cy, r, angle - 0.4, angle + 0.4)
    ctx.closePath()
    ctx.fillStyle = 'rgba(0, 255, 65, 0.06)'
    ctx.fill()

    // Scan line
    ctx.beginPath()
    ctx.moveTo(cx, cy)
    ctx.lineTo(cx + r * Math.cos(angle), cy + r * Math.sin(angle))
    ctx.strokeStyle = 'rgba(0, 255, 65, 0.7)'
    ctx.lineWidth = 1.5
    ctx.stroke()

    // Dots
    var items = activeSubPanel.value === 'targets' ? targets.value : probes.value
    var positions = getDotPositions(cx, cy, r, items.length)

    for (var di = 0; di < items.length; di++) {
      var p = positions[di]
      var px = cx + p.dist * Math.cos(p.angle)
      var py = cy + p.dist * Math.sin(p.angle)

      var diff = ((angle - p.angle) % (2 * Math.PI) + 2 * Math.PI) % (2 * Math.PI)
      var isHighlighted = diff < 0.5 || diff > 2 * Math.PI - 0.5

      if (isHighlighted) {
        ctx.beginPath()
        ctx.arc(px, py, 5, 0, Math.PI * 2)
        ctx.fillStyle = 'rgba(0, 255, 65, 0.9)'
        ctx.fill()
        ctx.shadowColor = '#00ff41'
        ctx.shadowBlur = 8
        ctx.beginPath()
        ctx.arc(px, py, 3, 0, Math.PI * 2)
        ctx.fill()
        ctx.shadowBlur = 0
      } else {
        ctx.beginPath()
        ctx.arc(px, py, 3, 0, Math.PI * 2)
        ctx.fillStyle = 'rgba(0, 255, 65, 0.4)'
        ctx.fill()
      }
    }

    angle += 0.015
    if (angle > Math.PI * 2) angle -= Math.PI * 2

    animId = requestAnimationFrame(draw)
  }

  resize()
  draw()

  return function () {
    if (animId) cancelAnimationFrame(animId)
  }
}

// ── Refresh ───────────────────────────────────────────

async function loadAll() {
  loadingData.value = true
  try {
    await Promise.all([loadTargets(), loadProbes(), loadLogs()])
  } finally {
    loadingData.value = false
    nextTick(function () {
      startRadar()
    })
  }
}

function startRadar() {
  if (radarAnimCleanup) radarAnimCleanup()
  if (radarResizeObserver) { radarResizeObserver.disconnect(); radarResizeObserver = null }

  var col = radarColRef.value
  if (col) {
    radarResizeObserver = new ResizeObserver(function () {
      if (radarAnimCleanup) { radarAnimCleanup(); radarAnimCleanup = null }
      radarAnimCleanup = setupRadarAnimation()
    })
    radarResizeObserver.observe(col)
  }

  nextTick(function () {
    radarAnimCleanup = setupRadarAnimation()
  })
}

function onRefresh() {
  emit('refresh')
  loadAll()
}

// ── UI controls ───────────────────────────────────────

function toggleExpanded() {
  emit('toggle')
}

function toggleFullscreen() {
  isFullscreen.value = !isFullscreen.value
}

var startResize = (function () {
  var startY = 0
  var startH = 0
  return function (e) {
    startY = e.clientY || e.touches[0].clientY
    startH = panelHeight.value
    function onMove(ev) {
      var dy = (ev.clientY || ev.touches[0].clientY) - startY
      panelHeight.value = Math.max(100, startH + dy)
    }
    function onUp() {
      document.removeEventListener('mousemove', onMove)
      document.removeEventListener('mouseup', onUp)
      document.removeEventListener('touchmove', onMove)
      document.removeEventListener('touchend', onUp)
    }
    document.addEventListener('mousemove', onMove)
    document.addEventListener('mouseup', onUp)
    document.addEventListener('touchmove', onMove)
    document.addEventListener('touchend', onUp)
  }
})()

// ── Watchers ──────────────────────────────────────────

watch(activeSubPanel, function () {
  selectedItem.value = null
  if (activeSubPanel.value !== 'logs') {
    nextTick(function () { startRadar() })
  }
})

watch(function () { return props.agentName }, function () {
  loadAll()
})

watch(isFullscreen, function () {
  if (activeSubPanel.value !== 'logs') {
    nextTick(function () { startRadar() })
  }
})

// ── Lifecycle ─────────────────────────────────────────

onMounted(function () {
  radarService.initRadar(props.agentName).catch(function (e) {
    console.error('[Radar] Init failed:', e)
  })
  loadAll()
})

onBeforeUnmount(function () {
  if (radarAnimCleanup) radarAnimCleanup()
  if (radarResizeObserver) radarResizeObserver.disconnect()
})
</script>

<style scoped>
/* ── Panel root ───────────────────────────────────── */
.radar-panel {
  display: flex;
  flex-direction: column;
  background: var(--bg-chat, #f5f5f5);
  border-bottom: 1px solid var(--border-subtle, rgba(0,0,0,0.07));
  position: relative;
  min-height: 40px;
  overflow: hidden;
}
.radar-panel.fullscreen {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  z-index: 1000;
  height: 100vh !important;
}

/* ── Header ────────────────────────────────────────── */
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
.radar-panel-icon { font-size: 14px; }
.radar-panel-title { font-size: 13px; font-weight: 600; color: var(--text-primary, #4d4d4d); }

.radar-sub-btns {
  display: flex;
  gap: 4px;
  margin-left: 8px;
}
.radar-sub-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 10px;
  border: 1px solid rgba(64, 149, 254, 0.25);
  background: rgba(64, 149, 254, 0.06);
  color: #4095fe;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  line-height: 20px;
  transition: all 0.15s;
}
.radar-sub-btn:hover {
  background: rgba(64, 149, 254, 0.12);
  border-color: rgba(64, 149, 254, 0.4);
}
.radar-sub-btn.active {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  border-color: transparent;
}
.radar-sub-btn span { font-size: 11px; }

.radar-panel-stats {
  font-size: 11px;
  color: var(--text-dim, #797979);
  background: var(--bg-hover, rgba(0,0,0,0.04));
  padding: 0 8px;
  border-radius: 8px;
  line-height: 20px;
  white-space: nowrap;
}

.radar-panel-actions {
  margin-left: auto;
  display: flex;
  gap: 4px;
}
.radar-fullscreen-btn {
  width: 28px; height: 28px;
  border: none; border-radius: 4px;
  background: transparent;
  color: var(--text-secondary, #797979);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
}
.radar-fullscreen-btn:hover { background: var(--bg-hover, rgba(0,0,0,0.04)); }
.radar-fullscreen-btn.active { background: rgba(64,149,254,0.15); color: #4095fe; }

.refresh-btn {
  width: 28px; height: 28px;
  border: none; border-radius: 4px;
  background: transparent;
  color: var(--text-secondary, #797979);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.refresh-btn:hover { background: var(--bg-hover, rgba(0,0,0,0.04)); color: var(--text-primary, #4d4d4d); }
.refresh-btn.spinning svg { animation: spin 1s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

/* ── Body ──────────────────────────────────────────── */
.radar-panel-body {
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

/* ── Three-column layout ────────────────────────────── */
.radar-three-col {
  display: flex;
  height: 100%;
  overflow: hidden;
}

/* ── Left column ──────────────────────────────────── */
.radar-left-col {
  width: 220px;
  min-width: 180px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border-subtle, rgba(0,0,0,0.07));
  overflow: hidden;
}
.radar-left-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-subtle, rgba(0,0,0,0.07));
  flex-shrink: 0;
}
.radar-left-title { font-size: 12px; font-weight: 600; color: var(--text-secondary, #797979); }

.radar-log-filter {
  font-size: 10px;
  padding: 1px 4px;
  border: 1px solid var(--border-light, rgba(0,0,0,0.1));
  border-radius: 3px;
  background: #fff;
  color: #757575;
  outline: none;
}

.radar-list-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 12px;
  cursor: pointer;
  border-bottom: 1px solid rgba(0,0,0,0.03);
  transition: background 0.1s;
}
.radar-list-row:hover { background: var(--bg-hover, rgba(0,0,0,0.02)); }
.radar-list-row.selected { background: rgba(64,149,254,0.08); }

.radar-status-dot {
  width: 8px; height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  margin-top: 4px;
}
.radar-status-dot.status-active,
.radar-status-dot.status-monitoring,
.radar-status-dot.status-running { background: #22c55e; }
.radar-status-dot.status-paused { background: #f59e0b; }
.radar-status-dot.status-archived { background: #9ca3af; }

.radar-list-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.radar-list-name {
  font-size: 13px;
  color: var(--text-primary, #4d4d4d);
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.radar-list-sub {
  font-size: 11px;
  color: var(--text-dim, #999);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.radar-new-btn {
  display: block;
  width: calc(100% - 16px);
  margin: 8px;
  padding: 6px;
  border: 1px dashed var(--border-light, rgba(0,0,0,0.15));
  border-radius: 4px;
  background: transparent;
  color: var(--text-dim, #999);
  font-size: 12px;
  cursor: pointer;
  text-align: center;
  transition: all 0.15s;
}
.radar-new-btn:hover {
  border-color: #4095fe;
  color: #4095fe;
  background: rgba(64,149,254,0.05);
}

/* ── Center column ────────────────────────────────── */
.radar-center-col {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
}

.radar-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 8px;
  color: var(--text-dim, #999);
}
.radar-empty-icon { font-size: 32px; }
.radar-empty-text { font-size: 13px; }

.radar-detail-area {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.radar-detail-section {
  background: var(--bg-primary, #fff);
  border: 1px solid var(--border-subtle, rgba(0,0,0,0.07));
  border-radius: 6px;
  padding: 12px;
}
.radar-detail-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary, #797979);
  margin-bottom: 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border-subtle, rgba(0,0,0,0.07));
}
.radar-detail-row {
  display: flex;
  gap: 8px;
  margin-bottom: 4px;
  font-size: 13px;
  line-height: 1.6;
}
.rd-label {
  font-weight: 600;
  color: var(--text-dim, #999);
  flex-shrink: 0;
  min-width: 64px;
  font-size: 12px;
}
.rd-label-indent { padding-left: 8px; min-width: 56px; }
.rd-value {
  color: var(--text-primary, #4d4d4d);
  word-break: break-word;
}
.rd-sub-section {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-dim, #999);
  margin: 8px 0 4px;
  padding-top: 4px;
  border-top: 1px solid var(--border-subtle, rgba(0,0,0,0.05));
}

.radar-channel-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}
.channel-tag {
  display: inline-block;
  padding: 1px 6px;
  border-radius: 3px;
  background: rgba(64,149,254,0.1);
  color: #4095fe;
  font-size: 11px;
  font-weight: 500;
}
.channel-loc {
  font-size: 12px;
  color: var(--text-primary, #4d4d4d);
  word-break: break-all;
}

.rd-empty-log {
  font-size: 12px;
  color: var(--text-dim, #999);
  padding: 8px 0;
  text-align: center;
}

.radar-log-line {
  display: flex;
  gap: 6px;
  padding: 3px 0;
  font-size: 12px;
  line-height: 1.5;
  border-bottom: 1px solid rgba(0,0,0,0.03);
}
.log-time {
  color: var(--text-dim, #999);
  flex-shrink: 0;
  font-family: monospace;
  font-size: 11px;
}
.log-msg {
  color: var(--text-primary, #4d4d4d);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ── Right column (radar animation) ───────────────── */
.radar-right-col {
  height: 100%;
  aspect-ratio: 1;
  flex-shrink: 0;
  min-width: 120px;
  max-width: 320px;
  position: relative;
  overflow: hidden;
  background: #0a1628;
}
.radar-panel.fullscreen .radar-right-col {
  max-width: 480px;
}
.radar-canvas {
  display: block;
}
.radar-status-overlay {
  position: absolute;
  bottom: 8px;
  left: 0;
  right: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-size: 11px;
  color: rgba(0, 255, 65, 0.7);
  font-family: monospace;
}
.radar-status-overlay .radar-status-dot { margin-top: 0; }

/* ── Two-column layout (logs) ──────────────────────── */
.radar-two-col {
  display: flex;
  height: 100%;
  overflow: hidden;
}

/* ── Log viewer ────────────────────────────────────── */
.radar-log-viewer {
  height: 100%;
  overflow-y: auto;
}
.radar-log-viewer pre {
  margin: 0;
  font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  font-size: 11px;
  line-height: 1.6;
  color: var(--text-primary, #4d4d4d);
  white-space: pre-wrap;
  word-break: break-word;
}

.log-type-icon { font-size: 12px; flex-shrink: 0; margin-top: 2px; }

/* ── Loading ───────────────────────────────────────── */
.radar-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px;
  color: var(--text-secondary, #797979);
  font-size: 13px;
}
.loading-spinner {
  width: 14px; height: 14px;
  border: 2px solid var(--border-subtle, rgba(0,0,0,0.1));
  border-top-color: #4095fe;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

/* ── Resize handle ─────────────────────────────────── */
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
  background: var(--border-subtle, rgba(0,0,0,0.1));
  border-radius: 1px;
}
.resize-handle:hover::after { background: #4095fe; }
</style>
