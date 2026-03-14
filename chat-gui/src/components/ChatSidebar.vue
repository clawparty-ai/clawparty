<template>
  <div class="sidebar-shell">

    <!-- Left: org switcher rail -->
    <nav class="org-rail">
      <!-- 0. Join Party button -->
      <button
        class="new-group-rail-btn"
        :class="{ active: showJoinParty }"
        @click="toggleJoinParty"
        title="Join Party"
      >
        <span class="join-party-icon">🥂</span>
      </button>

      <!-- 1. Create group chat button -->
      <button
        class="new-group-rail-btn"
        :class="{ active: showPicker }"
        @click="togglePicker"
        title="New group chat"
      >
        <span class="new-group-rail-icon">#<span class="new-group-plus">+</span></span>
      </button>

      <!-- 2. Group Chats org icon (two lobsters) -->
      <div
        class="org-icon group-chats-icon"
        :class="{ active: activeOrg === 'groups' }"
        @click="activeOrg = 'groups'"
        title="Group Chats"
      >
        <span class="org-double-lobster">🦞🦞</span>
        <span class="org-active-bar" v-if="activeOrg === 'groups'"></span>
      </div>

      <div class="org-divider"></div>

      <!-- 3. My Agents (single lobster) -->
      <div
        class="org-icon"
        :class="{ active: activeOrg === 'agents' }"
        @click="activeOrg = 'agents'"
        title="My Agents"
      >
        <span class="org-emoji">🦞</span>
        <span class="org-active-bar" v-if="activeOrg === 'agents'"></span>
      </div>

      <div class="org-divider"></div>

      <!-- 4. One icon per mesh -->
      <div
        v-for="mesh in meshes"
        :key="mesh.name"
        class="org-icon"
        :class="{ active: activeOrg === mesh.name }"
        @click="handleSelectMesh(mesh.name)"
        :title="mesh.name"
      >
        <span class="org-letter org-letter-sm">{{ mesh.name ? mesh.name.slice(0, 2).toUpperCase() : '?' }}</span>
        <span class="org-online-dot" :class="{ online: mesh.agent?.connected }"></span>
        <span class="org-active-bar" v-if="activeOrg === mesh.name"></span>
      </div>

      <div class="org-rail-spacer"></div>

      <div class="org-rail-bottom-spacer"></div>

      <!-- Profile icon -->
      <div
        class="org-icon profile-icon"
        :title="currentMeshAgentUsername || 'My Profile'"
      >
        <span class="profile-letter">{{ (currentMeshAgentUsername || '?')[0].toUpperCase() }}</span>
      </div>
    </nav>

    <!-- Right: member list panel -->
    <aside class="sidebar-panel">
      <div class="panel-header">
        <span class="panel-title">{{
          activeOrg === 'agents' ? 'My Agents' :
          activeOrg === 'groups' ? 'Group Chats' :
          activeOrg
        }}</span>
      </div>

    <!-- Join Party modal -->
    <Teleport to="body">
      <div v-if="showJoinParty" class="modal-backdrop" @click.self="closeJoinParty">
        <div class="modal-dialog">
          <div class="modal-header">
            <span class="modal-title">Join Party</span>
            <button class="modal-close" @click="closeJoinParty">✕</button>
          </div>

          <div class="join-party-body">
            <label class="join-party-label">Registration URL</label>
            <input
              v-model="joinPartyUrl"
              class="search-input"
              placeholder="https://clawparty.flomesh.io:7779"
              :disabled="joinPartyLoading"
            />
            <div v-if="joinPartyError" class="join-party-error">{{ joinPartyError }}</div>
            <div v-if="joinPartySuccess" class="join-party-success">{{ joinPartySuccess }}</div>
          </div>

          <div class="modal-footer">
            <button class="modal-cancel-btn" @click="closeJoinParty" :disabled="joinPartyLoading">Cancel</button>
            <button
              class="modal-create-btn"
              :disabled="joinPartyLoading"
              @click="handleJoinParty"
            >{{ joinPartyLoading ? 'Joining...' : 'Join Party' }}</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Group picker modal (teleported to body to avoid overflow clipping) -->
    <Teleport to="body">
      <div v-if="showPicker" class="modal-backdrop" @click.self="closePicker">
        <div class="modal-dialog">
          <div class="modal-header">
            <span class="modal-title">New Group Chat</span>
            <button class="modal-close" @click="closePicker">✕</button>
          </div>

          <!-- Search filter -->
          <div class="modal-search">
            <input
              v-model="pickerSearch"
              class="search-input"
              placeholder="Search members..."
              autofocus
            />
          </div>

          <div class="modal-list">
            <!-- Mesh users -->
            <template v-if="filteredUsers.length > 0">
              <div class="modal-section-label">Members</div>
              <label
                v-for="user in filteredUsers"
                :key="'u-' + user.id"
                class="modal-item"
                :class="{ selected: pickerSelected.includes(user.username || user.name) }"
              >
                <input
                  type="checkbox"
                  :checked="pickerSelected.includes(user.username || user.name)"
                  @change="togglePickerUser(user.username || user.name)"
                />
                <div class="item-avatar" :style="{ background: getAvatarColor(user.username || user.name) }">
                  {{ (user.username || user.name)[0].toUpperCase() }}
                </div>
                <span class="item-name">{{ user.name }}</span>
                <span class="item-subname" v-if="user.username">{{ user.username }}</span>
                <span class="item-status" :class="{ online: user.online }"></span>
              </label>
            </template>

            <!-- Local agents -->
            <template v-if="filteredAgents.length > 0">
              <div class="modal-section-label">Local Agents</div>
              <label
                v-for="agent in filteredAgents"
                :key="'a-' + agent.id"
                class="modal-item"
                :class="{ selected: pickerSelected.includes(agent.id) }"
              >
                <input
                  type="checkbox"
                  :checked="pickerSelected.includes(agent.id)"
                  @change="togglePickerUser(agent.id)"
                />
                <div class="item-avatar openclaw-avatar">{{ agent.emoji }}</div>
                <span class="item-name">{{ agent.name }}</span>
                <span class="item-tag">agent</span>
              </label>
            </template>

            <div v-if="filteredUsers.length === 0 && filteredAgents.length === 0" class="modal-empty">
              <span v-if="!currentMesh">Not connected to a party yet.</span>
              <span v-else-if="users.length === 0">No other members in the party yet. They may still be connecting.</span>
              <span v-else>No members match your search.</span>
            </div>
          </div>

          <div class="modal-footer">
            <span class="modal-count">{{ pickerSelected.length }} selected</span>
            <button class="modal-cancel-btn" @click="closePicker">Cancel</button>
            <button
              class="modal-create-btn"
              :disabled="pickerSelected.length === 0"
              @click="handleCreateGroup"
            >Create Group</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Edit members modal -->
    <Teleport to="body">
      <div v-if="showEditMembers" class="modal-backdrop" @click.self="closeEditMembers">
        <div class="modal-dialog">
          <div class="modal-header">
            <span class="modal-title">Edit Members — {{ editingChat?.name }}</span>
            <button class="modal-close" @click="closeEditMembers">✕</button>
          </div>
          <div class="modal-search">
            <input v-model="editMembersSearch" class="search-input" placeholder="Search members..." autofocus />
          </div>
          <div class="modal-list">
            <template v-if="filteredEditUsers.length > 0">
              <div class="modal-section-label">Members</div>
              <label
                v-for="user in filteredEditUsers"
                :key="'eu-' + user.id"
                class="modal-item"
                :class="{ selected: editMembersSelected.includes(user.username || user.name) }"
              >
                <input
                  type="checkbox"
                  :checked="editMembersSelected.includes(user.username || user.name)"
                  @change="toggleEditMember(user.username || user.name)"
                />
                <div class="item-avatar" :style="{ background: getAvatarColor(user.username || user.name) }">
                  {{ (user.username || user.name)[0].toUpperCase() }}
                </div>
                <span class="item-name">{{ user.name }}</span>
                <span class="item-subname" v-if="user.username">{{ user.username }}</span>
              </label>
            </template>
            <template v-if="filteredEditAgents.length > 0">
              <div class="modal-section-label">Local Agents</div>
              <label
                v-for="agent in filteredEditAgents"
                :key="'ea-' + agent.id"
                class="modal-item"
                :class="{ selected: editMembersSelected.includes(agent.id) }"
              >
                <input
                  type="checkbox"
                  :checked="editMembersSelected.includes(agent.id)"
                  @change="toggleEditMember(agent.id)"
                />
                <div class="item-avatar openclaw-avatar">{{ agent.emoji }}</div>
                <span class="item-name">{{ agent.name }}</span>
                <span class="item-tag">agent</span>
              </label>
            </template>
          </div>
          <div class="modal-footer">
            <span class="modal-count">{{ editMembersSelected.length }} selected</span>
            <button class="modal-cancel-btn" @click="closeEditMembers">Cancel</button>
            <button class="modal-create-btn" :disabled="editMembersSelected.length === 0" @click="handleUpdateMembers">Save</button>
          </div>
        </div>
      </div>
    </Teleport>

      <div class="panel-list">
        <!-- Group Chats view — all groups across meshes -->
        <template v-if="activeOrg === 'groups'">
          <template v-if="groupChats.length > 0">
            <div
              v-for="chat in groupChats"
              :key="chat.id"
              class="group-chat-entry"
            >
              <!-- Header row -->
              <div
                class="panel-item group-chat-header"
                :class="{ active: getChatIndex(chat.id) === activeChat }"
              >
                <div class="item-hash" @click.stop="toggleExpand(chat.id)" :title="expandedGroups.has(chat.id) ? 'Collapse members' : 'Expand members'">{{ expandedGroups.has(chat.id) ? '▼' : '▶' }}</div>
                <!-- Rename inline editor -->
                <template v-if="renamingChatId === chat.id">
                  <input
                    class="group-rename-input"
                    v-model="renameValue"
                    @keyup.enter="submitRename(chat)"
                    @keyup.escape="cancelRename"
                    @blur="submitRename(chat)"
                    ref="renameInputRef"
                  />
                </template>
                <template v-else>
                  <span class="item-name" @click="$emit('select', getChatIndex(chat.id))">{{ chat.name }}</span>
                </template>
                <span v-if="chat.updated > 0 && renamingChatId !== chat.id" class="unread-badge">{{ chat.updated > 99 ? '99+' : chat.updated }}</span>
                <button class="group-action-btn" @click.stop="startRename(chat)" title="Rename">✎</button>
                <button class="group-action-btn" @click.stop="openEditMembers(chat)" title="Edit members">👥</button>
              </div>
              <!-- Expanded members list -->
              <div v-if="expandedGroups.has(chat.id)" class="group-members-list">
                <div
                  v-for="member in (chat.members || [])"
                  :key="member"
                  class="group-member-item"
                >
                  <div class="member-avatar" :style="{ background: getAvatarColor(member) }">{{ member[0].toUpperCase() }}</div>
                  <span class="member-name">{{ resolveEpDisplayName(member) }}</span>
                </div>
                <div v-if="!chat.members || chat.members.length === 0" class="group-member-empty">No members</div>
              </div>
            </div>
          </template>
          <div v-else class="panel-empty">No group chats yet</div>
        </template>

        <!-- My Agents -->
        <template v-else-if="activeOrg === 'agents'">
          <div
            v-for="agent in openclawAgents"
            :key="agent.id"
            class="panel-item"
            :class="{ active: getChatIndex(agent.id, true) === activeChat }"
            @click="$emit('selectOpenclaw', agent)"
          >
            <div class="item-avatar openclaw-avatar">{{ agent.emoji }}</div>
            <span class="item-name">{{ agent.name }}</span>
          </div>
          <div v-if="openclawAgents.length === 0" class="panel-empty-state">
            <div class="panel-empty-state-title">No local agents</div>
            <div class="panel-empty-state-hint">openclaw is not installed locally. You can still interact with remote openclaw agents via group chat.</div>
          </div>
        </template>

        <!-- Mesh: users + groups -->
        <template v-else>
          <!-- Direct message users (one entry per EP) -->
          <div
            v-for="user in users"
            :key="'ep-' + user.id"
            class="panel-item"
            :class="{ active: getChatIndex(user.username || user.name) === activeChat }"
            @click="selectUser(user)"
          >
            <div class="item-avatar" :style="{ background: getAvatarColor(user.username || user.name) }">
              {{ (user.username || user.name)[0].toUpperCase() }}
            </div>
            <span class="item-name">{{ user.name }}</span>
            <span class="item-subname" v-if="user.username">{{ user.username }}</span>
            <span v-if="getChatUpdated(user.username || user.name) > 0" class="unread-badge">{{ getChatUpdated(user.username || user.name) > 99 ? '99+' : getChatUpdated(user.username || user.name) }}</span>
            <span class="item-status" :class="{ online: user.online }"></span>
          </div>

          <!-- Non-group chats: only show peers not already in the users list -->
          <div
            v-for="chat in dmChats"
            v-show="!users.find(u => u.name === chat.name)"
            :key="chat.id"
            class="panel-item"
            :class="{ active: getChatIndex(chat.id) === activeChat }"
            @click="$emit('select', getChatIndex(chat.id))"
          >
            <div class="item-avatar" :style="{ background: getAvatarColor(chat.name) }">
              {{ chat.name[0].toUpperCase() }}
            </div>
            <span class="item-name">{{ chat.displayName || chat.name }}</span>
            <span v-if="chat.updated > 0" class="unread-badge">{{ chat.updated > 99 ? '99+' : chat.updated }}</span>
          </div>
        </template>
      </div>
    </aside>

  </div>
