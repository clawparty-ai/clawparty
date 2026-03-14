<template>
  <header class="chat-header">
    <div class="header-left">
      <button v-if="showBackButton" class="back-btn" @click="$emit('back')">
        <svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M12.707 5.293a1 1 0 010 1.414L9.414 10l3.293 3.293a1 1 0 01-1.414 1.414l-4-4a1 1 0 010-1.414l4-4a1 1 0 011.414 0z" clip-rule="evenodd"/>
        </svg>
      </button>
      <h3 class="channel-name">
        <span class="channel-icon">#</span>
        {{ chat.name }}
      </h3>
      <div class="header-divider"></div>
      <div class="header-info">
        <span class="topic">{{ chat.lastMessage }}</span>
      </div>
    </div>
    <div class="header-right">
      <div class="header-icons">
        <div v-if="chat.isOpenclaw && openclawSessions && openclawSessions.length > 0" class="session-select-wrapper">
          <select 
            class="session-select" 
            :value="chat.sessionId" 
            @change="$emit('switchSession', $event.target.value)"
          >
            <option v-for="session in openclawSessions" :key="String(session.sessionId)" :value="String(session.sessionId)">
              {{ String(session.sessionId).slice(0, 8) }}
            </option>
          </select>
        </div>
        <!-- <button class="header-btn" title="成员">
          <svg width="18" height="18" viewBox="0 0 20 20" fill="currentColor">
            <path d="M7 8a4 4 0 100-8 4 4 0 000 8zm0 2c-4 0-7 2-7 4v2h14v-2c0-2-3-4-7-4z"/>
            <path d="M14 10a2 2 0 11-4 0 2 2 0 014 0z" opacity="0.5"/>
          </svg>
        </button> -->
       <!-- <button class="header-btn" title="搜索">
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5">
            <circle cx="9" cy="9" r="7"/>
            <path d="M16 16l-3-3"/>
          </svg>
        </button> -->
        <!-- Delete group (creator only) -->
        <button
          v-if="chat.isGroup && chat.creator === currentUserName"
          class="header-btn header-btn-danger"
          title="Delete Group"
          @click="emit('deleteGroup', chat)"
        >
          <svg width="16" height="16" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z" clip-rule="evenodd"/>
          </svg>
        </button>

        <!-- Leave group (non-creator members) -->
        <button
          v-if="chat.isGroup && chat.creator !== currentUserName"
          class="header-btn header-btn-danger"
          title="Leave Group"
          @click="emit('leaveGroup', chat)"
        >
          <svg width="16" height="16" viewBox="0 0 20 20" fill="currentColor">
            <path fill-rule="evenodd" d="M3 3a1 1 0 00-1 1v12a1 1 0 102 0V4a1 1 0 00-1-1zm10.293 9.293a1 1 0 001.414 1.414l3-3a1 1 0 000-1.414l-3-3a1 1 0 10-1.414 1.414L14.586 9H7a1 1 0 100 2h7.586l-1.293 1.293z" clip-rule="evenodd"/>
          </svg>
        </button>

        <button class="header-btn" title="Settings">
          <svg width="18" height="18" viewBox="0 0 20 20" fill="currentColor">
            <path d="M10 6a2 2 0 110-4 2 2 0 010 4zm0 6a2 2 0 110-4 2 2 0 010 4zm0 6a2 2 0 110-4 2 2 0 010 4z"/>
          </svg>
        </button>
      </div>
      <button class="header-btn" title="Download chat history" @click="emit('download')">
        <svg width="18" height="18" viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M3 17a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm3.293-7.707a1 1 0 011.414 0L9 10.586V3a1 1 0 112 0v7.586l1.293-1.293a1 1 0 111.414 1.414l-3 3a1 1 0 01-1.414 0l-3-3a1 1 0 010-1.414z" clip-rule="evenodd"/>
        </svg>
      </button>
      <div class="search-box">
        <svg class="search-icon" width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M11.742 10.344a6.5 6.5 0 10-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 001.415-1.414l-3.85-3.85a1.007 1.007 0 00-.115-.1zM12 6.5a5.5 5.5 0 11-11 0 5.5 5.5 0 0111 0z"/>
        </svg>
        <input type="text" v-model="searchQuery" placeholder="Search messages">
      </div>
    </div>
  </header>
</template>

<script setup>
import { ref, watch } from 'vue'

const props = defineProps({
  chat: {
    type: Object,
    required: true
  },
  openclawSessions: {
    type: Array,
    default: () => []
  },
  currentUserName: {
    type: String,
    default: ''
  },
  showBackButton: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['search', 'switchSession', 'deleteGroup', 'leaveGroup', 'back', 'download'])

const searchQuery = ref('')

watch(searchQuery, (val) => {
  emit('search', val)
})
</script>

<style scoped>
.chat-header {
  height: var(--header-height);
  padding: 0 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border-subtle);
  background: var(--bg-secondary);
}

.header-left {
  display: flex;
  align-items: center;
  min-width: 0;
}

.back-btn {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  display: none;
  align-items: center;
  justify-content: center;
  margin-right: 8px;
  flex-shrink: 0;
}

.back-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.channel-name {
  color: var(--text-primary);
  font-size: 16px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 6px;
}

.channel-icon {
  color: var(--text-dim);
}

.header-divider {
  width: 1px;
  height: 20px;
  background: rgba(0, 0, 0, 0.15);
  margin: 0 12px;
}

.header-info {
  min-width: 0;
}

.topic {
  color: var(--text-secondary);
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 300px;
  display: block;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-icons {
  display: flex;
  align-items: center;
  gap: 8px;
}

.session-select-wrapper {
  display: flex;
  align-items: center;
}

.session-select {
  background: var(--bg-hover);
  border: 1px solid transparent;
  border-radius: 4px;
  color: var(--text-primary);
  font-size: 12px;
  padding: 4px 8px;
  cursor: pointer;
  outline: none;
}

.session-select:hover {
  background: #fff;
  border-color: var(--slack-blue);
}

.header-btn {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.1s;
}

.header-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.header-btn-danger:hover {
  background: #fee2e2;
  color: #dc2626;
}

.search-box {
  position: relative;
}

.search-box input {
  width: 180px;
  padding: 6px 12px 6px 32px;
  background: var(--bg-hover);
  border: 1px solid transparent;
  border-radius: 6px;
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
  transition: all 0.15s;
}

.search-box input:focus {
  width: 240px;
  background: #fff;
  border-color: var(--slack-blue);
}

.search-box input::placeholder {
  color: var(--text-secondary);
}

.search-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-secondary);
}

@media (max-width: 768px) {
  .chat-header {
    padding: 0 12px;
    height: 52px;
  }
  
  .search-box {
    display: none;
  }
  
  .header-divider,
  .header-info {
    display: none;
  }
  
  .channel-name {
    font-size: 15px;
  }
  
  .back-btn {
    display: flex;
  }
}
</style>
