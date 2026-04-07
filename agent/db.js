var db = null

function open(pathname) {
  db = sqlite(pathname)

  db.exec(`
    CREATE TABLE IF NOT EXISTS hubs (
      id TEXT PRIMARY KEY,
      zone TEXT NOT NULL,
      info TEXT NOT NULL,
      updated_at REAL NOT NULL
    )
  `)

  try {
    db.exec(`
      ALTER TABLE hubs
      ADD COLUMN updated_at REAL NOT NULL DEFAULT 0
    `)
  } catch {}

  db.exec(`
    CREATE TABLE IF NOT EXISTS meshes (
      name TEXT PRIMARY KEY,
      ca TEXT NOT NULL,
      agent TEXT NOT NULL,
      bootstraps TEXT NOT NULL
    )
  `)

  db.exec(`
    CREATE TABLE IF NOT EXISTS apps (
      mesh TEXT NOT NULL,
      name TEXT NOT NULL,
      tag TEXT NOT NULL,
      provider TEXT NOT NULL,
      username TEXT NOT NULL,
      state TEXT NOT NULL
    )
  `)

  db.exec(`
    CREATE TABLE IF NOT EXISTS files (
      mesh TEXT NOT NULL,
      provider TEXT NOT NULL,
      app TEXT NOT NULL,
      path TEXT NOT NULL,
      data TEXT NOT NULL
    )
  `)

  db.exec(`
    CREATE TABLE IF NOT EXISTS keys (
      name TEXT PRIMARY KEY,
      data TEXT NOT NULL
    )
  `)

  db.exec(`
    CREATE TABLE IF NOT EXISTS api_log (
      id        INTEGER PRIMARY KEY AUTOINCREMENT,
      timestamp REAL NOT NULL,
      client_ip TEXT NOT NULL,
      api       TEXT NOT NULL,
      req_headers TEXT NOT NULL,
      req_body    TEXT NOT NULL,
      res_headers TEXT NOT NULL,
      res_body    TEXT NOT NULL
    )
  `)

  db.exec(`
    CREATE TABLE IF NOT EXISTS openclaws (
      agent_name    TEXT PRIMARY KEY,
      template_name TEXT NOT NULL,
      type          TEXT NOT NULL DEFAULT 'openclaw',
      api_url       TEXT NOT NULL,
      token         TEXT NOT NULL DEFAULT 'join-party',
      soul_content  TEXT
    )
  `)

  // Migration: add agent_name and template_name columns if they don't exist
  try {
    db.exec('ALTER TABLE openclaws ADD COLUMN agent_name TEXT')
  } catch {}
  try {
    db.exec('ALTER TABLE openclaws ADD COLUMN template_name TEXT')
  } catch {}
  // Migrate existing data: copy name to agent_name and template_name
  try {
    db.exec('UPDATE openclaws SET agent_name = name, template_name = name WHERE agent_name IS NULL')
  } catch {}

  db.exec(`
    CREATE TABLE IF NOT EXISTS chat_peer (
      mesh               TEXT    NOT NULL,
      peer               TEXT    NOT NULL,
      auto_reply         INTEGER NOT NULL DEFAULT 0,
      auto_reply_agent   TEXT    NOT NULL DEFAULT 'main',
      peer_agent_name    TEXT    NOT NULL DEFAULT '',
      is_blocked         INTEGER NOT NULL DEFAULT 0,
      run                INTEGER NOT NULL DEFAULT 1,
      muted              INTEGER NOT NULL DEFAULT 0,
      thinking_time      INTEGER NOT NULL DEFAULT 3,
      peer_profile       TEXT    NOT NULL DEFAULT '',
      short_context      TEXT    NOT NULL DEFAULT '',
      long_context       TEXT    NOT NULL DEFAULT '',
      peer_name          TEXT    NOT NULL DEFAULT '',
      half_automation    INTEGER NOT NULL DEFAULT 0,
      PRIMARY KEY (mesh, peer)
    )
  `)

  // Migrations: add columns if they don't exist yet (for existing databases)
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN peer_agent_name TEXT NOT NULL DEFAULT ''`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN credit INTEGER NOT NULL DEFAULT 100`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN filter_chain TEXT NOT NULL DEFAULT ''`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN send_filter_chain TEXT NOT NULL DEFAULT ''`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN is_blocked INTEGER NOT NULL DEFAULT 0`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN run INTEGER NOT NULL DEFAULT 1`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN muted INTEGER NOT NULL DEFAULT 0`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN thinking_time INTEGER NOT NULL DEFAULT 3`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN peer_profile TEXT NOT NULL DEFAULT ''`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN short_context TEXT NOT NULL DEFAULT ''`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN long_context TEXT NOT NULL DEFAULT ''`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN peer_name TEXT NOT NULL DEFAULT ''`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_peer ADD COLUMN half_automation INTEGER NOT NULL DEFAULT 0`)
  } catch {}

  db.exec(`
    CREATE TABLE IF NOT EXISTS blocked_keywords (
      id      INTEGER PRIMARY KEY AUTOINCREMENT,
      mesh    TEXT    NOT NULL,
      keyword TEXT    NOT NULL,
      UNIQUE(mesh, keyword)
    )
  `)

  db.exec(`
    CREATE TABLE IF NOT EXISTS chat_log (
      id         INTEGER PRIMARY KEY AUTOINCREMENT,
      time       REAL    NOT NULL,
      mesh       TEXT    NOT NULL,
      chat_type  TEXT    NOT NULL,
      chat_id    TEXT    NOT NULL,
      chat_name  TEXT,
      creator    TEXT,
      sender     TEXT    NOT NULL,
      event      TEXT    NOT NULL,
      content    TEXT,
      members    TEXT,
      session_id TEXT,
      muted      INTEGER NOT NULL DEFAULT 0
    )
  `)

  try {
    db.exec(`CREATE INDEX IF NOT EXISTS chat_log_mesh_chat ON chat_log(mesh, chat_type, chat_id)`)
    db.exec(`CREATE INDEX IF NOT EXISTS chat_log_time ON chat_log(time)`)
    db.exec(`CREATE INDEX IF NOT EXISTS chat_log_sender ON chat_log(sender)`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_log ADD COLUMN session_id TEXT`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_log ADD COLUMN muted INTEGER NOT NULL DEFAULT 0`)
  } catch {}
  try {
    db.exec(`ALTER TABLE chat_log ADD COLUMN msg_type TEXT NOT NULL DEFAULT 'response'`)
  } catch {}

  db.exec(`
    CREATE TABLE IF NOT EXISTS cache (
      key   TEXT PRIMARY KEY,
      value TEXT NOT NULL
    )
  `)

  db.exec(`
    CREATE TABLE IF NOT EXISTS cli_log (
      id           INTEGER PRIMARY KEY AUTOINCREMENT,
      timestamp    REAL    NOT NULL,
      agent_name   TEXT    NOT NULL,
      session_id   TEXT    NOT NULL DEFAULT '',
      message_md5  TEXT    NOT NULL
    )
  `)

  try {
    db.exec(`CREATE INDEX IF NOT EXISTS cli_log_lookup ON cli_log(agent_name, session_id, message_md5, timestamp)`)
  } catch {}

  // Migrations: add new columns to cli_log for detailed CLI call logging
  try { db.exec(`ALTER TABLE cli_log ADD COLUMN command TEXT NOT NULL DEFAULT ''`) } catch {}
  try { db.exec(`ALTER TABLE cli_log ADD COLUMN arguments TEXT NOT NULL DEFAULT ''`) } catch {}
  try { db.exec(`ALTER TABLE cli_log ADD COLUMN output TEXT NOT NULL DEFAULT ''`) } catch {}
  try { db.exec(`ALTER TABLE cli_log ADD COLUMN exit_code INTEGER NOT NULL DEFAULT 0`) } catch {}
  try { db.exec(`ALTER TABLE cli_log ADD COLUMN success INTEGER NOT NULL DEFAULT 0`) } catch {}
  try { db.exec(`ALTER TABLE cli_log ADD COLUMN duration_ms INTEGER NOT NULL DEFAULT 0`) } catch {}
  try { db.exec(`ALTER TABLE cli_log ADD COLUMN error_message TEXT NOT NULL DEFAULT ''`) } catch {}
}

function allZones() {
  var t = Date.now() - 30 * 24 * 60 * 60 * 1000
  return (
    db.sql('SELECT DISTINCT zone FROM hubs WHERE updated_at >= ?')
      .bind(1, t)
      .exec()
      .map(r => r.zone)
  )
}

function setZones(zones) {
  // allZones().forEach(
  //   zone => {
  //     if (!zones.includes(zone)) {
  //       db.sql('DELETE FROM hubs WHERE zone = ?')
  //         .bind(1, zone)
  //         .exec()
  //     }
  //   }
  // )
}

function allHubs(zone) {
  var t = Date.now() - 30 * 24 * 60 * 60 * 1000
  var all = {}
  db.sql('SELECT id, info FROM hubs WHERE zone = ? AND updated_at >= ?')
    .bind(1, zone)
    .bind(2, t)
    .exec()
    .forEach(r => {
      try {
        all[r.id] = JSON.parse(r.info)
      } catch {}
    })
  return all
}

function setHubs(zone, hubs) {
  var t = Date.now()
  var old = {}
  db.sql('SELECT id FROM hubs WHERE zone = ?')
    .bind(1, zone)
    .exec()
    .forEach(r => old[r.id] = true)
  Object.entries(hubs).forEach(
    ([id, hub]) => {
      var info = JSON.stringify({ ports: hub.ports, version: hub.version })
      if (id in old) {
        db.sql('UPDATE hubs SET info = ?, updated_at = ? WHERE id = ?')
          .bind(1, info)
          .bind(2, t)
          .bind(3, id)
          .exec()
      } else {
        db.sql('INSERT INTO hubs(id, zone, info, updated_at) VALUES(?, ?, ?, ?)')
          .bind(1, id)
          .bind(2, zone)
          .bind(3, info)
          .bind(4, t)
          .exec()
      }
    }
  )
  // Object.keys(old).forEach(
  //   id => {
  //     if (!(id in hubs)) {
  //       db.sql('DELETE FROM hubs WHERE id = ?')
  //         .bind(1, id)
  //         .exec()
  //     }
  //   }
  // )
}

function getHub(id) {
  return (
    db.sql('SELECT zone, info FROM hubs WHERE id = ?')
      .bind(1, id)
      .exec()
      .map(r => {
        try {
          var hub = JSON.parse(r.info)
        } catch {
          var hub = {}
        }
        return { zone: r.zone, ...hub }
      })[0]
  )
}

function setHub(id, hub) {
  var t = Date.now()
  var old = getHub(id)
  if (old) {
    var zone = hub.zone || old.zone
    var info = {
      ports: hub.ports || old.ports,
      version: hub.version || old.version,
    }
    db.sql('UPDATE hubs SET zone = ?, info = ?, updated_at = ? WHERE id = ?')
      .bind(1, zone)
      .bind(2, JSON.stringify(info))
      .bind(3, t)
      .bind(4, id)
      .exec()
  } else {
    var zone = hub.zone
    var info = {
      ports: hub.ports,
      version: hub.version,
    }
    db.sql('INSERT INTO hubs(id, zone, info, updated_at) VALUES(?, ?, ?, ?)')
      .bind(1, id)
      .bind(2, zone)
      .bind(3, JSON.stringify(info))
      .bind(4, t)
      .exec()
  }
}

function recordToMesh(rec) {
  return {
    name: rec.name,
    ca: rec.ca,
    agent: JSON.parse(rec.agent),
    bootstraps: rec.bootstraps.split(','),
  }
}

function allMeshes() {
  return (
    db.sql('SELECT * FROM meshes')
      .exec()
      .map(recordToMesh)
  )
}

function getMesh(name) {
  return (
    db.sql('SELECT * FROM meshes WHERE name = ?')
      .bind(1, name)
      .exec()
      .slice(0, 1)
      .map(recordToMesh)[0]
  )
}

function setMesh(name, mesh) {
  var old = getMesh(name)
  if (old) {
    mesh = { ...old, ...mesh }
    var agent = { ...(old.agent || {}), ...(mesh.agent || {}) }
    agent.id = old.agent?.id || algo.uuid()
    db.sql('UPDATE meshes SET ca = ?, agent = ?, bootstraps = ? WHERE name = ?')
      .bind(1, mesh.ca || '')
      .bind(2, JSON.stringify(agent))
      .bind(3, (mesh.bootstraps || []).join(','))
      .bind(4, name)
      .exec()
  } else {
    var agent = mesh.agent || {}
    agent.id = algo.uuid()
    db.sql('INSERT INTO meshes(name, ca, agent, bootstraps) VALUES(?, ?, ?, ?)')
      .bind(1, name)
      .bind(2, mesh.ca || '')
      .bind(3, JSON.stringify(agent))
      .bind(4, (mesh.bootstraps || []).join(','))
      .exec()
  }
}

function delMesh(name) {
  db.sql('DELETE FROM files WHERE mesh = ?')
    .bind(1, name)
    .exec()
  db.sql('DELETE FROM apps WHERE mesh = ?')
    .bind(1, name)
    .exec()
  db.sql('DELETE FROM meshes WHERE name = ?')
    .bind(1, name)
    .exec()
  var remaining = db.sql('SELECT COUNT(*) AS n FROM meshes').exec()[0]?.n || 0
  if (remaining === 0) {
    db.sql('DELETE FROM hubs').exec()
  }
}

function recordToApp(rec) {
  return {
    provider: rec.provider,
    name: rec.name,
    tag: rec.tag,
    username: rec.username,
    state: rec.state,
  }
}

function allApps(mesh) {
  return (
    db.sql('SELECT * FROM apps WHERE mesh = ?')
      .bind(1, mesh)
      .exec()
      .map(recordToApp)
  )
}

function getApp(mesh, provider, name, tag) {
  return (
    db.sql('SELECT * FROM apps WHERE mesh = ? AND provider = ? AND name = ? AND tag = ?')
      .bind(1, mesh)
      .bind(2, provider)
      .bind(3, name)
      .bind(4, tag)
      .exec()
      .slice(0, 1)
      .map(recordToApp)[0]
  )
}

function setApp(mesh, provider, name, tag, app) {
  var old = getApp(mesh, provider, name, tag)
  if (old) {
    db.sql('UPDATE apps SET username = ?, state = ? WHERE mesh = ? AND provider = ? AND name = ? AND tag = ?')
      .bind(1, 'username' in app ? app.username : old.username)
      .bind(2, 'state' in app ? app.state : old.state)
      .bind(3, mesh)
      .bind(4, provider)
      .bind(5, name)
      .bind(6, tag)
      .exec()
  } else {
    db.sql('INSERT INTO apps(mesh, provider, name, tag, username, state) VALUES(?, ?, ?, ?, ?, ?)')
      .bind(1, mesh)
      .bind(2, provider)
      .bind(3, name)
      .bind(4, tag)
      .bind(5, app.username || '')
      .bind(6, app.state || '')
      .exec()
  }
}

function delApp(mesh, provider, name, tag) {
  db.sql('DELETE FROM apps WHERE mesh = ? AND provider = ? AND name = ? AND tag = ?')
    .bind(1, mesh)
    .bind(2, provider)
    .bind(3, name)
    .bind(4, tag)
    .exec()
}

function allFiles(mesh, provider, app, path) {
  return (
    db.sql('SELECT path FROM files WHERE mesh = ? AND provider = ? AND app = ?')
      .bind(1, mesh)
      .bind(2, provider)
      .bind(3, app)
      .exec()
      .map(rec => rec.path)
  )
}

function getFile(mesh, provider, app, path) {
  var sql = db.sql('SELECT data FROM files WHERE mesh = ? AND provider = ? AND app = ? AND path = ?')
    .bind(1, mesh)
    .bind(2, provider)
    .bind(3, app)
    .bind(4, path)
  if (sql.step()) return null
  return sql.column(0)
}

function setFile(mesh, provider, app, path, data) {
  if (getFile(mesh, provider, app, path)) {
    db.sql('UPDATE files SET data = ? WHERE mesh = ? AND provider = ? AND app = ? AND path = ?')
      .bind(1, data)
      .bind(2, mesh)
      .bind(3, provider)
      .bind(4, app)
      .bind(5, path)
      .exec()
  } else {
    db.sql('INSERT INTO files(mesh, provider, app, path, data) VALUES(?, ?, ?, ?, ?)')
      .bind(1, mesh)
      .bind(2, provider)
      .bind(3, app)
      .bind(4, path)
      .bind(5, data)
      .exec()
  }
}

function delFile(mesh, provider, app, path) {
  db.sql('DELETE FROM files WHERE mesh = ? AND provider = ? AND app = ? AND path = ?')
    .bind(1, mesh)
    .bind(2, provider)
    .bind(3, app)
    .bind(4, path)
    .exec()
}

function logApi(clientIp, api, reqHeaders, reqBody, resHeaders, resBody) {
  db.sql(`INSERT INTO api_log(timestamp, client_ip, api, req_headers, req_body, res_headers, res_body) VALUES(?, ?, ?, ?, ?, ?, ?)`)
    .bind(1, Date.now())
    .bind(2, clientIp)
    .bind(3, api)
    .bind(4, JSON.stringify(reqHeaders || {}))
    .bind(5, reqBody || '')
    .bind(6, JSON.stringify(resHeaders || {}))
    .bind(7, resBody || '')
    .exec()
}

function allOpenclaws() {
  return db.sql('SELECT agent_name, template_name, type, api_url, token, soul_content FROM openclaws').exec()
}

function setOpenclaw(agentName, templateName, openclaw) {
  var type = openclaw?.type || 'openclaw'
  var apiURL = openclaw?.api_url || ''
  var token = openclaw?.token || 'join-party'
  var soulContent = openclaw?.soul_content || ''

  db.sql('INSERT OR REPLACE INTO openclaws(agent_name, template_name, type, api_url, token, soul_content) VALUES(?, ?, ?, ?, ?, ?)')
    .bind(1, agentName)
    .bind(2, templateName)
    .bind(3, type)
    .bind(4, apiURL)
    .bind(5, token)
    .bind(6, soulContent)
    .exec()
}

function getKey(name) {
  return db.sql(`SELECT data FROM keys WHERE name = ?`)
    .bind(1, name)
    .exec()[0]?.data
}

function setKey(name, data) {
  if (getKey(name)) {
    db.sql(`UPDATE keys SET data = ? WHERE name = ?`)
      .bind(1, data)
      .bind(2, name)
      .exec()
  } else {
    db.sql(`INSERT INTO keys(name, data) VALUES(?, ?)`)
      .bind(1, name)
      .bind(2, data)
      .exec()
  }
}

function delKey(name) {
  db.sql(`DELETE FROM keys WHERE name = ?`)
    .bind(1, name)
    .exec()
}

function getChatPeer(mesh, peer) {
  var row = db.sql('SELECT * FROM chat_peer WHERE mesh = ? AND peer = ?')
    .bind(1, mesh)
    .bind(2, peer)
    .exec()[0]
  if (!row) return null
  return {
    mesh: row.mesh,
    peer: row.peer,
    autoReply: row.auto_reply === 1,
    autoReplyAgent: row.auto_reply_agent,
    peerAgentName: row.peer_agent_name || '',
    credit: row.credit !== undefined ? row.credit : 100,
    filterChain: row.filter_chain || '',
    sendFilterChain: row.send_filter_chain || '',
    isBlocked: row.is_blocked === 1,
    run: row.run !== undefined ? row.run : 1,
    muted: row.muted === 1,
    thinkingTime: row.thinking_time !== undefined ? row.thinking_time : 3,
    peerProfile: row.peer_profile || '',
    shortContext: row.short_context || '',
    longContext: row.long_context || '',
    peerName: row.peer_name || '',
    halfAutomation: row.half_automation === 1,
  }
}

function setChatPeer(mesh, peer, config) {
  var old = getChatPeer(mesh, peer)
  if (old) {
    var autoReply = 'autoReply' in config ? (config.autoReply ? 1 : 0) : (old.autoReply ? 1 : 0)
    var autoReplyAgent = config.autoReplyAgent || old.autoReplyAgent
    var peerAgentName = 'peerAgentName' in config ? (config.peerAgentName || '') : (old.peerAgentName || '')
    var credit = 'credit' in config ? config.credit : old.credit
    var filterChain = 'filterChain' in config ? (config.filterChain || '') : (old.filterChain || '')
    var sendFilterChain = 'sendFilterChain' in config ? (config.sendFilterChain || '') : (old.sendFilterChain || '')
    var isBlocked = 'isBlocked' in config ? (config.isBlocked ? 1 : 0) : (old.isBlocked ? 1 : 0)
    var run = 'run' in config ? config.run : old.run
    var muted = 'muted' in config ? (config.muted ? 1 : 0) : (old.muted ? 1 : 0)
    var thinkingTime = 'thinkingTime' in config ? config.thinkingTime : old.thinkingTime
    var peerProfile = 'peerProfile' in config ? (config.peerProfile || '') : (old.peerProfile || '')
    var shortContext = 'shortContext' in config ? (config.shortContext || '') : (old.shortContext || '')
    var longContext = 'longContext' in config ? (config.longContext || '') : (old.longContext || '')
    var peerName = 'peerName' in config ? (config.peerName || '') : (old.peerName || '')
    var halfAutomation = 'halfAutomation' in config ? (config.halfAutomation ? 1 : 0) : (old.halfAutomation ? 1 : 0)
    db.sql('UPDATE chat_peer SET auto_reply = ?, auto_reply_agent = ?, peer_agent_name = ?, credit = ?, filter_chain = ?, send_filter_chain = ?, is_blocked = ?, run = ?, muted = ?, thinking_time = ?, peer_profile = ?, short_context = ?, long_context = ?, peer_name = ?, half_automation = ? WHERE mesh = ? AND peer = ?')
      .bind(1, autoReply)
      .bind(2, autoReplyAgent)
      .bind(3, peerAgentName)
      .bind(4, credit)
      .bind(5, filterChain)
      .bind(6, sendFilterChain)
      .bind(7, isBlocked)
      .bind(8, run)
      .bind(9, muted)
      .bind(10, thinkingTime)
      .bind(11, peerProfile)
      .bind(12, shortContext)
      .bind(13, longContext)
      .bind(14, peerName)
      .bind(15, halfAutomation)
      .bind(16, mesh)
      .bind(17, peer)
      .exec()
  } else {
    var autoReply = config.autoReply ? 1 : 0
    var autoReplyAgent = config.autoReplyAgent || 'main'
    var peerAgentName = config.peerAgentName || ''
    var credit = 'credit' in config ? config.credit : 100
    var filterChain = config.filterChain || ''
    var sendFilterChain = config.sendFilterChain || ''
    var isBlocked = config.isBlocked ? 1 : 0
    var run = 'run' in config ? config.run : 1
    var muted = config.muted ? 1 : 0
    var thinkingTime = 'thinkingTime' in config ? config.thinkingTime : 3
    var peerProfile = config.peerProfile || ''
    var shortContext = config.shortContext || ''
    var longContext = config.longContext || ''
    var peerName = config.peerName || ''
    var halfAutomation = config.halfAutomation ? 1 : 0
    db.sql('INSERT INTO chat_peer(mesh, peer, auto_reply, auto_reply_agent, peer_agent_name, credit, filter_chain, send_filter_chain, is_blocked, run, muted, thinking_time, peer_profile, short_context, long_context, peer_name, half_automation) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)')
      .bind(1, mesh)
      .bind(2, peer)
      .bind(3, autoReply)
      .bind(4, autoReplyAgent)
      .bind(5, peerAgentName)
      .bind(6, credit)
      .bind(7, filterChain)
      .bind(8, sendFilterChain)
      .bind(9, isBlocked)
      .bind(10, run)
      .bind(11, muted)
      .bind(12, thinkingTime)
      .bind(13, peerProfile)
      .bind(14, shortContext)
      .bind(15, longContext)
      .bind(16, peerName)
      .bind(17, halfAutomation)
      .exec()
  }
}

function allChatPeers(mesh) {
  return db.sql('SELECT * FROM chat_peer WHERE mesh = ?')
    .bind(1, mesh)
    .exec()
    .map(row => ({
      mesh: row.mesh,
      peer: row.peer,
      autoReply: row.auto_reply === 1,
      autoReplyAgent: row.auto_reply_agent,
      peerAgentName: row.peer_agent_name || '',
      credit: row.credit !== undefined ? row.credit : 100,
      filterChain: row.filter_chain || '',
      sendFilterChain: row.send_filter_chain || '',
      isBlocked: row.is_blocked === 1,
      run: row.run !== undefined ? row.run : 1,
      muted: row.muted === 1,
      thinkingTime: row.thinking_time !== undefined ? row.thinking_time : 3,
      peerProfile: row.peer_profile || '',
      shortContext: row.short_context || '',
      longContext: row.long_context || '',
      peerName: row.peer_name || '',
      halfAutomation: row.half_automation === 1,
    }))
}

function adjustCredit(mesh, peer, delta) {
  var old = getChatPeer(mesh, peer)
  if (!old) return
  var newCredit = old.credit + delta
  db.sql('UPDATE chat_peer SET credit = ? WHERE mesh = ? AND peer = ?')
    .bind(1, newCredit)
    .bind(2, mesh)
    .bind(3, peer)
    .exec()
}

function getBlockedKeywords(mesh) {
  return db.sql('SELECT keyword FROM blocked_keywords WHERE mesh = ?')
    .bind(1, mesh)
    .exec()
    .map(row => row.keyword)
}

function addBlockedKeyword(mesh, keyword) {
  try {
    db.sql('INSERT INTO blocked_keywords(mesh, keyword) VALUES(?, ?)')
      .bind(1, mesh)
      .bind(2, keyword)
      .exec()
    return true
  } catch {
    return false
  }
}

function delBlockedKeyword(mesh, keyword) {
  db.sql('DELETE FROM blocked_keywords WHERE mesh = ? AND keyword = ?')
    .bind(1, mesh)
    .bind(2, keyword)
    .exec()
}

function countRecentMessages(mesh, chatId, sender, content, withinSeconds) {
  var since = Date.now() / 1000 - withinSeconds
  var rows = db.sql(`
    SELECT COUNT(*) as cnt FROM chat_log
    WHERE mesh = ? AND chat_type = 'peer' AND chat_id = ? AND sender = ? AND content = ? AND time >= ?
  `)
    .bind(1, mesh)
    .bind(2, chatId)
    .bind(3, sender)
    .bind(4, content)
    .bind(5, since)
    .exec()
  return rows[0] ? rows[0].cnt : 0
}

function getCache(key) {
  var rows = db.sql('SELECT value FROM cache WHERE key = ?')
    .bind(1, key)
    .exec()
  if (!rows[0]) {
    return null
  }
  try {
    return JSON.parse(rows[0].value)
  } catch {
    return rows[0].value
  }
}

function setCache(key, value) {
  var json = JSON.stringify(value)
  console.info('[db cache] set', key, '->', json.substring(0, 100))
  db.sql('INSERT INTO cache(key, value) VALUES(?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value')
    .bind(1, key)
    .bind(2, json)
    .exec()
}

// event: 'message' | 'group_create' | 'group_delete' | 'group_leave'
function hasCliLog(agentName, sessionId, messageMd5) {
  var since = Date.now() / 1000 - 4 * 60 * 60
  var rows = db.sql(
    'SELECT id FROM cli_log WHERE agent_name = ? AND session_id = ? AND message_md5 = ? AND timestamp >= ?'
  )
    .bind(1, agentName)
    .bind(2, sessionId)
    .bind(3, messageMd5)
    .bind(4, since)
    .exec()
  return rows.length > 0
}

function addCliLog(agentName, sessionId, messageMd5) {
  db.sql('INSERT INTO cli_log(timestamp, agent_name, session_id, message_md5) VALUES(?, ?, ?, ?)')
    .bind(1, Date.now() / 1000)
    .bind(2, agentName)
    .bind(3, sessionId)
    .bind(4, messageMd5)
    .exec()
}

function logCliCall(command, args, output, exitCode, success, durationMs, errorMessage, agentName, sessionId, messageMd5) {
  db.sql(`INSERT INTO cli_log(timestamp, agent_name, session_id, message_md5, command, arguments, output, exit_code, success, duration_ms, error_message)
          VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
    .bind(1, Date.now() / 1000)
    .bind(2, agentName || '')
    .bind(3, sessionId || '')
    .bind(4, messageMd5 || '')
    .bind(5, command || '')
    .bind(6, args !== null && typeof args === 'object' ? JSON.stringify(args) : (args || ''))
    .bind(7, output || '')
    .bind(8, exitCode || 0)
    .bind(9, success ? 1 : 0)
    .bind(10, durationMs || 0)
    .bind(11, errorMessage || '')
    .exec()
}