</template>

<script setup>
import { ref, computed, inject, watch } from 'vue'
import { getAvatarColor } from '../utils/avatar'

const props = defineProps({
  chats: { type: Array, required: true },
  activeChat: { type: Number, default: null }
})

const currentMesh = inject('currentMesh')
const meshes = inject('meshes')
const openclawAgents = inject('openclawAgents')
const switchMesh = inject('switchMesh')
const users = inject('users')
const selectUser = inject('selectUser')
const createGroupChat = inject('createGroupChat')
const renameGroupChat = inject('renameGroupChat')
const updateGroupMembers = inject('updateGroupMembers')
const currentMeshAgentUsername = inject('currentMeshAgentUsername')
const joinParty = inject('joinParty')
const resolveEpDisplayName = inject('resolveEpDisplayName')
const localOpenclawAvailable = inject('localOpenclawAvailable')

// Group expand / rename state
const expandedGroups = ref(new Set())
const renamingChatId = ref(null)
const renameValue = ref('')
const renameInputRef = ref(null)

const toggleExpand = (chatId) => {
  const s = new Set(expandedGroups.value)
  if (s.has(chatId)) s.delete(chatId)
  else s.add(chatId)
  expandedGroups.value = s
}

const startRename = (chat) => {
  renamingChatId.value = chat.id
  renameValue.value = chat.name
  // Focus input on next tick
  setTimeout(() => {
    if (renameInputRef.value) {
      const el = Array.isArray(renameInputRef.value) ? renameInputRef.value[0] : renameInputRef.value
      el && el.focus && el.focus()
    }
  }, 30)
}

