<template>
  <div class="ai-panel" :style="{ height: panelHeight + 'px' }">
    <div class="ai-panel-header">
      <span class="ai-panel-icon">🤖</span>
      <span class="ai-panel-title">AI Panel</span>
      <span class="ai-panel-count" v-if="entries.length">{{ entries.length }}</span>
    </div>
    <div class="ai-panel-body" ref="panelBody">
      <div v-if="entries.length === 0" class="ai-panel-empty">
        Thinking log will appear here as the agent reasons...
      </div>
      <div
        v-for="(entry, idx) in entries"
        :key="idx"
        class="ai-panel-entry"
        :class="{ 'is-reasoning': entry.type === 'reasoning', 'is-tool': entry.type === 'tool' }"
      >
        <div class="ai-entry-time">{{ entry.time }}</div>
        <div class="ai-entry-content">{{ entry.content }}</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, nextTick } from 'vue'

const props = defineProps({
  entries: {
    type: Array,
    default: () => []
  },
  panelHeight: {
    type: Number,
    default: 200
  }
})

const panelBody = ref(null)

watch(() => props.entries.length, () => {
  nextTick(() => {
    if (panelBody.value) {
      panelBody.value.scrollTop = panelBody.value.scrollHeight
    }
  })
})
</script>

<style scoped>
.ai-panel {
  background: #f8f9fa;
  border-bottom: 1px solid #e0e0e0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.ai-panel-header {
  display: flex;
  align-items: center;
  padding: 6px 12px;
  background: #eef1f5;
  border-bottom: 1px solid #ddd;
  flex-shrink: 0;
}

.ai-panel-icon {
  font-size: 14px;
  margin-right: 6px;
}

.ai-panel-title {
  font-size: 12px;
  font-weight: 600;
  color: #555;
}

.ai-panel-count {
  font-size: 11px;
  color: #999;
  margin-left: auto;
  background: #ddd;
  border-radius: 10px;
  padding: 1px 8px;
}

.ai-panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  font-size: 12px;
  line-height: 1.5;
}

.ai-panel-empty {
  color: #aaa;
  text-align: center;
  padding: 20px;
  font-style: italic;
}

.ai-panel-entry {
  margin-bottom: 6px;
  padding: 4px 6px;
  border-radius: 4px;
  border-left: 2px solid #ccc;
}

.ai-panel-entry.is-reasoning {
  background: #f0f0ff;
  border-left-color: #88a;
}

.ai-panel-entry.is-tool {
  background: #fff8e1;
  border-left-color: #ca8;
}

.ai-entry-time {
  font-size: 10px;
  color: #999;
  margin-bottom: 2px;
}

.ai-entry-content {
  color: #666;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