// chat_type: 'peer' | 'group' | 'openclaw'
// msg_type: 'user' | 'response' | 'system' | 'tool' (default: 'response')
function logChat(mesh, chatType, chatId, chatName, creator, sender, event, content, members, sessionId, muted, msgType) {
  var t = Date.now() / 1000
  db.sql(`
    INSERT INTO chat_log(time, mesh, chat_type, chat_id, chat_name, creator, sender, event, content, members, session_id, muted, msg_type)
    VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `)
    .bind(1, t)
    .bind(2, mesh || '')
    .bind(3, chatType)
    .bind(4, chatId)
    .bind(5, chatName || null)
    .bind(6, creator || null)
    .bind(7, sender)
    .bind(8, event)
    .bind(9, content || null)
    .bind(10, members ? JSON.stringify(members) : null)
    .bind(11, sessionId || null)
    .bind(12, muted ? 1 : 0)
    .bind(13, msgType || 'response')
    .exec()
}

// msgTypes: array of msg_type values to include (default: ['user', 'response'])
function getChatLog(mesh, chatType, chatId, limit, msgTypes) {
  var rows
  var types = msgTypes || ['user', 'response']
  if (chatId) {
    rows = db.sql(`
      SELECT * FROM chat_log
      WHERE mesh = ? AND chat_type = ? AND chat_id = ?
      ORDER BY time DESC LIMIT ?
    `)
      .bind(1, mesh || '')
      .bind(2, chatType)
      .bind(3, chatId)
      .bind(4, limit || 200)
      .exec()
  } else {
    rows = db.sql(`
      SELECT * FROM chat_log
      WHERE mesh = ?
      ORDER BY time DESC LIMIT ?
    `)
      .bind(1, mesh || '')
      .bind(2, limit || 200)
      .exec()
  }
  // Filter by msg_type in JavaScript
  return rows.filter(r => types.includes(r.msg_type || 'response')).map(r => ({
    id: r.id,
    time: r.time,
    mesh: r.mesh,
    chatType: r.chat_type,
    chatId: r.chat_id,
    chatName: r.chat_name || null,
    creator: r.creator || null,
    sender: r.sender,
    event: r.event,
    content: r.content || null,
    members: r.members ? JSON.parse(r.members) : null,
    sessionId: r.session_id || null,
    msgType: r.msg_type || 'response',
  }))
}

export default {
  open,
  allZones,
  setZones,
  allHubs,
  setHubs,
  getHub,
  setHub,
  allMeshes,
  getMesh,
  setMesh,
  delMesh,
  allApps,
  getApp,
  setApp,
  delApp,
  allFiles,
  getFile,
  setFile,
  delFile,
  allOpenclaws,
  setOpenclaw,
  getKey,
  setKey,
  delKey,
  logApi,
  logChat,
  getChatLog,
  getChatPeer,
  setChatPeer,
  allChatPeers,
  adjustCredit,
  getBlockedKeywords,
  addBlockedKeyword,
  delBlockedKeyword,
  countRecentMessages,
  getCache,
  setCache,
  hasCliLog,
  addCliLog,
  logCliCall,
}
