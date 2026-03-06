var CONFIG_PATHNAME = `${os.home()}/.ztm.conf`

var config = null
try { config = JSON.decode(os.read(CONFIG_PATHNAME)) } catch {}
if (!config || typeof config !== 'object') config = {}

function getConfig() {
  return {
    agent: config.agent || 'localhost:6789',
    mesh: config.mesh,
    token: config.token,
  }
}

var agent = null

function getHost() {
  var host = os.env.ZTM_AGENT || getConfig().agent
  if (host.startsWith(':')) return 'localhost' + host
  if (!Number.isNaN(Number.parseInt(host))) return 'localhost:' + host
  return host
}

function getMesh() {
  return os.env.ZTM_MESH || getConfig().mesh
}

function getToken() {
  return os.env.ZTM_API_TOKEN || config.token || ''
}

function getHeaders() {
  var token = getToken()
  if (!token) return null
  token = String(token).trim()
  if (!token) return null
  return {
    authorization: `Bearer ${token}`,
    'x-ztm-token': token,
  }
}

function getAgent() {
  if (!agent) {
    agent = new http.Agent(getHost())
  }
  return agent
}

function check(res) {
  if (res.head.status >= 400) {
    var message
    try {
      var json = JSON.decode(res.body)
      message = json.message || res.body.toString()
    } catch {
      message = res.body.toString()
    }
    throw {
      status: res.head.status,
      message: message || res.head.statusText,
    }
  }
  return res.body
}

export default {
  config: (c) => {
    if (c) {
      if (c.agent) config.agent = c.agent
      if (c.mesh) config.mesh = c.mesh
      if (c.token) config.token = c.token
      os.write(CONFIG_PATHNAME, JSON.encode(config, null, 2))
    } else {
      return getConfig()
    }
  },

  host: getHost,
  mesh: getMesh,

  get: (path) => getAgent().request('GET', path, getHeaders()).then(check),
  post: (path, body) => getAgent().request('POST', path, getHeaders(), body).then(check),
  delete: (path) => getAgent().request('DELETE', path, getHeaders()).then(check),
}
