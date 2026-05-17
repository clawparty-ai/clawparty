import { get, post, del } from './request'

export const radarService = {
  initRadar(agentName) {
    return post(`/radar/${encodeURIComponent(agentName)}/init`)
  },
  getTargets(agentName) {
    return get(`/radar/${encodeURIComponent(agentName)}/targets`)
  },
  getTargetInfo(agentName, targetName) {
    return get(`/radar/${encodeURIComponent(agentName)}/targets/${encodeURIComponent(targetName)}`)
  },
  getTargetLog(agentName, targetName) {
    return get(`/radar/${encodeURIComponent(agentName)}/targets/${encodeURIComponent(targetName)}/log`)
  },
  createTarget(agentName, targetName, info) {
    return post(`/radar/${encodeURIComponent(agentName)}/targets/${encodeURIComponent(targetName)}`, info)
  },
  deleteTarget(agentName, targetName) {
    return del(`/radar/${encodeURIComponent(agentName)}/targets/${encodeURIComponent(targetName)}`)
  },
  getScans(agentName) {
    return get(`/radar/${encodeURIComponent(agentName)}/scans`)
  },
  getDiscoveries(agentName) {
    return get(`/radar/${encodeURIComponent(agentName)}/discoveries`)
  }
}
