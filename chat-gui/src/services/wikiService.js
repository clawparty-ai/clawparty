import { get, post, put, del, getMetaUrl } from './request.js'

export const wikiService = {
  initWiki(agentName) {
    return post(`/wiki/${encodeURIComponent(agentName)}/init`)
  },

  getTree(agentName, path) {
    var url = `/wiki/${encodeURIComponent(agentName)}/tree`
    if (path) url += `?path=${encodeURIComponent(path)}`
    return get(url)
  },

  getPage(agentName, filename, path) {
    var url = `/wiki/${encodeURIComponent(agentName)}/file/${encodeURIComponent(filename)}`
    if (path) url += `?path=${encodeURIComponent(path)}`
    return get(url, { responseType: 'text' })
  },

  search(agentName, query) {
    return get(`/wiki/${encodeURIComponent(agentName)}/search?q=${encodeURIComponent(query)}`)
  },

  getGraph(agentName) {
    return get(`/wiki/${encodeURIComponent(agentName)}/graph`)
  },

  refresh(agentName) {
    return post(`/wiki/${encodeURIComponent(agentName)}/refresh`)
  },

  uploadRaw(agentName, fileData, fileName) {
    var url = `/wiki/${encodeURIComponent(agentName)}/upload?name=${encodeURIComponent(fileName)}`
    return post(url, fileData, {
      headers: { 'Content-Type': 'application/octet-stream' },
      transformRequest: [function(data) { return data }]
    })
  }
}
