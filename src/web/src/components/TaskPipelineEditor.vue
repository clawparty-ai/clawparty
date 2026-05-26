<template>
  <div class="pipeline-editor">
    <div class="pipeline-header">
      <span class="pipeline-icon">🔗</span>
      <span class="pipeline-title">{{ mode === 'edit' ? '流水线编辑' : '流水线设计' }}</span>
      <button class="pipeline-close-btn" @click="$emit('close')" title="关闭流水线">×</button>
    </div>

    <div class="pipeline-body">
      <div
        class="pipeline-drop-zone"
        :class="{ 'drag-over': isDragOver, 'has-tasks': pipeline.length > 0 }"
        @dragover.prevent="onDragOver"
        @dragleave.prevent="onDragLeave"
        @drop.prevent="onDrop"
      >
        <div v-if="pipeline.length === 0" class="pipeline-empty">
          <span class="empty-icon">📋</span>
          <span class="empty-text">从左侧拖拽任务到这里构建流水线</span>
        </div>
        <div v-else class="pipeline-tasks">
          <div
            v-for="(task, index) in pipeline"
            :key="task.task_id"
            class="pipeline-task-card"
            :class="{ 'dragging': dragIndex === index }"
            draggable="true"
            @dragstart="onCardDragStart($event, index)"
            @dragover.prevent="onCardDragOver($event, index)"
            @drop.prevent.stop="onCardDrop($event, index)"
            @dragend="onCardDragEnd"
          >
            <div class="task-card-content">
              <span class="task-card-number">{{ index + 1 }}</span>
              <span class="task-card-title">{{ task.title || task.short_title || '未命名任务' }}</span>
              <button class="task-card-remove" @click.stop="removeTask(index)" title="移除">×</button>
            </div>
            <div v-if="index < pipeline.length - 1" class="pipeline-arrow">
              <span class="arrow-icon">→</span>
            </div>
          </div>
        </div>
      </div>

      <div class="prompt-section">
        <div class="prompt-header">
          <span class="prompt-label">提示词编辑区</span>
        </div>
        <textarea
          v-model="promptText"
          class="prompt-textarea"
          placeholder="点击「生成提示词」自动生成流水线提示词，或手动编辑..."
          rows="4"
        ></textarea>
        <div class="prompt-actions">
          <button
            class="action-btn generate-btn"
            :disabled="pipeline.length < 2 || isGenerating"
            @click="generatePrompt"
          >
            {{ isGenerating ? '生成中...' : '生成提示词' }}
          </button>
          <button
            v-if="mode === 'create'"
            class="action-btn create-btn"
            :disabled="!promptText.trim() || isCreating"
            @click="createTask"
          >
            {{ isCreating ? '创建中...' : '创建任务' }}
          </button>
          <button
            v-else
            class="action-btn save-btn"
            :disabled="!promptText.trim() || isSaving"
            @click="savePipeline"
          >
            {{ isSaving ? '保存中...' : '保存' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'

const props = defineProps({
  tasks: {
    type: Array,
    default: () => []
  },
  agentName: {
    type: String,
    default: ''
  },
  mode: {
    type: String,
    default: 'create'
  },
  pipelineTask: {
    type: Object,
    default: null
  }
})

const emit = defineEmits(['close', 'createTask', 'savePipeline'])

const pipeline = ref([])
const promptText = ref('')
const isGenerating = ref(false)
const isCreating = ref(false)
const isSaving = ref(false)
const isDragOver = ref(false)
const dragIndex = ref(-1)

// Load pipeline data when in edit mode
watch(() => props.pipelineTask, (newTask) => {
  if (props.mode === 'edit' && newTask && newTask.pipeline_definition) {
    pipeline.value = []
    for (let i = 0; i < newTask.pipeline_definition.length; i++) {
      const taskId = newTask.pipeline_definition[i]
      const task = props.tasks.find(t => t.task_id === taskId)
      if (task) {
        pipeline.value.push({
          task_id: task.task_id,
          title: task.short_title || task.title || '未命名任务',
          description: task.description || task.ai_description || ''
        })
      }
    }
    promptText.value = newTask.description || ''
  }
}, { immediate: true })

// Reset when switching to create mode
watch(() => props.mode, (newMode) => {
  if (newMode === 'create') {
    pipeline.value = []
    promptText.value = ''
  }
})

let dragCounter = 0

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

  const taskData = e.dataTransfer?.getData('application/json')
  if (taskData) {
    try {
      const task = JSON.parse(taskData)
      addTask(task)
    } catch (err) {
      console.error('[Pipeline] Failed to parse dropped task:', err)
    }
  }
}

const addTask = (task) => {
  const exists = pipeline.value.find(t => t.task_id === task.task_id)
  if (!exists) {
    pipeline.value.push({
      task_id: task.task_id,
      title: task.short_title || task.title || '未命名任务',
      description: task.description || task.ai_description || ''
    })
  }
}

const removeTask = (index) => {
  pipeline.value.splice(index, 1)
}

const onCardDragStart = (e, index) => {
  dragIndex.value = index
  e.dataTransfer.effectAllowed = 'move'
  e.dataTransfer.setData('text/plain', index.toString())
}

const onCardDragOver = (e, index) => {
  e.dataTransfer.dropEffect = 'move'
}

const onCardDrop = (e, targetIndex) => {
  e.preventDefault()
  const sourceIndex = dragIndex.value

  if (sourceIndex >= 0 && sourceIndex !== targetIndex) {
    const task = pipeline.value[sourceIndex]
    pipeline.value.splice(sourceIndex, 1)
    pipeline.value.splice(targetIndex, 0, task)
  }
  dragIndex.value = -1
}

const onCardDragEnd = () => {
  dragIndex.value = -1
}

const generatePrompt = async () => {
  if (pipeline.value.length < 2) return

  isGenerating.value = true
  try {
    const pipelineDesc = pipeline.value.map((task, index) => {
      return `步骤${index + 1}: ${task.title} (ID: ${task.task_id})`
    }).join('\n')

    promptText.value = `请将以下任务做成流水线，前一个任务的输出是下一任务的输入：

${pipelineDesc}

请按照以下要求执行：
1. 按顺序执行每个任务
2. 前一个任务的输出作为后一个任务的输入
3. 记录每个任务的执行状态和结果
4. 如果某个任务失败，停止整个流水线并报告错误`
  } catch (err) {
    console.error('[Pipeline] Failed to generate prompt:', err)
  } finally {
    isGenerating.value = false
  }
}

const createTask = async () => {
  if (!promptText.value.trim()) return

  isCreating.value = true
  try {
    emit('createTask', {
      prompt: promptText.value,
      pipeline: pipeline.value
    })
  } catch (err) {
    console.error('[Pipeline] Failed to create task:', err)
  } finally {
    isCreating.value = false
  }
}

const savePipeline = async () => {
  if (!props.pipelineTask || !promptText.value.trim()) return

  isSaving.value = true
  try {
    const definition = []
    for (let i = 0; i < pipeline.value.length; i++) {
      definition.push(pipeline.value[i].task_id)
    }
    emit('savePipeline', {
      taskId: props.pipelineTask.task_id,
      pipeline_definition: definition,
      prompt: promptText.value
    })
  } catch (err) {
    console.error('[Pipeline] Failed to save pipeline:', err)
  } finally {
    isSaving.value = false
  }
}
</script>

<style scoped>
.pipeline-editor {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-left: 1px solid rgba(0, 0, 0, 0.08);
  background: #ffffff;
  max-width: 50%;
}

.pipeline-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  background: #f9fafc;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
}

