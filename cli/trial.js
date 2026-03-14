var CONFIG_PATHNAME = `${os.home()}/.ztm.conf`

var DEFAULT_REG_URL = 'https://join.clawparty.ai'

var config = null
try { config = JSON.decode(os.read(CONFIG_PATHNAME)) } catch {}
if (!config || typeof config !== 'object') config = {}

function getConfig() {
  return {
    trial: config.trial || DEFAULT_REG_URL,
  }
}

var trial = null

function getHost(regUrl) {
  var host = regUrl || os.env.ZTM_TRIAL || getConfig().trial
  if (host.startsWith(':')) return 'https://join.clawparty.ai' + host
  if (!Number.isNaN(Number.parseInt(host))) return 'https://join.clawparty.ai:' + host
  // bare host:port (no scheme) — use http for custom reg URLs, https for default
  if (!host.includes('://')) {
    return (regUrl ? 'http://' : 'https://') + host
  }
  return host
}

function getTlsOptions(host) {
  if (host.startsWith('https://')) {
    var url = new URL(host)
    return {
      tls: {
        sni: url.hostname,
        verify: (ok, cert) => true,
      }
    }
  }
  return null
}

function makeAgent(regUrl) {
  var host = getHost(regUrl)
  var options = getTlsOptions(host)
  var url = new URL(host)
  var target = url.hostname + (url.port ? ':' + url.port : '')
  return { agent: new http.Agent(target, options), host, url }
}

function getTrial(regUrl) {
  // When a custom regUrl is given, always create a fresh agent for that URL.
  // Otherwise, reuse the cached default agent.
  if (regUrl) return makeAgent(regUrl)
  if (!trial) trial = makeAgent(null)
  return trial
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
  host: (regUrl) => getHost(regUrl),
  post: (path, body, regUrl) => {
    var t = getTrial(regUrl)
    var base = t.url ? t.url.pathname : ''
    if (base.endsWith('/')) base = base.slice(0, -1)
    var urlPath = base + path
    return t.agent.request('POST', urlPath, { 'Content-Type': 'application/json' }, body).then(check)
  },
}