const submitRename = async (chat) => {
  if (renamingChatId.value !== chat.id) return
  const newName = renameValue.value.trim()
  renamingChatId.value = null
  if (newName && newName !== chat.name) {
    await renameGroupChat(chat, newName)
  }
}

const cancelRename = () => {
  renamingChatId.value = null
  renameValue.value = ''
}

// Edit members state
const showEditMembers = ref(false)
const editingChat = ref(null)
const editMembersSelected = ref([])
const editMembersSearch = ref('')

const filteredEditUsers = computed(() => {
  const q = editMembersSearch.value.trim().toLowerCase()
  // Exclude the creator (always a member, can't be removed)
  const nonCreator = users.value.filter(u => (u.username || u.name) !== editingChat.value?.creator)
  if (!q) return nonCreator
  return nonCreator.filter(u =>
    u.name.toLowerCase().includes(q) ||
    (u.username && u.username.toLowerCase().includes(q))
  )
})

const filteredEditAgents = computed(() => {
  const q = editMembersSearch.value.trim().toLowerCase()
  if (!q) return openclawAgents.value
  return openclawAgents.value.filter(a => a.name.toLowerCase().includes(q))
})

const openEditMembers = (chat) => {
  editingChat.value = chat
  // Pre-select current members (excluding creator who is always included)
  editMembersSelected.value = (chat.members || []).filter(m => m !== chat.creator)
  editMembersSearch.value = ''
  showEditMembers.value = true
}

