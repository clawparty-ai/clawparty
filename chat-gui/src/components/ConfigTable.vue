<template>
  <div class="config-table-wrapper">
    <table class="config-table">
      <thead>
        <tr>
          <th v-for="col in columns" :key="col.key">{{ col.header }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(row, ri) in localRows" :key="row.peer">
          <td
            v-for="col in columns"
            :key="col.key"
            :class="{
              'editing': editingCell.ri === ri && editingCell.key === col.key,
              'readonly': col.key === 'peer',
              'long-text': longTextFields.has(col.key)
            }"
            @click="startEdit(ri, col.key)"
          >
            <template v-if="editingCell.ri === ri && editingCell.key === col.key">
              <textarea
                v-if="longTextFields.has(col.key)"
                ref="editInputRef"
                v-model="editingCell.value"
                @keydown="handleEditKeydown"
                @blur="commitEdit"
                class="cell-editor cell-textarea"
                rows="3"
              ></textarea>
              <input
                v-else
                ref="editInputRef"
                v-model="editingCell.value"
                @keydown="handleEditKeydown"
                @blur="commitEdit"
                class="cell-editor"
              />
            </template>
            <template v-else>
              <span class="cell-display" :title="fullValue(row, col.key)">{{ displayValue(row, col.key) }}</span>
            </template>
          </td>
        </tr>
      </tbody>
    </table>
    <div v-if="statusMsg" class="config-status" :class="statusType">{{ statusMsg }}</div>
  </div>
</template>

<script setup>
import { reactive, ref, nextTick, watch } from 'vue'
import { chatService } from '../services/chatService.js'

const props = defineProps({
  rows: { type: Array, required: true },
  meshName: { type: String, required: true }
})

const columns = [
  { key: 'peer',            header: 'peer' },
  { key: 'peerName',        header: 'peer_name' },
  { key: 'autoReply',       header: 'auto_reply' },
  { key: 'autoReplyAgent',  header: 'auto_reply_agent' },
  { key: 'peerAgentName',   header: 'peer_agent_name' },
  { key: 'credit',          header: 'credit' },
  { key: 'isBlocked',       header: 'is_blocked' },
  { key: 'run',             header: 'run' },
  { key: 'muted',           header: 'muted' },
  { key: 'thinkingTime',    header: 'thinking_time' },
  { key: 'filterChain',     header: 'filter_chain' },
  { key: 'sendFilterChain', header: 'send_filter_chain' },
  { key: 'peerProfile',     header: 'peer_profile' },
  { key: 'shortContext',    header: 'short_context' },
  { key: 'longContext',     header: 'long_context' },
]

const longTextFields = new Set(['peerProfile', 'shortContext', 'longContext'])

// Local mutable copy of rows so we can update cells in-place
const localRows = reactive(props.rows.map(r => ({ ...r })))

watch(() => props.rows, (newRows) => {
  localRows.splice(0, localRows.length, ...newRows.map(r => ({ ...r })))
})

const editingCell = reactive({ ri: -1, key: '', value: '', original: '' })
const editInputRef = ref(null)
const statusMsg = ref('')
const statusType = ref('')

function rawValue(row, key) {
  let val = row[key]
  if (val === undefined || val === null) val = ''
  if (typeof val === 'boolean') val = val ? '1' : '0'
  return String(val)
}

function fullValue(row, key) {
  return rawValue(row, key)
}

function displayValue(row, key) {
  const val = rawValue(row, key)
  if (longTextFields.has(key) && val.length > 60) return val.slice(0, 60) + '...'
  return val
}

function startEdit(ri, key) {
  if (key === 'peer') return
  // commit any open edit first
  if (editingCell.ri >= 0) commitEdit()
  const row = localRows[ri]
  const val = rawValue(row, key)
  editingCell.ri = ri
  editingCell.key = key
  editingCell.value = val
  editingCell.original = val
  nextTick(() => {
    const el = Array.isArray(editInputRef.value) ? editInputRef.value[0] : editInputRef.value
    if (el) { el.focus(); el.select?.() }
  })
}

function handleEditKeydown(e) {
  if (e.key === 'Escape') {
    e.preventDefault()
    cancelEdit()
  } else if (e.key === 'Enter') {
    // For textarea: require Ctrl+Enter to commit; plain Enter is a newline
    if (longTextFields.has(editingCell.key)) {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault()
        commitEdit()
      }
    } else {
      e.preventDefault()
      commitEdit()
    }
  }
}

function cancelEdit() {
  editingCell.ri = -1
  editingCell.key = ''
  editingCell.value = ''
  editingCell.original = ''
}

function parseValue(str) {
  if (str === 'true') return true
  if (str === 'false') return false
  if (str === '1') return 1
  if (str === '0') return 0
  const n = Number(str)
  if (!isNaN(n) && str.trim() !== '') return n
  return str
}

async function commitEdit() {
  const { ri, key, value, original } = editingCell
  if (ri < 0 || !key) return

  cancelEdit()

  if (value === original) return

  const row = localRows[ri]
  if (!row) return

  const peer = row.peer
  const parsed = parseValue(value)

  // Optimistic update
  row[key] = parsed

  try {
    await chatService.updatePeerConfig(props.meshName, peer, { [key]: parsed })
    showStatus(`Updated ${peer} · ${key} = ${value}`, 'success')
  } catch (e) {
    // Revert on failure
    row[key] = parseValue(original)
    showStatus(`Error: ${e?.message || String(e)}`, 'error')
  }
}

function showStatus(msg, type) {
  statusMsg.value = msg
  statusType.value = type
  setTimeout(() => { statusMsg.value = '' }, 3000)
}
</script>

<style scoped>
.config-table-wrapper {
  overflow-x: auto;
  margin: 4px 0;
}

.config-table {
  border-collapse: collapse;
  font-size: 13px;
  table-layout: auto;
  white-space: nowrap;
}

.config-table th,
.config-table td {
  border: 1px solid #bbb;
  padding: 4px 8px;
  text-align: left;
}

.config-table th {
  background: #e0e0e0;
  font-weight: 600;
  white-space: nowrap;
}

.config-table tr:nth-child(even) td {
  background: #f5f5f5;
}

.config-table td:not(.readonly):hover {
  background: #e3f2fd;
  cursor: cell;
}

.config-table td.editing {
  padding: 0;
  background: #fff;
  min-width: 120px;
}

.config-table td.long-text .cell-display {
  display: inline-block;
  max-width: 20ch;
  overflow: hidden;
  text-overflow: ellipsis;
  vertical-align: bottom;
}

.config-table td.readonly {
  color: #333;
  font-weight: 600;
}

.cell-editor {
  width: 100%;
  border: 2px solid #1976d2;
  padding: 3px 6px;
  font-size: 13px;
  font-family: inherit;
  outline: none;
  box-sizing: border-box;
  background: #fff;
  min-width: 100px;
}

.cell-textarea {
  resize: vertical;
  min-height: 64px;
  white-space: pre-wrap;
  word-break: break-all;
}

.config-status {
  margin-top: 4px;
  font-size: 12px;
  padding: 2px 6px;
  border-radius: 3px;
}

.config-status.success { color: #16a34a; }
.config-status.error   { color: #dc2626; }
</style>
