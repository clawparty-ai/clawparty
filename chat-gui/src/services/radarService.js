import { get, post } from './request'

export const radarService = {
  initRadar(agentName) {
    return post(`/radar/${encodeURIComponent(agentName)}/init`)
  },
  getProbes(agentName) {
    return get(`/radar/${encodeURIComponent(agentName)}/probes`)
  },
  getTargetsMd(agentName) {
    return get(`/radar/${encodeURIComponent(agentName)}/targets-md`)
  },
  getTargetsJson(agentName) {
    return get(`/radar/${encodeURIComponent(agentName)}/targets-json`)
  },
  getLogs(agentName) {
    return get(`/radar/${encodeURIComponent(agentName)}/logs`)
  },
  getLog(agentName, filename) {
    return get(`/radar/${encodeURIComponent(agentName)}/logs/${encodeURIComponent(filename)}`)
  }
}