const closeEditMembers = () => {
  showEditMembers.value = false
  editingChat.value = null
  editMembersSelected.value = []
  editMembersSearch.value = ''
}

const toggleEditMember = (name) => {
  const idx = editMembersSelected.value.indexOf(name)
  if (idx === -1) editMembersSelected.value.push(name)
  else editMembersSelected.value.splice(idx, 1)
}

const handleUpdateMembers = async () => {
  if (!editingChat.value) return
  // Always include the creator
  const members = [editingChat.value.creator, ...editMembersSelected.value.filter(m => m !== editingChat.value.creator)]
  await updateGroupMembers(editingChat.value, members)
  closeEditMembers()
}

// Active org
const activeOrg = ref('agents')

const emit = defineEmits(['select', 'selectOpenclaw', 'changeOrg'])

watch(currentMesh, (val) => {
  if (val && activeOrg.value !== 'agents') activeOrg.value = val
}, { immediate: true })

watch(activeOrg, (val) => {
  emit('changeOrg', val)
})

const handleSelectMesh = async (meshName) => {
  activeOrg.value = meshName
  if (meshName !== currentMesh.value) {
    await switchMesh(meshName)
  }
}

// Chats split by type
const groupChats = computed(() =>
  props.chats.filter(c => c.isGroup && !c.isOpenclaw)
)
const dmChats = computed(() =>
  props.chats.filter(c => !c.isGroup && !c.isOpenclaw)
)

