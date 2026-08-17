import axios from 'axios'
import { get, post, del, put, setToken, getToken, getShareToken, getMetaUrl } from './request'
import { wikiService } from './wikiService.js'
export function setApiToken(token) {
  return setToken(token)
}

export function getApiToken() {
  return getToken()
}

// const api = axios.create({
//   baseURL: '/api',
//   timeout: 120000
// })
const api = {
	get, post, del, put
}

export const meshService = {
  getMeshes() {
    return api.get('/meshes')
  },

  joinParty(regUrl, userName, inviteCode) {
    return api.post('/join-party', { regUrl, userName, inviteCode })
  },

  leaveMesh(meshName) {
    return api.del(`/meshes/${encodeURIComponent(meshName)}`)
  }
}

export const openclawService = {
  getAgents() {
    return api.get('/openclaw/agents').then(response => {
      if (typeof response.data === 'string') {
        const bracketIndex = response.data.indexOf('[')
        if (bracketIndex !== -1) {
          const jsonStr = response.data.slice(bracketIndex)
          try {
            response.data = JSON.parse(jsonStr)
          } catch (e) {
            console.error('[ChatService] 解析agents数据失败:', e)
            response.data = []
          }
        } else {
          response.data = []
        }
      }
      return response
    })
  },
  
  deleteAgent(agentId) {
    return api.del(`/openclaw/agents/${encodeURIComponent(agentId)}`)
  },

  getAgentWorkspaceFile(agentId, filename) {
    return api.get(`/openclaw/agents/${encodeURIComponent(agentId)}/workspace/${encodeURIComponent(filename)}`, {
      responseType: 'text'
    })
  },

  saveAgentWorkspaceFile(agentId, filename, content) {
    return api.post(`/openclaw/agents/${encodeURIComponent(agentId)}/workspace/${encodeURIComponent(filename)}`, content, {
      headers: { 'Content-Type': 'text/plain' }
    })
  },

  sendMessage(agentId, text) {
    return api.post(`/openclaw/chat/${agentId}`, text, {
      headers: { 'Content-Type': 'text/plain' }
    })
  },
  
  getMessages(agentId) {
    return api.get(`/openclaw/agents/${agentId}/messages`)
  },
  
  getMessagesSince(agentId, since) {
    return api.get(`/openclaw/agents/${agentId}/messages?since=${since}`)
  },

  getSessions(agentId) {
    return api.get(`/openclaw/session/${agentId}`)
  },

  getSessionHistory(agentId, sessionId) {
    return api.get(`/openclaw/session-history/${agentId}/${sessionId}`, {
      responseType: 'text'
    })
  },

  getChatLog(agentId) {
    return api.get(`/openclaw/${agentId}/chat-log`)
  },

  uploadPicture(agentId, fileData, fileName) {
    return api.post(`/openclaw/agents/${agentId}/pictures?name=${encodeURIComponent(fileName)}`, fileData, {
      headers: { 'Content-Type': 'application/octet-stream' },
      transformRequest: [data => data]
    })
  },

  getPictureUrl(agentId, fileName) {
    const token = getToken() ? `?token=${encodeURIComponent(getToken())}` : ''
    return `/api/openclaw/agents/${agentId}/pictures/${encodeURIComponent(fileName)}${token}`
  }
}

export const picoclawService = {
  checkHealth() {
    return api.get('/picoclaw/health')
  },
  
  sendMessage(message, sessionId) {
    return api.post('/picoclaw/chat', { 
      message, 
      session_id: sessionId 
    })
  }
}

export const zeroclawService = {
  checkHealth() {
    return api.get('/zeroclaw/health')
  },

  getSessions() {
    return api.get('/zeroclaw/sessions')
  },

  getMessages(agentName, sessionId) {
    if (!sessionId) sessionId = 'me'
    return api.get(`/zeroclaw/messages?agent=${encodeURIComponent(agentName)}&session=${encodeURIComponent(sessionId)}`)
  }
}

export class ZeroClawWS {
  constructor(agentName, sessionId, onMessage, onOpen, onClose, onError, wsPort) {
    this.agentName = agentName
    this.sessionId = sessionId
    this.onMessage = onMessage
    this.onOpen = onOpen
    this.onClose = onClose
    this.onError = onError
    this.wsPort = wsPort
    this.ws = null
    // Reconnect: infinite with exponential backoff (1s → 30s cap + jitter)
    this.reconnectAttempts = 0
    this.maxReconnectAttempts = Infinity
    this.reconnectDelay = 1000
    this.maxReconnectDelay = 30000
    this.reconnectTimer = null
    this.destroyed = false
    this.everConnected = false
    // Heartbeat: 25s interval, 2 missed = dead connection → force reconnect
    this.heartbeatIntervalMs = 25000
    this.heartbeatTimer = null
    this.missedHeartbeats = 0
    this.maxMissedHeartbeats = 2
    // Callbacks for UI status banner (optional)
    this.onStateChange = null   // (state, info) => void  state: 'connecting'|'connected'|'reconnecting'|'destroyed'
    this.onReconnected = null   // () => void  fired after a successful reconnect (not first connect)
  }

