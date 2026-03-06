import axios from 'axios'

const API_TOKEN_KEY = 'ztm_api_token'

let apiToken = (typeof localStorage !== 'undefined' && localStorage.getItem(API_TOKEN_KEY)) || ''

export function setApiToken(token) {
  apiToken = token || ''
  if (apiToken) {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(API_TOKEN_KEY, apiToken)
    }
  } else {
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem(API_TOKEN_KEY)
    }
  }
}

export function getApiToken() {
  return apiToken
}

const api = axios.create({
  baseURL: '/api',
  timeout: 120000
})

api.interceptors.request.use(config => {
  if (apiToken) {
    config.headers = config.headers || {}
    config.headers.Authorization = `Bearer ${apiToken}`
  }
  return config
})

export const meshService = {
  getMeshes() {
    return api.get('/meshes')
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
            console.error('解析agents数据失败:', e)
            response.data = []
          }
        } else {
          response.data = []
        }
      }
      return response
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

  botChat(currentAgentId, targetAgentId, text) {
    return api.post(`/openclaw/bot-chat/${currentAgentId}/${targetAgentId}`, text, {
      headers: { 'Content-Type': 'text/plain' }
    })
  }
}

export const chatService = {
  getChats(meshName) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/chats`)
  },
  
  getUsers(meshName) {
    return api.get(`/meshes/${meshName}/users?limit=100`)
  },
  
  getMessages(meshName, peer) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/peers/${peer}/messages`)
  },
  
  getMessagesSince(meshName, peer, since) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/peers/${peer}/messages?since=${since}`)
  },
  
  getGroupMessages(meshName, creator, groupId) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/groups/${creator}/${groupId}/messages`)
  },
  
  getGroupMessagesSince(meshName, creator, groupId, since) {
    return api.get(`/meshes/${meshName}/apps/ztm/chat/api/groups/${creator}/${groupId}/messages?since=${since}`)
  },
  
  sendMessage(meshName, peer, text) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/peers/${peer}/messages`, { text })
  },
  
  sendGroupMessage(meshName, creator, groupId, text) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/groups/${creator}/${groupId}/messages`, { text })
  },
  
  createGroup(meshName, creator, groupId, data) {
    return api.post(`/meshes/${meshName}/apps/ztm/chat/api/groups/${creator}/${groupId}`, data)
  }
}

export default api
