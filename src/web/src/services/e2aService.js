import { get, post, del } from './request.js'

export const e2aService = {
  upload(agentName, fileData, fileName) {
    const url = `/e2a/${encodeURIComponent(agentName)}/upload?name=${encodeURIComponent(fileName)}`
    return post(url, fileData, {
      headers: { 'Content-Type': 'application/octet-stream' },
      transformRequest: [function(data) { return data }]
    })
  },

  list(agentName) {
    return get(`/e2a/${encodeURIComponent(agentName)}/list`)
  },

  getFile(agentName, dataset, filename) {
    const url = `/e2a/${encodeURIComponent(agentName)}/file/${encodeURIComponent(dataset)}/${encodeURIComponent(filename)}`
    return get(url, {}, { responseType: 'text' })
  },

  deleteDataset(agentName, dataset) {
    const url = `/e2a/${encodeURIComponent(agentName)}/${encodeURIComponent(dataset)}`
    return del(url)
  }
}
