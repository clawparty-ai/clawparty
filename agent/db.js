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
      name    TEXT PRIMARY KEY,
      type    TEXT NOT NULL DEFAULT 'openclaw',
      api_url TEXT NOT NULL,
      token   TEXT NOT NULL DEFAULT 'join-party'
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
      members    TEXT
    )
  `)

  try {
    db.exec(`CREATE INDEX IF NOT EXISTS chat_log_mesh_chat ON chat_log(mesh, chat_type, chat_id)`)
    db.exec(`CREATE INDEX IF NOT EXISTS chat_log_time ON chat_log(time)`)
    db.exec(`CREATE INDEX IF NOT EXISTS chat_log_sender ON chat_log(sender)`)
  } catch {}
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
    var agent = { ...old.agent, ...mesh.agent }
    agent.id = old.agent.id
    db.sql('UPDATE meshes SET ca = ?, agent = ?, bootstraps = ? WHERE name = ?')
      .bind(1, mesh.ca || '')
      .bind(2, JSON.stringify(agent))
      .bind(3, mesh.bootstraps.join(','))
      .bind(4, name)
      .exec()
  } else {
    var agent = mesh.agent
    agent.id = algo.uuid()
    db.sql('INSERT INTO meshes(name, ca, agent, bootstraps) VALUES(?, ?, ?, ?)')
      .bind(1, name)
      .bind(2, mesh.ca || '')
      .bind(3, JSON.stringify(agent))
      .bind(4, mesh.bootstraps.join(','))
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
  return db.sql('SELECT name, type, api_url, token FROM openclaws').exec()
}

function setOpenclaw(name, openclaw) {
  var type = openclaw?.type || 'openclaw'
  var apiURL = openclaw?.api_url || ''
  var token = openclaw?.token || 'join-party'

  db.sql('INSERT OR REPLACE INTO openclaws(name, type, api_url, token) VALUES(?, ?, ?, ?)')
    .bind(1, name)
    .bind(2, type)
    .bind(3, apiURL)
    .bind(4, token)
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

// event: 'message' | 'group_create' | 'group_delete' | 'group_leave'
// chat_type: 'peer' | 'group'
function logChat(mesh, chatType, chatId, chatName, creator, sender, event, content, members) {
  var t = Date.now() / 1000
  db.sql(`
    INSERT INTO chat_log(time, mesh, chat_type, chat_id, chat_name, creator, sender, event, content, members)
    VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .exec()
}

function getChatLog(mesh, chatType, chatId, limit) {
  var rows
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
  return rows.map(r => ({
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
}
