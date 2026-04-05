import { get, post } from './request'

const api = { get, post }

export const templateService = {
  getLocalTemplates() {
    return api.get('/agent-templates/local')
  },

  getSharedTemplates() {
    return api.get('/agent-templates/shared')
  },

  installLocalTemplate(industry, agent, soulContent = '') {
    return api.post(`/agent-templates/local/${encodeURIComponent(industry)}/${encodeURIComponent(agent)}/install`, {
      soulContent,
    })
  },

  installSharedTemplate(industry, agent, soulContent = '') {
    return api.post(`/agent-templates/shared/${encodeURIComponent(industry)}/${encodeURIComponent(agent)}/install`, {
      soulContent,
    })
  },
}