  connect() {
    if (this.destroyed) return
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const host = window.location.host
    const isTauri = !!window.__TAURI_INTERNALS__
    const shouldUseDirectPort = isTauri && this.wsPort
    const url = shouldUseDirectPort
      ? `${protocol}//localhost:${this.wsPort}/ws/chat?agent=${encodeURIComponent(this.agentName)}&session_id=${encodeURIComponent(this.sessionId)}`
      : `${protocol}//${host}/ws/chat?agent=${encodeURIComponent(this.agentName)}&session_id=${encodeURIComponent(this.sessionId)}`
    
    console.log('[zAgentWS] Connecting to:', url)
    this._emitState(this.everConnected ? 'reconnecting' : 'connecting')
    
    try {
      this.ws = new WebSocket(url, 'zeroclaw.v1')
      
      this.ws.onopen = () => {
        console.log('[zAgentWS] Connected')
        const wasReconnect = this.everConnected
        this.everConnected = true
        this.reconnectAttempts = 0
        this.missedHeartbeats = 0
        this._startHeartbeat()
        this._emitState('connected')
        this.onOpen?.()
        if (wasReconnect) {
          this.onReconnected?.()
        }
      }
      
      this.ws.onmessage = (event) => {
        // Any inbound message proves the connection is alive
        this.missedHeartbeats = 0
        try {
          // Handle binary or non-string data
          if (typeof event.data !== 'string') {
            console.warn('[zAgentWS] Received non-string data:', typeof event.data)
            return
          }
          // Skip empty messages
          if (!event.data || event.data.trim().length === 0) {
            return
          }
          const data = JSON.parse(event.data)
          // Heartbeat pong is handled internally, don't surface to business layer
          if (data.type === 'pong') return
          console.log('[zAgentWS] Received:', data.type)
          this.onMessage?.(data)
        } catch (e) {
          console.error('[zAgentWS] Parse error:', e, 'data:', event.data?.substring?.(0, 200))
        }
      }
      
      this.ws.onclose = (event) => {
        console.log('[zAgentWS] Closed:', event.code, event.reason)
        this._stopHeartbeat()
        this.ws = null
        this.onClose?.(event)
        // Always reconnect (incl. code 1000) unless explicitly destroyed —
        // server restarts / reloads send 1000 and we must recover.
        if (!this.destroyed) {
          this._scheduleReconnect()
        }
      }
      
      this.ws.onerror = (error) => {
        console.error('[zAgentWS] Error:', error)
        this.onError?.(error)
      }
    } catch (e) {
      console.error('[zAgentWS] Connection error:', e)
      this.onError?.(e)
      if (!this.destroyed) {
        this._scheduleReconnect()
      }
    }
  }