const getChatUpdated = (userName) => {
  const chat = props.chats.find(c => c.name === userName && !c.isGroup && !c.isOpenclaw)
  return chat?.updated || 0
}

const getUserDisplayName = (userName) => {
  const chat = props.chats.find(c => c.name === userName && !c.isGroup && !c.isOpenclaw)
  return chat?.displayName || userName
}

const getChatIndex = (chatId, isOpenclaw = false) => {
  return props.chats.findIndex(c => c.id === chatId && Boolean(c.isOpenclaw) === isOpenclaw)
}

// Group picker state
const showPicker = ref(false)
const pickerSelected = ref([])
const pickerSearch = ref('')

const togglePicker = () => {
  showPicker.value = !showPicker.value
  if (!showPicker.value) {
    pickerSelected.value = []
    pickerSearch.value = ''
  }
}

const closePicker = () => {
  showPicker.value = false
  pickerSelected.value = []
  pickerSearch.value = ''
}

const filteredUsers = computed(() => {
  const q = pickerSearch.value.trim().toLowerCase()
  if (!q) return users.value
  return users.value.filter(u =>
    u.name.toLowerCase().includes(q) ||
    (u.username && u.username.toLowerCase().includes(q))
  )
})

const filteredAgents = computed(() => {
  const q = pickerSearch.value.trim().toLowerCase()
  if (!q) return openclawAgents.value
  return openclawAgents.value.filter(a => a.name.toLowerCase().includes(q))
})

const togglePickerUser = (name) => {
  const idx = pickerSelected.value.indexOf(name)
  if (idx === -1) pickerSelected.value.push(name)
  else pickerSelected.value.splice(idx, 1)
}

// Join Party state
const showJoinParty = ref(false)
const joinPartyUrl = ref('https://join.clawparty.ai')
const joinPartyLoading = ref(false)
const joinPartyError = ref('')
const joinPartySuccess = ref('')

const toggleJoinParty = () => {
  showJoinParty.value = !showJoinParty.value
  if (!showJoinParty.value) {
    joinPartyError.value = ''
    joinPartySuccess.value = ''
  }
}

const closeJoinParty = () => {
  if (joinPartyLoading.value) return
  showJoinParty.value = false
  joinPartyError.value = ''
  joinPartySuccess.value = ''
}

const handleJoinParty = async () => {
  if (joinPartyLoading.value) return
  joinPartyLoading.value = true
  joinPartyError.value = ''
  joinPartySuccess.value = ''
  try {
    await joinParty(joinPartyUrl.value)
    joinPartySuccess.value = 'Successfully joined the party!'
    setTimeout(() => {
      closeJoinParty()
    }, 1500)
  } catch (err) {
    const msg = err?.response?.data?.message || err?.message || 'Failed to join party'
    joinPartyError.value = msg
  } finally {
    joinPartyLoading.value = false
  }
}

