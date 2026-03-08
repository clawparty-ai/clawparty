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
    CREATE TABLE IF NOT EXISTS certificates (
      name TEXT PRIMARY KEY,
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
    CREATE TABLE IF NOT EXISTS files (
      path TEXT PRIMARY KEY,
      hash TEXT NOT NULL,
      size INTEGER NOT NULL,
      time REAL NOT NULL,
      since REAL NOT NULL
    )
  `)

  db.exec(`
    CREATE TABLE IF NOT EXISTS acl (
      path TEXT PRIMARY KEY,
      access TEXT NOT NULL,
      since REAL NOT NULL
    )
  `)

  db.exec(`
    CREATE TABLE IF NOT EXISTS evictions (
      username TEXT PRIMARY KEY,
      evicted_at REAL NOT NULL,
      expires_at REAL NOT NULL
    )
  `)

  db.exec(`
    CREATE TABLE IF NOT EXISTS users (
      username   TEXT PRIMARY KEY,
      ep_name    TEXT NOT NULL,
      pass_key   TEXT,
      status     TEXT NOT NULL DEFAULT '注册中',
      created_at REAL NOT NULL,
      updated_at REAL NOT NULL
    )
  `)

  // Migration: add pass_key column for existing databases
  try {
    db.exec(`ALTER TABLE users ADD COLUMN pass_key TEXT`)
  } catch {}

  db.exec(`
    CREATE TABLE IF NOT EXISTS user_log (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      time REAL NOT NULL,
      username TEXT NOT NULL,
      action TEXT NOT NULL,
      operator TEXT,
      detail TEXT
    )
  `)

  try {
    db.exec(`CREATE INDEX IF NOT EXISTS user_log_username ON user_log(username)`)
    db.exec(`CREATE INDEX IF NOT EXISTS user_log_time ON user_log(time)`)
  } catch {}

  db.exec(`
    CREATE TABLE IF NOT EXISTS api_log (
      id        INTEGER PRIMARY KEY AUTOINCREMENT,
      time      REAL NOT NULL,
      method    TEXT NOT NULL,
      path      TEXT NOT NULL,
      client_ip TEXT,
      status    INTEGER NOT NULL,
      username  TEXT,
      detail    TEXT
    )
  `)

  try {
    db.exec(`CREATE INDEX IF NOT EXISTS api_log_time ON api_log(time)`)
    db.exec(`CREATE INDEX IF NOT EXISTS api_log_username ON api_log(username)`)
  } catch {}
}

function allHubs() {
  var t = Date.now() - 30 * 24 * 60 * 60 * 1000
  var all = {}
  db.sql('SELECT id, info FROM hubs WHERE updated_at >= ?')
    .bind(1, t)
    .exec()
    .forEach(r => {
      try {
        all[r.id] = {
          zone: r.zone,
          ...JSON.parse(r.info),
        }
      } catch {}
    })
  return all
}

function setHubs(hubs) {
  var t = Date.now()
  var old = {}
  db.sql('SELECT id FROM hubs')
    .exec()
    .forEach(r => old[r.id] = true)
  Object.entries(hubs).forEach(
    ([id, hub]) => {
      var info = JSON.stringify({ ports: hub.ports, version: hub.version })
      if (id in old) {
        db.sql('UPDATE hubs SET zone = ?, info = ?, updated_at = ? WHERE id = ?')
          .bind(1, hub.zone)
          .bind(2, info)
          .bind(3, t)
          .bind(4, id)
          .exec()
      } else {
        db.sql('INSERT INTO hubs(id, zone, info, updated_at) VALUES(?, ?, ?, ?)')
          .bind(1, id)
          .bind(2, hub.zone)
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

function getCert(name) {
  return db.sql(`SELECT data FROM certificates WHERE name = ?`)
    .bind(1, name)
    .exec()[0]?.data
}

function setCert(name, data) {
  if (getCert(name)) {
    db.sql(`UPDATE certificates SET data = ? WHERE name = ?`)
      .bind(1, data)
      .bind(2, name)
      .exec()
  } else {
    db.sql(`INSERT INTO certificates(name, data) VALUES(?, ?)`)
      .bind(1, name)
      .bind(2, data)
      .exec()
  }
}

function delCert(name) {
  db.sql(`DELETE FROM certificates WHERE name = ?`)
    .bind(1, name)
    .exec()
}

function allKeys() {
  return db.sql(`SELECT name, data FROM keys`).exec()
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

function recordToFile(rec) {
  return {
    pathname: rec.path,
    hash: rec.hash,
    size: +rec.size,
    time: rec.time,
    since: rec.since,
  }
}

function allFiles() {
  return (
    db.sql('SELECT * FROM files')
      .exec()
      .map(recordToFile)
  )
}

function getFile(pathname) {
  return (
    db.sql('SELECT * FROM files WHERE path = ?')
      .bind(1, pathname)
      .exec()
      .map(recordToFile)[0]
  )
}

function setFile(pathname, file) {
  var obj = getFile(pathname)
  if (obj) {
    Object.assign(obj, file)
    db.sql('UPDATE files SET hash = ?, size = ?, time = ?, since = ? WHERE path = ?')
      .bind(1, obj.hash)
      .bind(2, obj.size)
      .bind(3, obj.time)
      .bind(4, obj.since)
      .bind(5, pathname)
      .exec()
  } else {
    db.sql('INSERT INTO files(path, hash, size, time, since) VALUES(?, ?, ?, ?, ?)')
      .bind(1, pathname)
      .bind(2, file.hash)
      .bind(3, file.size)
      .bind(4, file.time)
      .bind(5, file.since)
      .exec()
  }
}

function recordToACL(rec) {
  try {
    var access = JSON.parse(rec.access)
  } catch {
    var access = {}
  }
  return {
    pathname: rec.path,
    access,
    since: rec.since,
  }
}

function allACL() {
  return (
    db.sql('SELECT * FROM acl')
      .exec()
      .map(recordToACL)
  )
}

function getACL(pathname) {
  return (
    db.sql('SELECT * FROM acl WHERE path = ?')
      .bind(1, pathname)
      .exec()
      .map(recordToACL)[0]
  )
}

function setACL(pathname, access, since) {
  var obj = getACL(pathname)
  if (obj) {
    db.sql('UPDATE acl SET access = ?, since = ? WHERE path = ?')
      .bind(1, JSON.stringify(access))
      .bind(2, since)
      .bind(3, pathname)
      .exec()
  } else {
    db.sql('INSERT INTO acl(path, access, since) VALUES(?, ?, ?)')
      .bind(1, pathname)
      .bind(2, JSON.stringify(access))
      .bind(3, since)
      .exec()
  }
}

function recordToEviction(rec) {
  return {
    username: rec.username,
    time: rec.evicted_at,
    expiration: rec.expires_at,
  }
}

function allEvictions() {
  return (
    db.sql('SELECT * FROM evictions')
      .exec()
      .map(recordToEviction)
  )
}

function getEviction(username) {
  return (
    db.sql('SELECT * FROM evictions WHERE username = ?')
      .bind(1, username)
      .exec()
      .map(recordToEviction)[0]
  )
}

function setEviction(username, time, expiration) {
  var obj = getEviction(username)
  if (obj) {
    db.sql('UPDATE evictions SET evicted_at = ?, expires_at = ? WHERE username = ?')
      .bind(1, time)
      .bind(2, expiration)
      .bind(3, username)
      .exec()
  } else {
    db.sql('INSERT INTO evictions(username, evicted_at, expires_at) VALUES(?, ?, ?)')
      .bind(1, username)
      .bind(2, time)
      .bind(3, expiration)
      .exec()
  }
}

function delEviction(username) {
  db.sql(`DELETE FROM evictions WHERE username = ?`)
    .bind(1, username)
    .exec()
}

function recordToUser(rec) {
  var epNames
  try { epNames = JSON.parse(rec.ep_name) } catch {}
  if (!(epNames instanceof Array)) epNames = rec.ep_name ? [rec.ep_name] : []
  return {
    username: rec.username,
    epNames,
    passKey: rec.pass_key || null,
    status: rec.status,
    createdAt: rec.created_at,
    updatedAt: rec.updated_at,
  }
}

function getUser(username) {
  return db.sql('SELECT * FROM users WHERE username = ?')
    .bind(1, username)
    .exec()
    .map(recordToUser)[0]
}

function allUsers() {
  return db.sql('SELECT * FROM users ORDER BY created_at DESC')
    .exec()
    .map(recordToUser)
}

// Insert a new user; returns false if username already exists
function createUser(username, epName, passKey) {
  if (getUser(username)) return false
  var t = Date.now() / 1000
  db.sql('INSERT INTO users(username, ep_name, pass_key, status, created_at, updated_at) VALUES(?, ?, ?, ?, ?, ?)')
    .bind(1, username)
    .bind(2, JSON.stringify([epName]))
    .bind(3, passKey || null)
    .bind(4, '注册中')
    .bind(5, t)
    .bind(6, t)
    .exec()
  return true
}

// Append a new ep_name to an existing user's ep_name list
function addUserEpName(username, epName) {
  var user = getUser(username)
  if (!user) return false
  var epNames = user.epNames
  if (epNames.indexOf(epName) < 0) epNames.push(epName)
  var t = Date.now() / 1000
  db.sql('UPDATE users SET ep_name = ?, updated_at = ? WHERE username = ?')
    .bind(1, JSON.stringify(epNames))
    .bind(2, t)
    .bind(3, username)
    .exec()
  return true
}

function setUserStatus(username, status) {
  var t = Date.now() / 1000
  db.sql('UPDATE users SET status = ?, updated_at = ? WHERE username = ?')
    .bind(1, status)
    .bind(2, t)
    .bind(3, username)
    .exec()
}

function addApiLog(method, path, clientIp, status, username, detail) {
  var t = Date.now() / 1000
  db.sql('INSERT INTO api_log(time, method, path, client_ip, status, username, detail) VALUES(?, ?, ?, ?, ?, ?, ?)')
    .bind(1, t)
    .bind(2, method)
    .bind(3, path)
    .bind(4, clientIp || null)
    .bind(5, status)
    .bind(6, username || null)
    .bind(7, detail ? JSON.stringify(detail) : null)
    .exec()
}

function getApiLog(username, limit) {
  var rows
  if (username) {
    rows = db.sql('SELECT * FROM api_log WHERE username = ? ORDER BY time DESC LIMIT ?')
      .bind(1, username)
      .bind(2, limit || 100)
      .exec()
  } else {
    rows = db.sql('SELECT * FROM api_log ORDER BY time DESC LIMIT ?')
      .bind(1, limit || 100)
      .exec()
  }
  return rows.map(r => ({
    id: r.id,
    time: r.time,
    method: r.method,
    path: r.path,
    clientIp: r.client_ip || null,
    status: r.status,
    username: r.username || null,
    detail: r.detail ? JSON.parse(r.detail) : null,
  }))
}

// action: 'cert_issued' | 'connect' | 'disconnect' | 'evict' | 'evict_removed'
// operator: who triggered the action (null means the user themselves)
// detail: optional JSON string with extra context
function addUserLog(username, action, operator, detail) {
  var t = Date.now() / 1000
  db.sql('INSERT INTO user_log(time, username, action, operator, detail) VALUES(?, ?, ?, ?, ?)')
    .bind(1, t)
    .bind(2, username)
    .bind(3, action)
    .bind(4, operator || null)
    .bind(5, detail ? JSON.stringify(detail) : null)
    .exec()
}

function getUserLog(username, limit) {
  var rows
  if (username) {
    rows = db.sql('SELECT * FROM user_log WHERE username = ? ORDER BY time DESC LIMIT ?')
      .bind(1, username)
      .bind(2, limit || 100)
      .exec()
  } else {
    rows = db.sql('SELECT * FROM user_log ORDER BY time DESC LIMIT ?')
      .bind(1, limit || 100)
      .exec()
  }
  return rows.map(r => ({
    id: r.id,
    time: r.time,
    username: r.username,
    action: r.action,
    operator: r.operator || null,
    detail: r.detail ? JSON.parse(r.detail) : null,
  }))
}

export default {
  open,
  allHubs,
  setHubs,
  getHub,
  setHub,
  getCert,
  setCert,
  delCert,
  allKeys,
  getKey,
  setKey,
  delKey,
  allFiles,
  getFile,
  setFile,
  allACL,
  getACL,
  setACL,
  allEvictions,
  getEviction,
  setEviction,
  delEviction,
  addUserLog,
  getUserLog,
  addApiLog,
  getApiLog,
  getUser,
  allUsers,
  createUser,
  addUserEpName,
  setUserStatus,
}