  sendMessage(content, options = {}) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      const msg = JSON.stringify({ type: 'message', content, ...options })
      console.log('[zAgentWS] Sending:', msg.substring(0, 100))
      this.ws.send(msg)
      return true
    } else {
      console.error('[zAgentWS] Cannot send - not connected')
      return false
    }
  }

  close() {
    // Deprecated alias — closing a connection must tear it down for good,
    // otherwise onclose re-arms _scheduleReconnect and the old connection
    // keeps reconnecting in the background. Route through destroy().
    this.destroy()
  }

  /** Explicitly destroy the client — no further reconnects. */
  destroy() {
    this.destroyed = true
    this._clearReconnectTimer()
    this._stopHeartbeat()
    if (this.ws) {
      const ws = this.ws
      this.ws = null
      try { ws.close(1000, 'client-destroy') } catch (e) { /* ignore */ }
      setTimeout(() => {
        if (ws.readyState !== WebSocket.CLOSED) {
          ws.onopen = ws.onmessage = ws.onerror = ws.onclose = null
        }
      }, 3000)
    }
    this._emitState('destroyed')
  }

  isConnected() {
    return this.ws && this.ws.readyState === WebSocket.OPEN
  }

  // ---------------- internal ----------------

  _scheduleReconnect() {
    if (this.destroyed) return
    this._clearReconnectTimer()
    this.reconnectAttempts += 1
    // Exponential backoff: 1s, 2s, 4s, 8s ... capped at 30s, ±25% jitter
    const exp = Math.min(this.reconnectDelay * 2 ** (this.reconnectAttempts - 1), this.maxReconnectDelay)
    const jitter = exp * (0.75 + Math.random() * 0.5)
    const delay = Math.min(Math.round(jitter), this.maxReconnectDelay)
    console.log(`[zAgentWS] Reconnect #${this.reconnectAttempts} in ${delay}ms`)
    this._emitState('reconnecting', { nextRetryMs: delay })
    this.reconnectTimer = setTimeout(() => this.connect(), delay)
  }

  _startHeartbeat() {
    this._stopHeartbeat()
    this.heartbeatTimer = setInterval(() => {
      if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return
      this.missedHeartbeats += 1
      if (this.missedHeartbeats > this.maxMissedHeartbeats) {
        // Half-open connection: proactively close to trigger reconnect
        console.warn('[zAgentWS] Heartbeat timeout, forcing reconnect')
        const ws = this.ws
        this.ws = null
        ws.onopen = ws.onmessage = ws.onerror = ws.onclose = null
        try { ws.close(4000, 'heartbeat-timeout') } catch (e) { /* ignore */ }
        this._stopHeartbeat()
        this._scheduleReconnect()
        return
      }
      try { this.ws.send(JSON.stringify({ type: 'ping', ts: Date.now() })) } catch (e) { /* ignore */ }
    }, this.heartbeatIntervalMs)
  }

  _stopHeartbeat() {
    if (this.heartbeatTimer) { clearInterval(this.heartbeatTimer); this.heartbeatTimer = null }
    this.missedHeartbeats = 0
  }

  _clearReconnectTimer() {
    if (this.reconnectTimer) { clearTimeout(this.reconnectTimer); this.reconnectTimer = null }
  }

  _emitState(state, info = {}) {
    this.onStateChange?.(state, { attempt: this.reconnectAttempts, nextRetryMs: null, ...info })
  }
}

export const taskService = {
  getAgentTasks(agentName, groupId) {
    var url = `/tasks?agent=${encodeURIComponent(agentName)}`
    if (groupId) url += `&group=${encodeURIComponent(groupId)}`
    return api.get(url)
  },

  createTask(taskData) {
    return api.post('/tasks', taskData)
  },

  updateTask(taskId, updates) {
    return api.put(`/tasks/${encodeURIComponent(taskId)}`, updates)
  },

  deleteTask(taskId) {
    return api.del(`/tasks/${encodeURIComponent(taskId)}`)
  },

  getTaskEvents(taskId) {
    return api.get(`/tasks/${encodeURIComponent(taskId)}/events`)
  },

  getAnalysisLog(agentName, groupId) {
    var url = `/task/analysis?agent=${encodeURIComponent(agentName)}`
    if (groupId) url += `&group=${encodeURIComponent(groupId)}`
    return api.get(url)
  },

  setAnalysisLog(agentName, groupId, lastAnalyzedAt) {
    return api.put('/task/analysis', {
      agent_name: agentName,
      group_id: groupId || null,
      last_analyzed_at: lastAnalyzedAt,
    })
  },

  generatePrompt(taskId) {
    return api.post(`/tasks/${encodeURIComponent(taskId)}/generate-prompt`)
  },

  getTaskExecutionLogs(taskId) {
    return api.get(`/tasks/${encodeURIComponent(taskId)}/execution-logs`)
  },

  getTaskChatLogs(taskId) {
    return api.get(`/tasks/${encodeURIComponent(taskId)}/chat-logs`)
  },

  addTaskChatLog(taskId, sender, content, msgType) {
    return api.post(`/tasks/${encodeURIComponent(taskId)}/chat-logs`, {
      sender: sender,
      content: content,
      msg_type: msgType || 'assistant',
    })
  },

  batchRefresh(agentName, groupId, lastAnalyzedAt, changes) {
    return api.post('/tasks/batch-refresh', {
      agent_name: agentName,
      group_id: groupId || null,
      last_analyzed_at: lastAnalyzedAt,
      changes: changes || []
    })
  },
}

export const kanbanService = {
  getKanbanConfig(agentName, groupId) {
    var url = `/kanban?agent=${encodeURIComponent(agentName)}`
    if (groupId) url += `&group=${encodeURIComponent(groupId)}`
    return api.get(url)
  },

  setKanbanConfig(agentName, groupId, name, prompt, config) {
    return api.put('/kanban', {
      agent_name: agentName,
      group_id: groupId || null,
      name: name,
      prompt: prompt,
      config: config,
    })
  },
}

