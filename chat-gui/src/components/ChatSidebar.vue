<template>
  <div class="sidebar-shell">

    <!-- Left: org switcher rail -->
    <nav class="org-rail">
      <!-- My Agents org -->
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

      <!-- One icon per mesh -->
      <div
        v-for="mesh in meshes"
        :key="mesh.name"
        class="org-icon"
        :class="{ active: activeOrg === mesh.name }"
        @click="handleSelectMesh(mesh.name)"
        :title="mesh.name"
      >
        <span class="org-letter">{{ mesh.name[0].toUpperCase() }}</span>
        <span class="org-online-dot" :class="{ online: mesh.agent?.connected }"></span>
        <span class="org-active-bar" v-if="activeOrg === mesh.name"></span>
      </div>
    </nav>

    <!-- Right: member list panel -->
    <aside class="sidebar-panel">
      <div class="panel-header">
        <span class="panel-title">{{ activeOrg === 'agents' ? 'My Agents' : activeOrg }}</span>
      </div>

      <div class="panel-list">
        <!-- My Agents -->
        <template v-if="activeOrg === 'agents'">
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
        </template>

        <!-- Mesh users -->
        <template v-else>
          <div
            v-for="user in users"
            :key="'user-' + user.name"
            class="panel-item"
            :class="{ active: getChatIndex(user.name) === activeChat }"
            @click="selectUser(user)"
          >
            <div class="item-avatar" :style="{ background: getAvatarColor(user.name) }">
              {{ user.name[0].toUpperCase() }}
            </div>
            <span class="item-name">{{ user.name }}</span>
            <span class="item-status" :class="{ online: user.endpoints?.instances?.[0]?.online }"></span>
          </div>

          <div
            v-for="chat in filteredChats"
            :key="chat.id"
            v-show="!chat.isOpenclaw"
            class="panel-item"
            :class="{ active: getChatIndex(chat.id) === activeChat }"
            @click="$emit('select', getChatIndex(chat.id))"
          >
            <div class="item-avatar" :style="{ background: getAvatarColor(chat.name) }">
              {{ chat.name[0].toUpperCase() }}
            </div>
            <span class="item-name">{{ chat.name }}</span>
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

defineEmits(['select', 'selectOpenclaw'])

const currentMesh = inject('currentMesh')
const meshes = inject('meshes')
const openclawAgents = inject('openclawAgents')
const switchMesh = inject('switchMesh')
const users = inject('users')
const selectUser = inject('selectUser')

// Active org: 'agents' or a mesh name
const activeOrg = ref('agents')

// When currentMesh changes externally (e.g. on load), sync activeOrg
watch(currentMesh, (val) => {
  if (val && activeOrg.value !== 'agents') activeOrg.value = val
}, { immediate: true })

const handleSelectMesh = async (meshName) => {
  activeOrg.value = meshName
  if (meshName !== currentMesh.value) {
    await switchMesh(meshName)
  }
}

const filteredChats = computed(() => props.chats)

const getChatIndex = (chatId, isOpenclaw = false) => {
  return props.chats.findIndex(c => c.id === chatId && Boolean(c.isOpenclaw) === isOpenclaw)
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

.org-emoji {
  font-size: 20px;
}

.org-letter {
  color: #fff;
  font-size: 16px;
  font-weight: 700;
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

.org-online-dot.online {
  background: var(--slack-green);
}

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
  padding: 0 16px;
  display: flex;
  align-items: center;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  flex-shrink: 0;
}

.panel-title {
  color: #fff;
  font-size: 15px;
  font-weight: 700;
  text-transform: capitalize;
}

.panel-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
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

.panel-item:hover {
  background: rgba(255, 255, 255, 0.1);
}

.panel-item.active {
  background: var(--slack-aubergine);
}

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

.item-name {
  flex: 1;
  color: #e8e8e8;
  font-size: 14px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
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

.item-status.online {
  background: var(--slack-green);
}

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

@media (max-width: 768px) {
  .sidebar-shell {
    width: 100%;
    height: 48px;
  }
  .org-rail {
    flex-direction: row;
    width: auto;
    height: 48px;
    padding: 0 8px;
  }
  .sidebar-panel {
    display: none;
  }
  .org-active-bar {
    display: none;
  }
}
</style>