.pipeline-icon {
  font-size: 14px;
}

.pipeline-title {
  font-size: 11px;
  font-weight: 600;
  color: #757575;
  flex: 1;
}

.pipeline-close-btn {
  width: 20px;
  height: 20px;
  border: none;
  background: transparent;
  color: #999;
  font-size: 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: all 0.15s;
}

.pipeline-close-btn:hover {
  background: rgba(0, 0, 0, 0.06);
  color: #666;
}

.pipeline-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.pipeline-drop-zone {
  flex: 1;
  min-height: 0;
  border: 2px dashed rgba(0, 0, 0, 0.15);
  border-radius: 8px;
  background: #fafbfc;
  transition: all 0.2s;
  padding: 12px;
  overflow-y: auto;
}

.pipeline-drop-zone.drag-over {
  border-color: #4095fe;
  background: rgba(64, 149, 254, 0.05);
}

.pipeline-drop-zone.has-tasks {
  border-style: solid;
  border-color: rgba(0, 0, 0, 0.1);
  background: #fff;
}

.pipeline-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 80px;
  gap: 8px;
}

.empty-icon {
  font-size: 24px;
  opacity: 0.5;
}

.empty-text {
  font-size: 13px;
  color: #999;
  text-align: center;
}

.pipeline-tasks {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}

.pipeline-task-card {
  display: flex;
  align-items: center;
  gap: 4px;
}

.task-card-content {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 8px;
  cursor: grab;
  transition: all 0.2s;
  box-shadow: 0 2px 8px rgba(102, 126, 234, 0.3);
  min-width: 100px;
}

.task-card-content:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.task-card-content:active {
  cursor: grabbing;
}

.pipeline-task-card.dragging .task-card-content {
  opacity: 0.5;
  transform: scale(0.95);
}

.task-card-number {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.3);
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.task-card-title {
  font-size: 13px;
  color: #fff;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 120px;
}

.task-card-remove {
  width: 18px;
  height: 18px;
  border: none;
  background: rgba(255, 255, 255, 0.2);
  color: #fff;
  font-size: 14px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  transition: all 0.15s;
  flex-shrink: 0;
  margin-left: 2px;
}

.task-card-remove:hover {
  background: rgba(255, 255, 255, 0.4);
}

.pipeline-arrow {
  display: flex;
  align-items: center;
  padding: 0 2px;
}

.arrow-icon {
  font-size: 18px;
  color: #667eea;
  font-weight: bold;
}

.prompt-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-height: 0;
}

.prompt-header {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.prompt-label {
  font-size: 11px;
  font-weight: 600;
  color: #757575;
}

.prompt-textarea {
  flex: 1;
  width: 100%;
  padding: 10px;
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: 6px;
  font-size: 13px;
  line-height: 1.5;
  resize: none;
  min-height: 0;
  font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  transition: border-color 0.2s;
}

.prompt-textarea:focus {
  outline: none;
  border-color: #4095fe;
  box-shadow: 0 0 0 2px rgba(64, 149, 254, 0.15);
}

.prompt-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  flex-shrink: 0;
}

.action-btn {
  padding: 6px 16px;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.generate-btn {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
}

.generate-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.create-btn {
  background: linear-gradient(135deg, #2eb67d 0%, #26a69a 100%);
  color: #fff;
}

.create-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(46, 182, 125, 0.4);
}

.save-btn {
  background: linear-gradient(135deg, #4095fe 0%, #1d9bd1 100%);
  color: #fff;
}

.save-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(64, 149, 254, 0.4);
}
</style>