export const workspaceService = {
  saveFile(agentName, filename, content) {
    return api.post(`/agents/${encodeURIComponent(agentName)}/workspace/${encodeURIComponent(filename)}`, content, {
      headers: { 'Content-Type': 'text/plain' }
    })
  }
}

export const zagentService = {
  getAgents() {
    return api.get('/agents')
  },
  createAgent(config) {
    return api.post('/agents', {
      agent_name: config.agent_name,
      display_name: config.display_name,
      description: config.description || null,
      provider: config.provider || null,
      api_endpoint: config.api_endpoint || null,
      api_key: config.api_key || null,
      model: config.model || null,
      soul_content: config.soul_content || null
    })
  },
  deleteAgent(name) {
    return api.del(`/agents/${encodeURIComponent(name)}`)
  },
  startAgent(name) {
    return api.post(`/agents/${encodeURIComponent(name)}/start`)
  },
  stopAgent(name) {
    return api.post(`/agents/${encodeURIComponent(name)}/stop`)
  },
  getGlobalConfig() {
    return api.get('/global-config')
  },

  reconcileAgents() {
    return api.post('/agents/reconcile')
  }
}

export const chatService = {
  getChats(meshName) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/chats`)
  },
  
  getUsers(meshName) {
    return api.get(`/meshes/${meshName}/endpoints?limit=500`)
  },
  
  getMessages(meshName, peer) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/peers/${peer}/messages`)
  },
  
  getMessagesSince(meshName, peer, since) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/peers/${peer}/messages?since=${since}`)
  },
  
  getGroupMessages(meshName, creator, groupId) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/groups/${encodeURIComponent(creator)}/${encodeURIComponent(groupId)}/messages`)
  },
  
  getGroupMessagesSince(meshName, creator, groupId, since) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/groups/${encodeURIComponent(creator)}/${encodeURIComponent(groupId)}/messages?since=${since}`)
  },
  
  sendMessage(meshName, peer, text, sessionId, files) {
    const body = { text, sessionId: sessionId || null }
    if (files && files.length > 0) body.files = files
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/peers/${encodeURIComponent(peer)}/messages`, body)
  },
  
  sendGroupMessage(meshName, creator, groupId, text, sessionId, files) {
    const body = { text, sessionId: sessionId || null }
    if (files && files.length > 0) body.files = files
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/groups/${encodeURIComponent(creator)}/${encodeURIComponent(groupId)}/messages`, body)
  },

  uploadFile(meshName, fileData) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/files`, fileData, {
      headers: { 'Content-Type': 'application/octet-stream' },
      transformRequest: [data => data]
    })
  },

  uploadFileToSession(meshName, fileData, sessionId, fileName) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/files/upload?sessionId=${encodeURIComponent(sessionId)}&name=${encodeURIComponent(fileName)}`, fileData, {
      headers: { 'Content-Type': 'application/octet-stream' },
      transformRequest: [data => data]
    })
  },

  getFileUrl(meshName, owner, hash) {
    const token = getToken() ? `?token=${encodeURIComponent(getToken())}` : ''
    return `/api/meshes/${meshName}/apps/ztm/chat/api/files/${owner}/${hash}${token}`
  },

  getFileFromSessionUrl(meshName, sessionId, hash) {
    const token = getToken() ? `?token=${encodeURIComponent(getToken())}` : ''
    return `/api/meshes/${meshName}/apps/ztm/chat/api/files/upload/${encodeURIComponent(sessionId)}/${hash}${token}`
  },
  
  createGroup(meshName, creator, groupId, data) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/groups/${encodeURIComponent(creator)}/${encodeURIComponent(groupId)}`, data)
  },

  deleteGroup(meshName, creator, groupId) {
    return api.del(`/meshes/${meshName}/apps/ztm/chat/api/groups/${encodeURIComponent(creator)}/${encodeURIComponent(groupId)}`)
  },

  leaveGroup(meshName, creator, groupId) {
    return api.del(`/meshes/${meshName}/apps/ztm/chat/api/groups/${encodeURIComponent(creator)}/${encodeURIComponent(groupId)}?leave=1`)
  },

  updateGroupMembers(meshName, creator, groupId, name, members) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/groups/${encodeURIComponent(creator)}/${encodeURIComponent(groupId)}`, { name, members })
  },

  approvePeerAutoReply(meshName, peer, agentName, peerAgentName) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/peers/${peer}/auto-reply`, {
      autoReply: true,
      autoReplyAgent: agentName || 'main',
      peerAgentName: peerAgentName || agentName || 'main'
    })
  },

  approveGroupAgentAutoReply(meshName, gcid, agentName) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/groupchat/${gcid}/agents/${agentName}/auto-reply`)
  },

  revokeGroupAgentAutoReply(meshName, gcid, agentName) {
    return api.del(`/meshes/${meshName}/apps/ztm/chat/api/groupchat/${gcid}/agents/${agentName}/auto-reply`)
  },

  approveGroupEpAutoReply(meshName, gcid, agentName) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/groupchat/${gcid}/auto-reply`, { agent: agentName || 'main' })
  },

  revokeGroupEpAutoReply(meshName, gcid) {
    return api.del(`/meshes/${meshName}/apps/ztm/chat/api/groupchat/${gcid}/auto-reply`)
  },

  getAllPeerConfigs(meshName) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/auto-reply`)
  },

  getPeerConfig(meshName, peer) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/peers/${encodeURIComponent(peer)}/auto-reply`)
  },

  updatePeerConfig(meshName, peer, config) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/peers/${encodeURIComponent(peer)}/auto-reply`, config)
  },

  halfAutomationRewrite(meshName, peer, draftText, humanHint, sessionId) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/peers/${encodeURIComponent(peer)}/half-rewrite`, {
      draftText,
      humanHint,
      sessionId
    })
  },
}