const handleCreateGroup = async () => {
  if (pickerSelected.value.length === 0) return
  // pickerSelected stores username (for EP users) or agent name (for agents)
  const fromUsers = users.value.filter(u => pickerSelected.value.includes(u.username || u.name))
  const fromAgents = openclawAgents.value.filter(a => pickerSelected.value.includes(a.id))
  const selectedObjs = [
    // EP users: pass username as the member identifier
    ...fromUsers.map(u => ({ name: u.username || u.name })),
    // Agents: use agent id (e.g. "video-agent") so it matches getLocalAgentNames() output
    ...fromAgents.map(a => ({ name: a.id }))
  ]
  // Build a human-friendly group name using display names
  const displayNames = [
    ...fromUsers.map(u => u.name || u.username),
    ...fromAgents.map(a => a.name)
  ]
  const groupName = displayNames.join(', ')
  await createGroupChat(selectedObjs, groupName)
  closePicker()
  activeOrg.value = 'groups'
}
</script>

<style scoped>
.sidebar-shell {
  display: flex;
  height: 100%;
  width: var(--sidebar-width);
  flex-shrink: 0;
}

/* ── Org rail ── */
.org-rail {
  width: 64px;
  background: var(--slack-aubergine);
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 0;
  gap: 6px;
  overflow-y: auto;
  flex-shrink: 0;
}

.org-divider {
  width: 32px;
  height: 1px;
  background: rgba(255, 255, 255, 0.2);
  margin: 4px 0;
}

.org-icon {
  position: relative;
  width: 40px;
  height: 40px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.15);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: border-radius 0.15s, background 0.15s;
  flex-shrink: 0;
}

.org-icon:hover {
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.25);
}

.org-icon.active {
  border-radius: 14px;
  background: var(--slack-purple);
}

.org-emoji { font-size: 20px; }

.org-letter {
  color: #fff;
  font-size: 16px;
  font-weight: 700;
}

.org-letter-sm {
  font-size: 12px;
  letter-spacing: 0.5px;
}

.org-online-dot {
  position: absolute;
  bottom: 2px;
  right: 2px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #ccc;
  border: 1.5px solid var(--slack-aubergine);
}

.org-online-dot.online { background: var(--slack-green); }

.org-active-bar {
  position: absolute;
  left: -8px;
  top: 50%;
  transform: translateY(-50%);
  width: 4px;
  height: 24px;
  background: #fff;
  border-radius: 0 3px 3px 0;
}

/* ── Panel ── */
.sidebar-panel {
  flex: 1;
  background: var(--slack-purple);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-header {
  height: 48px;
  padding: 0 12px 0 16px;
  display: flex;
  align-items: center;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  flex-shrink: 0;
}

.panel-title {
  flex: 1;
  color: #fff;
  font-size: 15px;
  font-weight: 700;
  text-transform: capitalize;
}

/* ── Group Chats org icon accent ── */
.group-chats-icon {
  background: rgba(255, 112, 67, 0.2);
}

.group-chats-icon:hover,
.group-chats-icon.active {
  background: rgba(255, 112, 67, 0.38);
}

.org-double-lobster {
  font-size: 13px;
  line-height: 1;
  letter-spacing: -3px;
}

/* ── Panel empty state ── */
.panel-empty {
  padding: 32px 16px;
  text-align: center;
  color: rgba(255, 255, 255, 0.35);
  font-size: 13px;
}

.panel-empty-state {
  padding: 24px 16px;
  text-align: center;
}

.panel-empty-state-title {
  color: rgba(255, 255, 255, 0.45);
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 8px;
}

.panel-empty-state-hint {
  color: rgba(255, 255, 255, 0.28);
  font-size: 12px;
  line-height: 1.5;
}

/* ── New group button in org-rail ── */
.new-group-rail-btn {
  position: relative;
  width: 40px;
  height: 40px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.15);
  border: none;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: border-radius 0.15s, background 0.15s;
  flex-shrink: 0;
}

.new-group-rail-btn:hover,
.new-group-rail-btn.active {
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.3);
}

.new-group-rail-icon {
  color: #fff;
  font-size: 15px;
  font-weight: 700;
  line-height: 1;
  letter-spacing: -1px;
}

.new-group-plus {
  font-size: 11px;
  vertical-align: super;
}

