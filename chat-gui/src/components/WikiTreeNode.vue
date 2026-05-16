<template>
  <div class="tree-node" :style="{ paddingLeft: level * 12 + 'px' }">
    <div
      class="tree-item"
      :class="{ active: isActive, directory: item.type === 'dir' }"
      @click="handleClick"
    >
      <span class="tree-icon">{{ item.type === 'dir' ? (isExpanded ? '📂' : '📁') : getFileIcon(item.name) }}</span>
      <span class="tree-label" :title="item.title || item.name">{{ item.title || item.name }}</span>
    </div>
    
    <div v-if="item.type === 'dir' && isExpanded" class="tree-children">
      <WikiTreeNode
        v-for="child in item.children"
        :key="child.path || child.name"
        :item="child"
        :level="level + 1"
        :activePath="activePath"
        @select="$emit('select', $event)"
      />
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'

const props = defineProps({
  item: {
    type: Object,
    required: true
  },
  level: {
    type: Number,
    default: 0
  },
  activePath: {
    type: String,
    default: ''
  }
})

const emit = defineEmits(['select'])

const isExpanded = ref(props.level < 1)

const isActive = computed(() => {
  return props.item.path === props.activePath
})

const handleClick = () => {
  if (props.item.type === 'dir') {
    isExpanded.value = !isExpanded.value
  } else {
    emit('select', props.item)
  }
}

const getFileIcon = (name) => {
  if (name === 'index.md') return '📇'
  if (name === 'log.md') return '📝'
  if (name === 'schema.md') return '⚙️'
  return '📄'
}
</script>

<style scoped>
.tree-node {
  user-select: none;
}

.tree-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  color: var(--text-primary);
}

.tree-item:hover {
  background: var(--bg-hover);
}

.tree-item.active {
  background: rgba(64, 149, 254, 0.15);
  color: #4095fe;
}

.tree-item.directory {
  font-weight: 500;
}

.tree-icon {
  font-size: 11px;
  flex-shrink: 0;
}

.tree-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tree-children {
  margin-top: 2px;
}
</style>