export const webshareService = {
  getAgentWebshareList(agentName, path) {
    var url = `/webshare/${encodeURIComponent(agentName)}/list`
    if (path) url += `?path=${encodeURIComponent(path)}`
    return api.get(url)
  },
  getAgentWebshareFileUrl(agentName, filename, path) {
    var token = getShareToken() ? `token=${encodeURIComponent(getShareToken())}` : ''
    var url = `/webshare/${encodeURIComponent(agentName)}/file/${encodeURIComponent(filename)}`
    var sep = '?'
    if (path) {
      url += `?path=${encodeURIComponent(path)}`
      sep = '&'
    }
    if (token) url += sep + token
    if (typeof window !== 'undefined' && !window.__TAURI_INTERNALS__) {
      url = getMetaUrl(url)
    }
    return url
  },
  getAgentWebshareFileContent(agentName, filename, path) {
    var url = `/webshare/${encodeURIComponent(agentName)}/file/${encodeURIComponent(filename)}`
    if (path) url += `?path=${encodeURIComponent(path)}`
    return api.get(url, { responseType: 'arraybuffer' })
  },
  uploadAgentWebshareFile(agentName, fileData, fileName, path) {
    var url = `/webshare/${encodeURIComponent(agentName)}/upload?name=${encodeURIComponent(fileName)}`
    if (path) url += `&path=${encodeURIComponent(path)}`
    return api.post(url, fileData, {
      headers: { 'Content-Type': 'application/octet-stream' },
      transformRequest: [function(data) { return data }]
    })
  }
}

export const groupChatService = {
  getGroupChats() {
    return api.get('/groupchats')
  },
  createGroupChat(groupName, memberAgents) {
    const groupId = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
      const r = Math.random() * 16 | 0
      const v = c === 'x' ? r : (r & 0x3 | 0x8)
      return v.toString(16)
    })
    return api.post('/groupchats', {
      group_id: groupId,
      group_name: groupName,
      owner_agent: memberAgents[0] || '',
      members: memberAgents
    })
  },
  getGroupChat(groupId) {
    return api.get(`/groupchats/${encodeURIComponent(groupId)}`)
  },
  updateGroupChat(groupId, data) {
    return api.put(`/groupchats/${encodeURIComponent(groupId)}`, data)
  },
  deleteGroupChat(groupId) {
    return api.del(`/groupchats/${encodeURIComponent(groupId)}`)
  },
  addMembers(groupId, memberNames) {
    return api.post(`/groupchats/${encodeURIComponent(groupId)}/members`, { members: memberNames })
  },
  removeMember(groupId, agentName) {
    return api.del(`/groupchats/${encodeURIComponent(groupId)}/members/${encodeURIComponent(agentName)}`)
  },
  getGroupMessages(groupId) {
    return api.get(`/groupchats/${encodeURIComponent(groupId)}/messages`)
  },
  sendGroupMessage(groupId, sender, content, msgType) {
    return api.post(`/groupchats/${encodeURIComponent(groupId)}/messages`, {
      sender,
      content,
      msg_type: msgType || 'user'
    })
  }
}

export const toolCallService = {
  getSessionToolCalls(sessionId) {
    return api.get(`/sessions/${encodeURIComponent(sessionId)}/tool-calls`)
  }
}

export { wikiService }

export default api