/* ── Rail icon tooltip (text comes from title) ── */
.new-group-rail-btn[title]::before,
.org-icon[title]::before {
  content: '';
  position: absolute;
  left: calc(100% + 4px);
  top: 50%;
  transform: translateY(-50%);
  border: 5px solid transparent;
  border-right-color: #1a1d21;
  opacity: 0;
  transition: opacity 0.12s;
  pointer-events: none;
  z-index: 101;
}

.new-group-rail-btn[title]::after,
.org-icon[title]::after {
  content: attr(title);
  position: absolute;
  left: calc(100% + 10px);
  top: 50%;
  transform: translateY(-50%);
  background: #1a1d21;
  color: #fff;
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  padding: 6px 10px;
  border-radius: 6px;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.4);
  opacity: 0;
  transition: opacity 0.12s;
  pointer-events: none;
  z-index: 100;
}

.new-group-rail-btn[title]:hover::before,
.new-group-rail-btn[title]:hover::after,
.org-icon[title]:hover::before,
.org-icon[title]:hover::after {
  opacity: 1;
}

/* ── Modal backdrop ── */
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-dialog {
  background: #1a1d21;
  border-radius: 12px;
  width: 460px;
  max-width: 92vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  padding: 18px 20px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
}

.modal-title {
  flex: 1;
  color: #fff;
  font-size: 17px;
  font-weight: 700;
}

.modal-close {
  width: 28px;
  height: 28px;
  background: transparent;
  border: none;
  color: rgba(255, 255, 255, 0.5);
  font-size: 16px;
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.1s, color 0.1s;
}

.modal-close:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.modal-search {
  padding: 12px 16px 8px;
  flex-shrink: 0;
}

.search-input {
  width: 100%;
  box-sizing: border-box;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 7px;
  color: #fff;
  font-size: 14px;
  outline: none;
  transition: border-color 0.15s;
}

.search-input::placeholder { color: rgba(255, 255, 255, 0.35); }

.search-input:focus {
  border-color: rgba(255, 255, 255, 0.35);
  background: rgba(255, 255, 255, 0.11);
}

.modal-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px 8px;
}

.modal-section-label {
  padding: 8px 10px 4px;
  color: rgba(255, 255, 255, 0.4);
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.7px;
}

.modal-item {
  display: flex;
  align-items: center;
  padding: 5px 10px;
  height: 42px;
  cursor: pointer;
  border-radius: 7px;
  margin-bottom: 1px;
  transition: background 0.1s;
}

.modal-item:hover { background: rgba(255, 255, 255, 0.07); }

.modal-item.selected { background: rgba(255, 255, 255, 0.12); }

.modal-item input[type="checkbox"] {
  margin-right: 10px;
  width: 15px;
  height: 15px;
  cursor: pointer;
  accent-color: var(--slack-green);
  flex-shrink: 0;
}

.modal-empty {
  padding: 32px 0;
  text-align: center;
  color: rgba(255, 255, 255, 0.35);
  font-size: 14px;
}

.modal-footer {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  gap: 10px;
  flex-shrink: 0;
}

.modal-count {
  flex: 1;
  color: rgba(255, 255, 255, 0.45);
  font-size: 13px;
}

.modal-cancel-btn {
  padding: 7px 16px;
  background: rgba(255, 255, 255, 0.1);
  border: none;
  border-radius: 6px;
  color: rgba(255, 255, 255, 0.8);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.1s;
}

.modal-cancel-btn:hover { background: rgba(255, 255, 255, 0.18); }

.modal-create-btn {
  padding: 7px 18px;
  background: var(--slack-green);
  border: none;
  border-radius: 6px;
  color: #fff;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.1s;
}

.modal-create-btn:hover { opacity: 0.85; }

.modal-create-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.item-tag {
  font-size: 10px;
  color: rgba(255, 255, 255, 0.4);
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
  padding: 1px 5px;
  margin-left: 4px;
  flex-shrink: 0;
}

/* ── Panel list ── */
.panel-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.list-divider {
  display: flex;
  align-items: center;
  padding: 8px 12px 4px;
  color: rgba(255, 255, 255, 0.4);
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.6px;
  gap: 8px;
}

.list-divider::after {
  content: '';
  flex: 1;
  height: 1px;
  background: rgba(255, 255, 255, 0.1);
}

