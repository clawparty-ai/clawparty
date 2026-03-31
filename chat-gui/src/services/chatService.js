import axios from 'axios'
import { get, post, del, put, setToken, getToken } from './request'
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

  joinParty(regUrl) {
    return api.post('/join-party', { regUrl })
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

export default api
