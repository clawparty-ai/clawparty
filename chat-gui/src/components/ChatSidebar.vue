<template>
  <div class="sidebar-shell">

    <!-- Left: org switcher rail -->
    <nav class="org-rail">
      <!-- 0. Join Party button -->
      <button
        class="new-group-rail-btn"
        :class="{ active: showJoinParty }"
        @click="toggleJoinParty"
        title="加入组织"
      >
        <span class="join-party-icon">🌐</span>
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
        <span class="org-double-lobster">
					<svg t="1775022664441" class="icon" viewBox="0 0 1152 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" p-id="3524" width="18" height="18"><path d="M768 770.62144l0-52.77696c70.49216-39.7312 128-138.77248 128-237.83424 0-159.06816 0-288.01024-192-288.01024s-192 128.94208-192 288.01024c0 99.06176 57.50784 198.10304 128 237.83424l0 52.77696c-217.10848 17.75616-384 124.416-384 253.37856l896 0c0-128.96256-166.89152-235.64288-384-253.37856z" fill="#ffffff" p-id="3525" data-spm-anchor-id="a313x.search_index.0.i3.4b4d3a810fYL5Y" class="selected"></path><path d="M327.18848 795.32032c55.31648-36.1472 124.08832-63.63136 199.7824-80.40448-15.0528-17.77664-28.71296-37.62176-40.48896-59.02336-30.4128-55.23456-46.4896-116.06016-46.4896-175.90272 0-86.03648 0-167.30112 30.59712-233.75872 29.696-64.512 83.12832-104.48896 159.232-119.48032-16.91648-76.47232-61.93152-126.75072-181.82144-126.75072-192 0-192 128.94208-192 288.01024 0 99.06176 57.50784 198.10304 128 237.83424l0 52.77696c-217.10848 17.75616-384 124.416-384 253.37856l278.99904 0c14.52032-12.9024 30.59712-25.16992 48.18944-36.67968z" fill="#ffffff" p-id="3526" data-spm-anchor-id="a313x.search_index.0.i4.4b4d3a810fYL5Y" class="selected"></path></svg>
				</span>
        <span class="org-active-bar" v-if="activeOrg === 'groups'"></span>
      </div>

      <div class="org-divider"></div>

      <!-- zAgents icon -->
      <div
        class="org-icon"
        :class="{ active: activeOrg === 'zagents' }"
        @click="activeOrg = 'zagents'"
        title="zAgents"
      >
        <span class="org-emoji">
          <svg t="1775022567064" class="icon" viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" p-id="2473" width="18" height="18"><path d="M963.764706 963.764706v60.235294H60.235294v-60.235294c0-232.869647 202.270118-421.647059 451.764706-421.647059 249.494588 0 451.764706 188.777412 451.764706 421.647059zM752.941176 240.941176A240.941176 240.941176 0 1 1 271.058824 240.941176a240.941176 240.941176 0 0 1 481.882352 0z" fill="#ffffff" p-id="2474" data-spm-anchor-id="a313x.search_index.0.i0.4b4d3a810fYL5Y" class="selected"></path></svg>
        </span>
        <span class="org-active-bar" v-if="activeOrg === 'zagents'"></span>
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
      <div class="panel-header" :class="{ 'agents-header': activeOrg === 'zagents' }">
        <span class="panel-title">{{
          activeOrg === 'groups' ? 'Group Chats' :
          activeOrg === 'zagents' ? 'zAgents' :
          activeOrg
        }}</span>
        <div v-if="activeOrg !== 'groups' && activeOrg !== 'zagents' && activeOrg" class="panel-header-actions">
          <button class="leave-mesh-btn" @click="handleLeaveMesh(activeOrg)" title="Leave mesh">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
              <path d="M17 7l-1.41 1.41L18.17 11H8v2h10.17l-2.58 2.58L17 17l5-5zM4 5h8V3H4c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h8v-2H4V5z"/>
            </svg>
          </button>
        </div>
        <div v-if="activeOrg === 'zagents'" class="panel-header-actions">
          <button
            class="reconcile-btn"
            :class="{ spinning: isReconciling }"
            title="刷新 zAgent 状态"
            @click="handleReconcile"
          >
            <svg width="13" height="13" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z" clip-rule="evenodd"/>
            </svg>
          </button>
          <button class="add-agent-btn" @click="showZAgentTemplates = true" title="从模板添加 zAgent">#A</button>
          <button class="add-agent-btn plain" @click="showCreateZAgentDialog = true" title="创建 Agent">+A</button>
        </div>
      </div>

    <!-- Join Party modal -->
    <Teleport to="body">
      <div v-if="showJoinParty" class="modal-backdrop" @click.self="closeJoinParty">
        <div class="modal-dialog">
          <div class="modal-header">
            <span class="modal-title">加入组织</span>
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
            <label class="join-party-label">Username (optional)</label>
            <input
              v-model="joinPartyUserName"
              class="search-input"
              placeholder="Leave empty for random name"
              :disabled="joinPartyLoading"
            />
            <label class="join-party-label">邀请码 Invite Code <span class="required">*</span></label>
            <input
              v-model="joinPartyInviteCode"
              class="search-input"
              placeholder="请输入邀请码"
              :disabled="joinPartyLoading"
              maxlength="32"
            />
            <div v-if="joinPartyStep" class="join-party-step">{{ joinPartyStep }}</div>
            <div v-if="joinPartyError" class="join-party-error">{{ joinPartyError }}</div>
            <div v-if="joinPartySuccess" class="join-party-success">{{ joinPartySuccess }}</div>
          </div>

          <div class="modal-footer">
            <button class="modal-cancel-btn" @click="closeJoinParty" :disabled="joinPartyLoading">Cancel</button>
            <button
              class="modal-create-btn"
              :disabled="joinPartyLoading"
              @click="handleJoinParty"
            >{{ joinPartyLoading ? '加入中...' : '加入组织' }}</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Create Session Dialog -->
    <Teleport to="body">
      <div v-if="showCreateSessionDialog" class="modal-backdrop" @click.self="showCreateSessionDialog = false">
        <div class="modal-dialog">
          <div class="modal-header">
            <span class="modal-title">创建 Session</span>
            <button class="modal-close" @click="showCreateSessionDialog = false">✕</button>
          </div>

          <div class="join-party-body">
            <label class="join-party-label">Session Name</label>
            <input
              v-model="newSessionName"
              class="search-input"
              placeholder="请输入名字"
              @keyup.enter="handleCreateSession"
            />
          </div>

          <div class="modal-footer">
            <button class="modal-cancel-btn" @click="showCreateSessionDialog = false">取消</button>
            <button class="modal-create-btn" @click="handleCreateSession">创建</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Create zAgent Dialog -->
    <Teleport to="body">
      <div v-if="showCreateZAgentDialog" class="modal-backdrop" @click.self="showCreateZAgentDialog = false">
        <div class="modal-dialog zagent-create-dialog">
          <div class="modal-header">
            <span class="modal-title">创建 zAgent</span>
            <button class="modal-close" @click="showCreateZAgentDialog = false">✕</button>
          </div>

          <div class="zagent-form-body">
            <!-- Agent Name -->
            <div class="form-field">
              <label class="form-label">Agent Name <span class="required">*</span></label>
              <input
                v-model="newZAgentName"
                class="search-input"
                placeholder="请输入 agent 名字"
                @keyup.enter="handleCreateZAgent"
              />
            </div>

            <!-- Description -->
            <div class="form-field">
              <label class="form-label">
                描述（可选）
                <span class="char-count" :class="{ 'char-count-warn': newZAgentDescription.length > 2800 }">
                  {{ newZAgentDescription.length }}/3000
                </span>
              </label>
              <textarea
                v-model="newZAgentDescription"
                class="search-input desc-textarea"
                placeholder="Agent 描述，例如：负责客服回复的助手"
                :maxlength="3000"
                rows="4"
              ></textarea>
            </div>

            <!-- Model Config Section -->
            <div class="model-config-section">
              <button class="model-config-toggle" @click="showModelConfig = !showModelConfig">
                <span class="toggle-icon">{{ showModelConfig ? '▾' : '▸' }}</span>
                模型配置（可选，留空使用全局配置）
              </button>

              <!-- Global config hint -->
              <div v-if="globalConfig && !newZAgentProvider" class="global-config-hint">
                <span class="hint-icon">ℹ️</span>
                <span class="hint-text">将使用全局配置: {{ globalConfig.llm?.model || '未知模型' }}</span>
              </div>

              <div v-if="showModelConfig" class="model-config-fields">
                <!-- Provider -->
                <div class="form-field">
                  <label class="form-label">Provider</label>
                  <select v-model="newZAgentProvider" class="search-input form-select">
                    <option value="">-- 使用全局配置 --</option>
                    <option value="custom">OpenAI 兼容（自定义端点）</option>
                    <option value="openai">OpenAI</option>
                    <option value="anthropic">Anthropic</option>
                    <option value="qwen">阿里云通义</option>
                    <option value="moonshot">月之暗面 Kimi</option>
                    <option value="doubao">字节豆包</option>
                    <option value="deepseek">DeepSeek</option>
                    <option value="ollama">本地 Ollama</option>
                  </select>
                </div>

                <!-- API Endpoint (only for custom / ollama) -->
                <div v-if="newZAgentProvider === 'custom' || newZAgentProvider === 'ollama'" class="form-field">
                  <label class="form-label">
                    API 端点
                    <span v-if="newZAgentProvider === 'custom'" class="required">*</span>
                  </label>
                  <input
                    v-model="newZAgentApiEndpoint"
                    class="search-input"
                    :placeholder="newZAgentProvider === 'ollama' ? 'http://localhost:11434' : 'https://your-endpoint/v1'"
                  />
                </div>

                <!-- API Key -->
                <div class="form-field">
                  <label class="form-label">
                    API Key
                    <span v-if="newZAgentProvider && newZAgentProvider !== 'ollama'" class="required">*</span>
                  </label>
                  <input
                    v-model="newZAgentApiKey"
                    type="password"
                    class="search-input"
                    placeholder="sk-..."
                    autocomplete="new-password"
                  />
                </div>

                <!-- Model -->
                <div class="form-field">
                  <label class="form-label">
                    模型名称
                    <span v-if="newZAgentProvider" class="required">*</span>
                  </label>
                  <input
                    v-model="newZAgentModel"
                    class="search-input"
                    :placeholder="modelPlaceholder"
                  />
                </div>

                <p v-if="modelConfigError" class="form-error">{{ modelConfigError }}</p>
              </div>
            </div>
          </div>

          <div class="modal-footer">
            <button class="modal-cancel-btn" @click="showCreateZAgentDialog = false">取消</button>
            <button class="modal-create-btn" @click="handleCreateZAgent" :disabled="!newZAgentName.trim()">创建</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Group chat creation modal -->
    <Teleport to="body">
      <div v-if="showPicker" class="modal-backdrop" @click.self="closePicker">
        <div class="modal-dialog">
          <div class="modal-header">
            <span class="modal-title">新群聊</span>
            <button class="modal-close" @click="closePicker">✕</button>
          </div>

          <!-- Group name -->
          <div class="modal-search" style="margin-bottom: 4px;">
            <input
              v-model="newGroupName"
              class="search-input"
              placeholder="输入群聊名称..."
              autofocus
            />
          </div>

          <!-- Search filter -->
          <div class="modal-search">
            <input
              v-model="pickerSearch"
              class="search-input"
              placeholder="搜索 zAgent..."
            />
          </div>

          <div class="modal-list">
            <!-- zAgents only -->
            <template v-if="filteredZAgents.length > 0">
              <div class="modal-section-label">zAgents</div>
              <label
                v-for="agent in filteredZAgents"
                :key="'z-' + agent.agent_name"
                class="modal-item"
                :class="{ selected: pickerSelected.includes(agent.agent_name) }"
              >
                <input
                  type="checkbox"
                  :checked="pickerSelected.includes(agent.agent_name)"
                  @change="togglePickerUser(agent.agent_name)"
                />
                <div class="item-avatar">🤖</div>
                <span class="item-name">{{ agent.display_name || agent.agent_name }}</span>
                <span class="item-tag">zagent</span>
              </label>
            </template>

            <div v-if="filteredZAgents.length === 0" class="modal-empty">
              <span v-if="zAgents.length === 0">没有可用的 zAgent，请先创建一个。</span>
              <span v-else>没有匹配的 zAgent。</span>
            </div>
          </div>

          <div class="modal-footer">
            <span class="modal-count">{{ pickerSelected.length }} 已选</span>
            <button class="modal-cancel-btn" @click="closePicker">取消</button>
            <button
              class="modal-create-btn"
              :disabled="pickerSelected.length === 0 || !newGroupName.trim()"
              @click="handleCreateGroup"
            >创建群聊</button>
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

    <!-- ISA Editor modal -->
    <Teleport to="body">
      <div v-if="editingFile" class="modal-backdrop" @click.self="closeEditor">
        <div class="editor-dialog">
          <div class="editor-header">
            <span class="editor-title">{{ editingAgent?.name }} - {{ editingFile }}</span>
            <div class="file-tabs">
              <button 
                v-for="f in isaFiles" 
                :key="f"
                class="file-tab"
                :class="{ active: editingFile === f }"
                @click="switchFile(f)"
              >{{ f.replace('.md', '') }}</button>
            </div>
            <button class="editor-close" @click="closeEditor">✕</button>
          </div>

          <div v-if="loadingFile" class="editor-loading">Loading...</div>
          <div v-else-if="fileError" class="editor-error">{{ fileError }}</div>
          <textarea 
            v-else 
            v-model="fileContent" 
            class="editor-textarea"
            placeholder="File content..."
          ></textarea>

          <div class="editor-actions">
            <button class="cancel-btn" @click="closeEditor">Cancel</button>
            <button 
              class="save-btn" 
              :disabled="savingFile || !canSave" 
              @click="saveFile"
            >
              {{ savingFile ? 'Saving...' : 'Save' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- zAgent Template Picker -->
    <TemplatePicker
      :show="showZAgentTemplates"
      source="local"
      mode="zagent"
      @close="showZAgentTemplates = false"
      @installed="fetchZAgents()"
      @created="(data) => emit('zagentTemplateCreated', data)"
    />

      <div class="panel-list">
        <!-- Group Chats view — local ZeroClaw agent groups -->
        <template v-if="activeOrg === 'groups'">
          <template v-if="localGroupChats && localGroupChats.length > 0">
            <div
              v-for="chat in localGroupChats"
              :key="chat.groupId"
              class="group-chat-entry"
            >
              <div
                class="panel-item group-chat-header"
                :class="{ active: activeGroupId === chat.groupId }"
                @click="enterGroupChat(chat.groupId)"
              >
                <div class="item-hash" @click.stop="toggleExpand(chat.groupId)">{{ expandedGroups.has(chat.groupId) ? '▼' : '▶' }}</div>
                <span class="item-name">{{ chat.groupName }}</span>
                <button class="group-action-btn" @click.stop="handleDeleteLocalGroup(chat)" title="删除群聊">🗑</button>
              </div>
              <div v-if="expandedGroups.has(chat.groupId)" class="group-members-list">
                <div class="group-member-item">
                  <div class="member-avatar">👑</div>
                  <span class="member-name">{{ chat.ownerAgent }} (owner)</span>
                </div>
                <div
                  v-for="member in (chat.members || [])"
                  :key="member"
                  class="group-member-item"
                >
                  <div class="member-avatar">🤖</div>
                  <span class="member-name">{{ member }}</span>
                </div>
              </div>
            </div>
          </template>
          <div v-else class="panel-empty">还没有群聊，点击 #+ 创建</div>
        </template>

        
        <!-- zAgents -->
        <template v-if="activeOrg === 'zagents'">
          <template v-if="zAgents && zAgents.length > 0">
            <div
              v-for="agent in zAgents"
              :key="agent.agent_name"
              class="zagent-item-wrapper"
            >
              <div
                class="panel-item zagent-item"
                :class="{ active: activeZAgent?.agent_name === agent.agent_name }"
                @click="selectZAgent(agent)"
              >
                <div class="item-avatar" :style="{ background: getAvatarColor(agent.display_name || agent.agent_name) }">{{ getAgentEmoji(agent.display_name || agent.agent_name) }}</div>
                <div class="zagent-info">
                  <div class="zagent-name-row">
                    <span class="item-name">{{ agent.display_name || agent.agent_name }}</span>
                  </div>
                  <div class="zagent-status-text" :class="'status-' + (agent.status || 'created')">
                    {{ getStatusText(agent) }}
                  </div>
                </div>
                <div class="zagent-actions">
                  <!-- Start button: show for non-0#Agent created/stopped/error -->
                  <button
                    v-if="agent.agent_name !== '0#Agent' && ['created', 'stopped', 'error'].includes(agent.status || 'created')"
                    class="agent-action-btn start-btn"
                    @click.stop="handleStartAgent(agent.agent_name)"
                    title="Start Agent"
                  >
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                      <path d="M8 5v14l11-7z"/>
                    </svg>
                  </button>
                  <!-- Stop button: show for non-0#Agent running -->
                  <button
                    v-if="agent.agent_name !== '0#Agent' && agent.status === 'running'"
                    class="agent-action-btn stop-btn"
                    @click.stop="handleStopAgent(agent.agent_name)"
                    title="Stop Agent"
                  >
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                      <rect x="6" y="6" width="12" height="12"/>
                    </svg>
                  </button>
                  <!-- Delete button -->
                  <button
                    v-if="agent.agent_name !== '0#Agent'"
                    class="agent-action-btn delete-btn"
                    @click.stop="handleDeleteZAgent(agent.agent_name)"
                    title="Delete Agent"
                  >
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                      <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                    </svg>
                  </button>
                </div>
              </div>
            </div>
          </template>
          <div v-else class="panel-empty">No zAgents. Click +A to create one.</div>
        </template>

        <!-- My Agents -->
        <template v-else-if="activeOrg === 'agents'">
          <div
            v-for="agent in openclawAgents"
            :key="agent.id"
            class="panel-item agent-item"
            :class="{ active: activeOpenclawAgent?.agentId === agent.id }"
            @click="$emit('selectOpenclaw', agent)"
          >
            <div class="item-avatar openclaw-avatar">{{ agent.emoji }}</div>
            <span class="item-name">{{ agent.name }}</span>
            <div class="agent-item-actions">
              <template v-if="agent.id !== 'main'">
                <button
                  class="isa-btn isa-i"
                  @click.stop="openEditor(agent, 'IDENTITY.md')"
                  title="Edit Identity"
                >I</button>
                <button
                  class="isa-btn isa-s"
                  @click.stop="openEditor(agent, 'SOUL.md')"
                  title="Edit Soul"
                >S</button>
                <button
                  class="isa-btn isa-a"
                  @click.stop="openEditor(agent, 'AGENTS.md')"
                  title="Edit Agents"
                >A</button>
              </template>
              <button
                v-if="agent.id !== 'main'"
                class="delete-agent-btn"
                @click.stop="handleDeleteAgent(agent)"
                title="Delete agent"
              >
                <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
                  <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                </svg>
              </button>
            </div>
          </div>
          <div v-if="openclawAgents.length === 0" class="panel-empty-state">
            <div class="panel-empty-state-title">No local agents</div>
            <div class="panel-empty-state-hint">openclaw is not installed locally. You can still interact with remote openclaw agents via group chat.</div>
          </div>
        </template>

        <!-- Mesh: users + groups -->
        <template v-if="meshes.some(m => m.name === activeOrg)">
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
              <span class="item-subname" v-if="user.username">{{ user.name }}/{{ user.username }}</span>
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
            <span class="item-name">{{ chat.displayName }}</span>
            <span class="item-subname" v-if="chat.name !== chat.displayName">{{ chat.displayName }}/{{ chat.name }}</span>
            <span v-if="chat.updated > 0" class="unread-badge">{{ chat.updated > 99 ? '99+' : chat.updated }}</span>
          </div>

          </template>
      </div>
    </aside>

  </div>
</template>

<script setup>
import { ref, computed, inject, watch, onMounted, onUnmounted } from 'vue'
import { getAvatarColor } from '../utils/avatar'
import { getAgentEmoji } from '../utils/emoji'
import TemplatePicker from './TemplatePicker.vue'

const props = defineProps({
  chats: { type: Array, required: true },
  activeChat: { type: Number, default: null }
})

const currentMesh = inject('currentMesh')
const meshes = inject('meshes')
const openclawAgents = inject('openclawAgents')
const zeroclawSessions = inject('zeroclawSessions')
const switchMesh = inject('switchMesh')
const users = inject('users')
const selectUser = inject('selectUser')
const createGroupChat = inject('createGroupChat')
const renameGroupChat = inject('renameGroupChat')
const updateGroupMembers = inject('updateGroupMembers')
const currentMeshAgentUsername = inject('currentMeshAgentUsername')
const activeOpenclawAgent = inject('activeOpenclawAgent')
const activeZeroClawSession = inject('activeZeroClawSession')
const selectZeroClawSession = inject('selectZeroClawSession')
const joinParty = inject('joinParty')
const leaveMesh = inject('leaveMesh')
const resolveEpDisplayName = inject('resolveEpDisplayName')
const localOpenclawAvailable = inject('localOpenclawAvailable')
const createSession = inject('createSession')
const fetchZeroClawSessions = inject('fetchZeroClawSessions')
const zAgents = inject('zAgents')
const activeZAgent = inject('activeZAgent')
const selectZAgent = inject('selectZAgent')
const createZAgent = inject('createZAgent')
const localGroupChats = inject('localGroupChats')
const enterGroupChat = inject('enterGroupChat')
const leaveGroupChat = inject('leaveGroupChat')
const activeGroupId = inject('activeGroupId')
const handleDeleteLocalGroup = inject('handleDeleteLocalGroup')
const deleteZAgent = inject('deleteZAgent')
const fetchZAgents = inject('fetchZAgents')

// Agent status reconcile (force refresh)
const isReconciling = ref(false)

const handleReconcile = async () => {
  if (isReconciling.value) return
  isReconciling.value = true
  try {
    const { zagentService } = await import('../services/chatService')
    const res = await zagentService.reconcileAgents()
    // Backend may return array directly or wrapped in { data: [...] }
    const agents = Array.isArray(res) ? res : (res.data || [])
    const validAgents = agents.filter(a => a && a.agent_name)
    if (validAgents.length > 0) {
      zAgents.value = validAgents
    } else {
      await fetchZAgents()
    }
  } catch (e) {
    console.error('[ChatSidebar] Reconcile failed:', e)
    await fetchZAgents()
  }
  isReconciling.value = false
}

// Agent status polling — global timer to avoid one request per starting agent
var sidebarPollingTimer = null

const startPolling = () => {
  if (sidebarPollingTimer) return
  sidebarPollingTimer = setInterval(() => {
    fetchZAgents()
  }, 3000)
}

const stopPolling = () => {
  if (sidebarPollingTimer) {
    clearInterval(sidebarPollingTimer)
    sidebarPollingTimer = null
  }
}

const stopAllPolling = () => {
  stopPolling()
}

// Watch zAgents for status changes
watch(zAgents, (newAgents) => {
  if (!newAgents) return

  var hasStarting = false
  for (var i = 0; i < newAgents.length; i++) {
    if (newAgents[i].status === 'starting') {
      hasStarting = true
      break
    }
  }

  if (hasStarting) {
    startPolling()
  } else {
    stopPolling()
  }
}, { deep: true })

onUnmounted(() => {
  stopAllPolling()
})

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
const activeOrg = ref('zagents')

const emit = defineEmits(['select', 'selectOpenclaw', 'changeOrg', 'openLocalTemplates', 'openSharedTemplates', 'resetActiveChat', 'zagentTemplateCreated'])

watch(currentMesh, (val) => {
  if (val && activeOrg.value !== 'zagents' && activeOrg.value !== 'groups') activeOrg.value = val
}, { immediate: true })

watch(activeOrg, (val) => {
  emit('changeOrg', val)
  emit('resetActiveChat')
  if (val === 'zagents') {
    fetchZAgents()
    // Auto-select first agent if available
    if (zAgents.value && zAgents.value.length > 0 && !activeZAgent.value) {
      selectZAgent(zAgents.value[0])
    }
  }
})

const handleSelectMesh = async (meshName) => {
  activeOrg.value = meshName
  await switchMesh(meshName)
}

const handleLeaveMesh = async (meshName) => {
  if (!confirm(`Are you sure you want to leave mesh "${meshName}"?`)) return
  try {
    await leaveMesh(meshName)
    activeOrg.value = 'agents'
  } catch (err) {
    console.error('Failed to leave mesh:', err)
  }
}

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
const newGroupName = ref('')

const togglePicker = () => {
  showPicker.value = !showPicker.value
  if (!showPicker.value) {
    pickerSelected.value = []
    pickerSearch.value = ''
    newGroupName.value = ''
  }
}

const closePicker = () => {
  showPicker.value = false
  pickerSelected.value = []
  pickerSearch.value = ''
  newGroupName.value = ''
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

const filteredZeroclawSessions = computed(() => {
  const q = pickerSearch.value.trim().toLowerCase()
  if (!q) return zeroclawSessions.value
  return zeroclawSessions.value.filter(s => 
    (s.session_id || '').toLowerCase().includes(q) ||
    (s.name || '').toLowerCase().includes(q)
  )
})

const filteredZAgents = computed(() => {
  const q = pickerSearch.value.trim().toLowerCase()
  if (!q) return zAgents.value
  return zAgents.value.filter(a => 
    (a.agent_name || '').toLowerCase().includes(q) ||
    (a.display_name || '').toLowerCase().includes(q)
  )
})

const togglePickerUser = (name) => {
  const idx = pickerSelected.value.indexOf(name)
  if (idx === -1) pickerSelected.value.push(name)
  else pickerSelected.value.splice(idx, 1)
}

// Join Party state
const showJoinParty = ref(false)
const joinPartyUrl = ref('https://join.clawparty.ai')
const joinPartyUserName = ref('')
const joinPartyInviteCode = ref('')
const joinPartyLoading = ref(false)
const joinPartyError = ref('')
const joinPartySuccess = ref('')
const joinPartyStep = ref('')

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
  joinPartyStep.value = ''
  joinPartyUserName.value = ''
  joinPartyInviteCode.value = ''
}

// Create Session state
const showCreateSessionDialog = ref(false)
const newSessionName = ref('')

const handleCreateSession = async () => {
  if (!newSessionName.value.trim()) return
  try {
    await createSession(newSessionName.value.trim())
    showCreateSessionDialog.value = false
    newSessionName.value = ''
  } catch (error) {
    console.error('Failed to create session:', error)
  }
}

// Create zAgent state
const showCreateZAgentDialog = ref(false)
const showZAgentTemplates = ref(false)
const newZAgentName = ref('')
const newZAgentDescription = ref('')
const showModelConfig = ref(false)
const newZAgentProvider = ref('')
const newZAgentApiEndpoint = ref('')
const newZAgentApiKey = ref('')
const newZAgentModel = ref('')
const modelConfigError = ref('')
const globalConfig = ref(null)

// Load global config when component mounts
onMounted(async () => {
  try {
    const { zagentService } = await import('../services/chatService')
    const response = await zagentService.getGlobalConfig()
    globalConfig.value = response.data
  } catch (error) {
    console.log('No global config found:', error)
  }
})

const modelPlaceholder = computed(() => {
  const placeholders = {
    openai: 'gpt-4o-mini',
    anthropic: 'claude-3-5-haiku-20241022',
    qwen: 'qwen-plus',
    moonshot: 'moonshot-v1-8k',
    doubao: 'doubao-pro-32k',
    deepseek: 'deepseek-chat',
    ollama: 'llama3.2',
    custom: 'your-model-name',
  }
  return placeholders[newZAgentProvider.value] || '模型名称'
})

const handleCreateZAgent = async () => {
  if (!newZAgentName.value.trim()) return

  modelConfigError.value = ''

  // Validate model config: if any field is filled, api_key and model are required
  const hasAnyModelField = newZAgentProvider.value || newZAgentApiKey.value || newZAgentModel.value || newZAgentApiEndpoint.value
  if (hasAnyModelField) {
    if (!newZAgentApiKey.value && newZAgentProvider.value !== 'ollama') {
      modelConfigError.value = '填写了模型配置时，API Key 为必填项'
      return
    }
    if (!newZAgentModel.value) {
      modelConfigError.value = '填写了模型配置时，模型名称为必填项'
      return
    }
    if (newZAgentProvider.value === 'custom' && !newZAgentApiEndpoint.value) {
      modelConfigError.value = '使用自定义端点时，API 端点为必填项'
      return
    }
  }

  try {
    const agentConfig = {
      agent_name: newZAgentName.value.trim(),
      description: newZAgentDescription.value.trim() || null,
      provider: newZAgentProvider.value || null,
      api_endpoint: newZAgentApiEndpoint.value.trim() || null,
      api_key: newZAgentApiKey.value.trim() || null,
      model: newZAgentModel.value.trim() || null,
    }
    await createZAgent(agentConfig)
    showCreateZAgentDialog.value = false
    // Reset form
    newZAgentName.value = ''
    newZAgentDescription.value = ''
    newZAgentProvider.value = ''
    newZAgentApiEndpoint.value = ''
    newZAgentApiKey.value = ''
    newZAgentModel.value = ''
    modelConfigError.value = ''
  } catch (error) {
    console.error('Failed to create zAgent:', error)
  }
}

const handleDeleteZAgent = async (agentName) => {
  if (!confirm(`Are you sure you want to delete zAgent "${agentName}"? This action cannot be undone.`)) return
  try {
    await deleteZAgent(agentName)
  } catch (error) {
    console.error('Failed to delete zAgent:', error)
  }
}

const handleStartAgent = async (agentName) => {
  try {
    const { zagentService } = await import('../services/chatService')
    await zagentService.startAgent(agentName)
    // Start polling immediately after starting the agent (global timer)
    startPolling()
    await fetchZAgents()
  } catch (error) {
    console.error('Failed to start zAgent:', error)
    alert(`启动 Agent "${agentName}" 失败: ${error?.response?.data?.message || error?.message || '未知错误'}`)
  }
}

const handleStopAgent = async (agentName) => {
  if (!confirm(`确认停止 Agent "${agentName}"？`)) return
  try {
    const { zagentService } = await import('../services/chatService')
    await zagentService.stopAgent(agentName)
    await fetchZAgents()
  } catch (error) {
    console.error('Failed to stop zAgent:', error)
    alert(`停止 Agent "${agentName}" 失败: ${error?.response?.data?.message || error?.message || '未知错误'}`)
  }
}

const handleJoinParty = async () => {
  if (joinPartyLoading.value) return

  // Validate invite code
  if (!joinPartyInviteCode.value.trim()) {
    joinPartyError.value = '请输入邀请码'
    return
  }

  joinPartyLoading.value = true
  joinPartyError.value = ''
  joinPartySuccess.value = ''
  joinPartyStep.value = '正在连接 Hub...'
  try {
    await joinParty(joinPartyUrl.value, joinPartyUserName.value.trim() || undefined, joinPartyInviteCode.value.trim())
    joinPartyStep.value = '正在获取配置...'
    // Wait a bit to show the step
    await new Promise(resolve => setTimeout(resolve, 300))
    joinPartyStep.value = '正在创建 0#Agent...'
    await new Promise(resolve => setTimeout(resolve, 300))
    joinPartySuccess.value = '成功加入组织！'
    joinPartyStep.value = ''
    setTimeout(() => {
      closeJoinParty()
    }, 1500)
  } catch (err) {
    const msg = err?.response?.data?.message || err?.message || 'Failed to join party'
    // Map error messages to Chinese
    if (msg === 'missing InviteCode') {
      joinPartyError.value = '请输入邀请码'
    } else if (msg === 'invalid invite code') {
      joinPartyError.value = '邀请码无效，请确认后重试'
    } else if (msg === 'invite code already used') {
      joinPartyError.value = '该邀请码已被使用'
    } else {
      joinPartyError.value = msg
    }
    joinPartyStep.value = ''
  } finally {
    joinPartyLoading.value = false
  }
}

const handleCreateGroup = async () => {
  if (pickerSelected.value.length === 0 || !newGroupName.value.trim()) return

  // Only zAgents are selectable now
  const selectedAgentNames = pickerSelected.value  // array of agent_name strings
  const groupName = newGroupName.value.trim()

  await createGroupChat(selectedAgentNames, groupName)
  closePicker()
  activeOrg.value = 'groups'
}

const handleDeleteAgent = async (agent) => {
  if (!confirm(`Are you sure you want to delete agent "${agent.name}"? This action cannot be undone.`)) return
  
  try {
    // Find main agent and select it first
    const mainAgent = openclawAgents.value.find(a => a.id === 'main')
    if (mainAgent) {
      emit('selectOpenclaw', mainAgent)
    }
    
    // Wait a bit for the UI to update, then send delete command
    setTimeout(async () => {
      const { openclawService } = await import('../services/chatService')
      await openclawService.sendMessage('main', `/agent delete ${agent.id}`)
    }, 300)
  } catch (err) {
    console.error('Failed to send delete command:', err)
    alert('Failed to delete agent: ' + (err?.response?.data?.message || err?.message || 'Unknown error'))
  }
}

const isaFiles = ['IDENTITY.md', 'SOUL.md', 'AGENTS.md']
const editingAgent = ref(null)
const editingFile = ref(null)
const fileContent = ref('')
const fileOriginalContent = ref('')
const loadingFile = ref(false)
const fileError = ref('')
const savingFile = ref(false)

const canSave = computed(() => {
  return fileContent.value !== fileOriginalContent.value && !loadingFile.value && !savingFile.value && !fileError.value
})

const openEditor = async (agent, filename) => {
  editingAgent.value = agent
  editingFile.value = filename
  loadingFile.value = true
  fileError.value = ''
  fileContent.value = ''
  fileOriginalContent.value = ''
  
  try {
    const { openclawService } = await import('../services/chatService')
    const response = await openclawService.getAgentWorkspaceFile(agent.id, filename)
    fileContent.value = response.data || ''
    fileOriginalContent.value = fileContent.value
  } catch (err) {
    if (err?.response?.status === 403) {
      fileError.value = err.response.data?.message || 'Directory is not writable'
    } else if (err?.response?.status === 404) {
      fileError.value = err.response.data?.message || 'Agent workspace not found'
      fileContent.value = ''
      fileOriginalContent.value = ''
    } else {
      fileError.value = 'Failed to load file'
      console.error('Failed to load file:', err)
    }
  } finally {
    loadingFile.value = false
  }
}

const switchFile = async (filename) => {
  if (!editingAgent.value) return
  await openEditor(editingAgent.value, filename)
}

const saveFile = async () => {
  if (!editingAgent.value || !editingFile.value || !canSave.value) return
  
  savingFile.value = true
  fileError.value = ''
  
  try {
    const { openclawService } = await import('../services/chatService')
    await openclawService.saveAgentWorkspaceFile(editingAgent.value.id, editingFile.value, fileContent.value)
    fileOriginalContent.value = fileContent.value
    alert('File saved successfully')
  } catch (err) {
    fileError.value = err?.response?.data?.message || 'Failed to save file'
    console.error('Failed to save file:', err)
  } finally {
    savingFile.value = false
  }
}

const closeEditor = () => {
  editingAgent.value = null
  editingFile.value = null
  fileContent.value = ''
  fileOriginalContent.value = ''
  fileError.value = ''
  loadingFile.value = false
  savingFile.value = false
}

// Get status text for zAgent
const getStatusText = (agent) => {
  const status = agent.status || 'created'
  switch (status) {
    case 'created':
      return '未启动'
    case 'starting':
      return '正在启动...'
    case 'running':
      return '运行中'
    case 'stopped':
      return '已停止'
    case 'error':
      const msg = agent.error_msg || 'Unknown error'
      return msg.length > 30 ? msg.substring(0, 30) + '...' : msg
    default:
      return status
  }
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
  background: var(--org-rail-bg);
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
  background: rgba(255, 255, 255, 0.8);
  margin: 4px 0;
}

.org-icon {
  position: relative;
  width: 40px;
  height: 40px;
  border-radius: 12px;
  background: #4095fe;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: border-radius 0.15s, background 0.15s;
  flex-shrink: 0;
}

.org-icon:hover {
  border-radius: 14px;
  background: #1d6cff;
}

.org-icon.active {
  border-radius: 14px;
  background: var(--slack-aubergine);
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
  border: 1.5px solid var(--org-rail-bg);
}

.org-online-dot.online { background: var(--slack-green); }

.org-active-bar {
  position: absolute;
  left: -8px;
  top: 50%;
  transform: translateY(-50%);
  width: 4px;
  height: 24px;
  background: #4095fe;
  border-radius: 0 3px 3px 0;
}

/* ── Panel ── */
.sidebar-panel {
  flex: 1;
  background: var(--sidebar-panel-bg);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: inset -4px 0 8px rgba(0, 0, 0, 0.05);
}

.panel-header {
  height: 48px;
  padding: 0 12px 0 16px;
  display: flex;
  align-items: center;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  flex-shrink: 0;
  background: rgba(0, 0, 0, 0.08);
}

.agents-header {
  background: rgba(0, 0, 0, 0.08);
  border-bottom: 1px solid rgba(0, 0, 0, 0.1);
}

.panel-title {
  flex: 1;
  color: #333;
  font-size: 15px;
  font-weight: 700;
  text-transform: capitalize;
}

.panel-header-actions {
  display: flex;
  gap: 6px;
  align-items: center;
}

.add-agent-btn {
  padding: 3px 8px;
  background: var(--slack-green);
  border: none;
  border-radius: 5px;
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
  transition: opacity 0.15s;
  font-family: monospace;
}

.add-agent-btn:hover {
  opacity: 0.85;
}

.add-agent-btn.plain {
  background: none;
  border: 1px solid var(--slack-green);
  color: var(--slack-green);
}

.add-agent-btn.plain:hover {
  background: rgba(1, 178, 75, 0.1);
}

.reconcile-btn {
  width: 26px;
  height: 26px;
  padding: 0;
  background: transparent;
  border: none;
  border-radius: 5px;
  color: var(--text-dim, #797979);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.reconcile-btn:hover {
  background: rgba(64, 149, 254, 0.1);
  color: #4095fe;
}

.reconcile-btn.spinning svg {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.shared-btn {
  background: #4095fe;
}

.leave-mesh-btn {
  width: 28px;
  height: 28px;
  padding: 0;
  background: transparent;
  border: none;
  border-radius: 5px;
  color: #888;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.15s, background 0.15s;
}

.leave-mesh-btn:hover {
  color: #e01e5a;
  background: rgba(224, 30, 90, 0.1);
}

/* ── Group Chats org icon accent ── */
.group-chats-icon {
  background: #4095fe;
}

.group-chats-icon:hover,
.group-chats-icon.active {
  background: #1d6cff;
}

.org-double-lobster {
  font-size: 13px;
  line-height: 1;
  letter-spacing: 0;
}

/* ── Panel empty state ── */
.panel-empty {
  padding: 32px 16px;
  text-align: center;
  color: rgba(0, 0, 0, 0.35);
  font-size: 13px;
}

.panel-empty-state {
  padding: 24px 16px;
  text-align: center;
}

.panel-empty-state-title {
  color: rgba(0, 0, 0, 0.45);
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 8px;
}

.panel-empty-state-hint {
  color: rgba(0, 0, 0, 0.28);
  font-size: 12px;
  line-height: 1.5;
}

/* ── New group button in org-rail ── */
.new-group-rail-btn {
  position: relative;
  width: 40px;
  height: 40px;
  border-radius: 12px;
  background: #4095fe;
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
  background: #1d6cff;
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
  background: #ffffff;
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
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
  flex-shrink: 0;
}

.modal-title {
  flex: 1;
  color: #333333;
  font-size: 17px;
  font-weight: 700;
}

.modal-close {
  width: 28px;
  height: 28px;
  background: transparent;
  border: none;
  color: rgba(0, 0, 0, 0.5);
  font-size: 16px;
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.1s, color 0.1s;
}

.modal-close:hover {
  background: rgba(0, 0, 0, 0.1);
  color: #333333;
}

.modal-search {
  padding: 12px 16px 8px;
  flex-shrink: 0;
}

.search-input {
  width: 100%;
  box-sizing: border-box;
  padding: 8px 12px;
  background: rgba(0, 0, 0, 0.05);
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: 7px;
  color: #333333;
  font-size: 14px;
  outline: none;
  transition: border-color 0.15s;
}

.search-input::placeholder { color: rgba(0, 0, 0, 0.35); }

.search-input:focus {
  border-color: rgba(0, 0, 0, 0.35);
  background: rgba(0, 0, 0, 0.08);
}

.modal-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px 8px;
}

.modal-section-label {
  padding: 8px 10px 4px;
  color: rgba(0, 0, 0, 0.4);
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

.modal-item .item-name {
  color: #000000;
}

.modal-item .item-subname {
  color: rgba(0, 0, 0, 0.6);
}

.modal-empty {
  padding: 32px 0;
  text-align: center;
  color: rgba(0, 0, 0, 0.35);
  font-size: 14px;
}

.modal-footer {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
  gap: 10px;
  flex-shrink: 0;
}

.modal-count {
  flex: 1;
  color: rgba(0, 0, 0, 0.45);
  font-size: 13px;
}

.modal-cancel-btn {
  padding: 7px 16px;
  background: rgba(0, 0, 0, 0.1);
  border: none;
  border-radius: 6px;
  color: rgba(0, 0, 0, 0.8);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.1s;
}

.modal-cancel-btn:hover { background: rgba(0, 0, 0, 0.18); }

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

.panel-item.active { background: rgba(0, 0, 0, 0.1); }

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
  background: linear-gradient(135deg, #cecece, #cecece);
  font-size: 17px;
}

.item-hash {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #0A2E6F;
  font-size: 11px;
  font-weight: 700;
  margin-right: 6px;
  flex-shrink: 0;
  cursor: pointer;
  transition: color 0.15s;
}

.item-hash:hover {
  color: #0A2E6F;
}

.panel-item.active .item-hash { color: #0A2E6F; }

.item-name {
  flex: 1;
  color: #333;
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
  color: #000;
  font-weight: 700;
}

.agent-item {
  position: relative;
}

.agent-item-actions {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  align-items: center;
  gap: 2px;
}

.isa-btn {
  width: 20px;
  height: 20px;
  border: none;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.7;
  transition: opacity 0.15s;
  color: #fff;
}

.isa-btn:hover {
  opacity: 1;
}

.isa-i {
  background: #3b82f6;
}

.isa-s {
  background: #10b981;
}

.isa-a {
  background: #8b5cf6;
}

.zagent-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-left: 8px;
}

.agent-action-btn {
  width: 24px;
  height: 24px;
  padding: 0;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: #888;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.7;
  transition: all 0.2s;
  flex-shrink: 0;
}

.agent-action-btn:hover {
  opacity: 1;
}

.start-btn:hover {
  color: #2eb67d;
  background: rgba(46, 182, 125, 0.1);
}

.stop-btn:hover {
  color: #e01e5a;
  background: rgba(224, 30, 90, 0.1);
}

.delete-btn:hover {
  color: #e01e5a;
  background: rgba(224, 30, 90, 0.1);
}

.item-status {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #ccc;
  flex-shrink: 0;
}

.item-status.online { background: var(--slack-green); }
.item-status.running { background: #4caf50; }

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
  background: #4095fe;
  cursor: default;
}

.profile-icon:hover {
  background: #1d6cff;
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

.join-party-label .required {
  color: #e01e5a;
  margin-left: 2px;
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

.join-party-step {
  color: #667eea;
  font-size: 13px;
  padding: 4px 0;
}

.global-config-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  background: rgba(102, 126, 234, 0.08);
  border-radius: 6px;
  margin: 6px 0;
  font-size: 12px;
}

.hint-text {
  color: #667eea;
}

@media (max-width: 768px) {
  .sidebar-shell { 
    width: 100%; 
    height: 48px; 
    position: fixed;
    top: 0;
    left: 0;
    z-index: 100;
    padding-top: env(safe-area-inset-top, 0);
    background: var(--slack-aubergine);
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
  /* color: #aaa; */
  cursor: pointer;
  font-size: 12px;
  padding: 0 3px;
  flex-shrink: 0;
  line-height: 1;
}
.group-action-btn:hover { 
	/* color: #fff; */
}

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

/* ISA Editor */
.editor-dialog {
  background: #fff;
  border-radius: 12px;
  width: 600px;
  max-width: 90vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.editor-header {
  display: flex;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
  flex-shrink: 0;
}

.editor-title {
  font-size: 15px;
  font-weight: 600;
  color: #333;
  flex-shrink: 0;
}

.file-tabs {
  display: flex;
  gap: 4px;
  margin-left: 16px;
  flex: 1;
}

.file-tab {
  padding: 4px 10px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: #666;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.file-tab:hover {
  background: rgba(0, 0, 0, 0.05);
}

.file-tab.active {
  background: #4095fe;
  color: #fff;
}

.editor-close {
  width: 28px;
  height: 28px;
  background: transparent;
  border: none;
  color: rgba(0, 0, 0, 0.5);
  font-size: 16px;
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.1s, color 0.1s;
  flex-shrink: 0;
}

.editor-close:hover {
  background: rgba(0, 0, 0, 0.05);
  color: #333;
}

.editor-loading {
  padding: 40px 20px;
  text-align: center;
  color: rgba(0, 0, 0, 0.5);
  font-size: 14px;
}

.editor-error {
  padding: 16px 20px;
  color: #e01e5a;
  font-size: 13px;
  text-align: center;
}

.editor-textarea {
  flex: 1;
  width: 100%;
  padding: 16px 20px;
  border: none;
  font-size: 13px;
  font-family: monospace;
  line-height: 1.5;
  resize: none;
  outline: none;
  background: #fafafa;
  color: #333;
  min-height: 200px;
}

.editor-textarea:focus {
  background: #fff;
}

.editor-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 12px 16px;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
  flex-shrink: 0;
}

.cancel-btn {
  padding: 8px 16px;
  background: transparent;
  border: 1px solid rgba(0, 0, 0, 0.2);
  border-radius: 6px;
  font-size: 13px;
  color: #666;
  cursor: pointer;
}

.cancel-btn:hover {
  background: rgba(0, 0, 0, 0.03);
}

.save-btn {
  padding: 8px 20px;
  background: #2eb67d;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 600;
  color: #fff;
  cursor: pointer;
}

.save-btn:hover:not(:disabled) {
  opacity: 0.85;
}

.save-btn:disabled {
  background: rgba(0, 0, 0, 0.1);
  color: rgba(0, 0, 0, 0.3);
  cursor: not-allowed;
}

.group-members-list {
  padding: 4px 0 4px 28px;
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
  color: #797979;
}

.group-member-empty {
  font-size: 12px;
  color: #666;
  padding: 2px 8px;
}

/* ZeroClaw session avatar */
.zeroclaw-avatar {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
}

/* ZeroClaw section header */
.zeroclaw-section-header {
  font-size: 12px;
  font-weight: 600;
  color: #667eea;
  padding: 8px 12px;
  margin-top: 8px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

/* zAgent create dialog */
.zagent-create-dialog {
  width: 500px;
}

.zagent-form-body {
  padding: 16px 20px 8px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
  max-height: calc(80vh - 120px);
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.form-label {
  color: #555;
  font-size: 13px;
  font-weight: 600;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.char-count {
  font-size: 11px;
  font-weight: 400;
  color: rgba(0, 0, 0, 0.35);
}

.char-count-warn {
  color: #e01e5a;
}

.desc-textarea {
  resize: vertical;
  min-height: 80px;
  line-height: 1.5;
  font-family: inherit;
}

.required {
  color: #e01e5a;
  margin-left: 2px;
}

.form-select {
  appearance: none;
  cursor: pointer;
}

.model-config-section {
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 8px;
  overflow: hidden;
}

.model-config-toggle {
  width: 100%;
  background: rgba(102, 126, 234, 0.06);
  border: none;
  padding: 10px 14px;
  text-align: left;
  font-size: 13px;
  font-weight: 600;
  color: #444;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: background 0.15s;
}

.model-config-toggle:hover {
  background: rgba(102, 126, 234, 0.12);
}

.toggle-icon {
  font-size: 11px;
  color: #667eea;
}

.model-config-fields {
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: #fafafa;
}

.form-error {
  color: #e01e5a;
  font-size: 12px;
  margin: 0;
  padding: 4px 0;
}

/* zAgent status indicator */
.zagent-item-wrapper {
  margin: 0 6px 2px;
}

.zagent-item {
  height: auto;
  min-height: 40px;
  padding: 6px 12px;
  margin: 0;
  align-items: flex-start;
}

.zagent-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.zagent-name-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.zagent-name-row .item-name {
  flex: 1;
  min-width: 0;
}

.zagent-status-text {
  font-size: 11px;
  color: #9CA3AF;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.3;
}

.zagent-status-text.status-running  { color: #10B981; }
.zagent-status-text.status-starting { color: #F59E0B; }
.zagent-status-text.status-error    { color: #EF4444; }
</style>