.panel-item {
  display: flex;
  align-items: center;
  padding: 5px 12px;
  height: 40px;
  cursor: pointer;
  border-radius: 6px;
  margin: 0 6px 2px;
  transition: background 0.1s;
}

.panel-item:hover { background: rgba(255, 255, 255, 0.1); }

.panel-item.active { background: var(--slack-aubergine); }

.item-avatar {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: 700;
  font-size: 13px;
  margin-right: 10px;
  flex-shrink: 0;
}

.openclaw-avatar {
  background: linear-gradient(135deg, #ff6b6b, #ffa500);
  font-size: 17px;
}

.item-hash {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.5);
  font-size: 11px;
  font-weight: 700;
  margin-right: 6px;
  flex-shrink: 0;
  cursor: pointer;
  transition: color 0.15s;
}

.item-hash:hover {
  color: rgba(255, 255, 255, 0.9);
}

.panel-item.active .item-hash { color: rgba(255, 255, 255, 0.9); }

.item-name {
  flex: 1;
  color: #e8e8e8;
  font-size: 14px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-subname {
  font-size: 11px;
  color: #aaa;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 80px;
  flex-shrink: 0;
}

.panel-item.active .item-name {
  color: #fff;
  font-weight: 700;
}

.item-status {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #ccc;
  flex-shrink: 0;
}

.item-status.online { background: var(--slack-green); }

.unread-badge {
  min-width: 18px;
  height: 18px;
  background: #e01e5a;
  border-radius: 9px;
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 5px;
  flex-shrink: 0;
}

.org-rail-spacer {
  flex: 1;
}

.org-rail-bottom-spacer {
  width: 32px;
  height: 1px;
  background: rgba(255, 255, 255, 0.2);
  margin: 4px 0;
}

.profile-icon {
  position: relative;
  background: rgba(255, 255, 255, 0.2);
  cursor: default;
}

.profile-icon:hover {
  background: rgba(255, 255, 255, 0.3);
}

.profile-letter {
  color: #fff;
  font-size: 16px;
  font-weight: 700;
}

.join-party-icon {
  font-size: 20px;
  line-height: 1;
}

.join-party-body {
  padding: 16px 20px 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.join-party-label {
  color: rgba(255, 255, 255, 0.7);
  font-size: 13px;
  font-weight: 600;
}

.join-party-error {
  color: #e01e5a;
  font-size: 13px;
  padding: 4px 0;
}

.join-party-success {
  color: var(--slack-green);
  font-size: 13px;
  padding: 4px 0;
}

@media (max-width: 768px) {
  .sidebar-shell { 
    width: 100%; 
    height: 48px; 
    position: fixed;
    top: 0;
    left: 0;
    z-index: 100;
  }
  .org-rail { 
    flex-direction: row; 
    width: 100%; 
    height: 48px; 
    padding: 0 8px;
    flex-shrink: 0; 
    overflow-x: auto;
    overflow-y: hidden;
  }
  .sidebar-panel { 
    display: none;
  }
  .org-active-bar { display: none; }
  .org-divider {
    width: 1px;
    height: 24px;
    margin: 0 4px;
    flex-shrink: 0;
  }
  .org-icon {
    flex-shrink: 0;
  }
  .org-rail-spacer {
    display: none;
  }
}

/* Group chat expand / rename */
.group-chat-entry {
  display: flex;
  flex-direction: column;
}

.group-chat-header {
  display: flex;
  align-items: center;
  gap: 4px;
}

.group-action-btn {
  background: none;
  border: none;
  color: #aaa;
  cursor: pointer;
  font-size: 12px;
  padding: 0 3px;
  flex-shrink: 0;
  line-height: 1;
}
.group-action-btn:hover { color: #fff; }

.group-rename-input {
  flex: 1;
  background: #222;
  border: 1px solid #555;
  border-radius: 4px;
  color: #fff;
  font-size: 13px;
  padding: 2px 6px;
  outline: none;
  min-width: 0;
}

.group-members-list {
  padding: 4px 0 4px 28px;
  background: rgba(0,0,0,0.15);
}

.group-member-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 8px;
}

.member-avatar {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: 700;
  color: #fff;
  flex-shrink: 0;
}

.member-name {
  font-size: 12px;
  color: #ccc;
}

.group-member-empty {
  font-size: 12px;
  color: #666;
  padding: 2px 8px;
}
</style>
