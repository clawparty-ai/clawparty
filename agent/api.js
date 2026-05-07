import db from './db.js'
import Mesh from './mesh.js'
import templates from './templates.js'
import config from './config.js'

var rootDir = ''
var agentListen = ''
var proxyAddress = ''
var pqcSettings = null
var p2pConfig = null
var meshes = {}

// Agent status cache: microsecond-level process checks instead of slow lsof
var AGENT_STATUS_CACHE_TTL = 1000  // 1 second
var _agentStatusCache = { ts: 0, data: {} }

function findMesh(name) {
  var m = meshes[name]
  if (m) return m
  throw `Mesh not found: ${name}`
}

function init(dirname, listen, proxy, pqc, p2pCfg) {
  rootDir = os.path.resolve(dirname)
  agentListen = listen
  proxyAddress = proxy
  pqcSettings = pqc
  p2pConfig = p2pCfg || {}
  templates.init(rootDir, db)
  config.init(rootDir)  // Initialize config module
  var allMeshesData = db.allMeshes()
  for (var i = 0; i < allMeshesData.length; i++) {
    var mesh = allMeshesData[i]
    var name = mesh.name
    meshes[name] = Mesh(
      os.path.join(rootDir, 'meshes', name),
      agentListen,
      proxyAddress,
      pqcSettings,
      p2pConfig,
      mesh,
      function (newMesh) {
        db.setMesh(name, newMesh)
      }
    )
    if (!mesh.agent?.offline) {
      meshes[name].start()
    }
  }
}

function setIdentity(pem) {
  var key = new crypto.PrivateKey(pem)
  db.setKey('agent', key.toPEM().toString())
}

function getIdentity() {
  var keyData = db.getKey('agent')
  var key = keyData ? new crypto.PrivateKey(keyData) : new crypto.PrivateKey({ type: pqcSettings?.signature || 'rsa', bits: 2048 })
  if (!keyData) db.setKey('agent', key.toPEM().toString())
  return new crypto.PublicKey(key).toPEM().toString()
}

function allMeshes() {
  var meshValues = Object.values(meshes)
  var result = []
  for (var i = 0; i < meshValues.length; i++) {
    result.push(meshValues[i].getStatus())
  }
  return result
}

function getMesh(name) {
  var mesh = meshes[name]
  if (mesh) return mesh.getStatus()
  return null
}

function getMeshLog(name) {
  var mesh = meshes[name]
  return mesh ? mesh.getLog() : null
}

function setMesh(name, mesh) {
  db.setMesh(name, mesh)
  var old = meshes[name]
  if (old) {
    old.leave()
    delete meshes[name]
  }
  mesh = db.getMesh(name)
  mesh.agent ??= {}
  mesh.agent.listen = agentListen
  meshes[name] = Mesh(
    os.path.join(rootDir, 'meshes', mesh.name),
    agentListen,
    proxyAddress,
    pqcSettings,
    p2pConfig,
    mesh,
    function (newMesh) {
      db.setMesh(name, newMesh)
    }
  )
  if (!mesh.agent.offline) {
    meshes[name].start()
  }
  return getMesh(name)
}

function delMesh(name) {
  db.delMesh(name)
  var old = meshes[name]
  if (old) {
    old.leave()
    delete meshes[name]
  }
}

function getPermit(mesh, username, identity) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  return m.issuePermit(username, identity)
}

function allHubs(mesh) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve([])
  return m.discoverHubs()
}

function setHub(mesh, id, hub) {
  var m = meshes[mesh]
  if (m && hub.connected === true) {
    return m.attachHub(id)
  } else {
    return Promise.resolve()
  }
}

function getHub(mesh, id) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  return m.findHub(id)
}

function getHubLog(mesh, id) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve([])
  return m.findHubLog(id)
}

function createInviteCode(mesh, hubId, data) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  return m.createInviteCode(hubId, data)
}

function allEndpoints(mesh, id, name, user, keyword, offset, limit) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve([])
  var idLocal = m.config.agent.id
  try {
    return m.discoverEndpoints(id, name, user, keyword, offset, limit).then(
      function(list) {
        var mapped = []
        for (var i = 0; i < list.length; i++) {
          var ep = list[i]
          mapped.push({ isLocal: ep.id === idLocal, agent: ep.agent, id: ep.id, name: ep.name, hubs: ep.hubs, username: ep.username, ip: ep.ip, port: ep.port, heartbeat: ep.heartbeat, ping: ep.ping, online: ep.online, stats: ep.stats })
        }
        return mapped
      }
    )
  } catch (e) {
    return Promise.resolve([])
  }
}

function getEndpoint(mesh, ep) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  return m.findEndpoint(ep).then(
    function(epResult) {
      epResult.isLocal = (epResult.id === m.config.agent.id)
      return epResult
    }
  )
}

function getEndpointLabels(mesh, ep) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  if (!ep || ep === m.config.agent.id) {
    return Promise.resolve(m.getLabels())
  } else {
    return m.remoteGetLabels(ep)
  }
}

function setEndpointLabels(mesh, ep, labels) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  if (!ep || ep === m.config.agent.id) {
    return Promise.resolve(m.setLabels(labels))
  } else {
    return m.remoteSetLabels(ep, labels)
  }
}

function getEndpointLog(mesh, ep) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  if (!ep || ep === m.config.agent.id) {
    return Promise.resolve(m.getLog())
  } else {
    return m.remoteQueryLog(ep)
  }
}

function allUsers(mesh, name, keyword, offset, limit) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve([])
  try {
    return m.discoverUsers(name, keyword, offset, limit).then(
      function(results) {
        var idLocal = m.config.agent.id
        for (var i = 0; i < results.length; i++) {
          var user = results[i]
          if (user.endpoints && user.endpoints.instances) {
            var instances = user.endpoints.instances
            for (var j = 0; j < instances.length; j++) {
              var ep = instances[j]
              if (ep.id === idLocal) ep.isLocal = true
            }
          }
        }
        return results
      }
    )
  } catch (e) {
    return Promise.resolve([])
  }
}

function delUser(mesh, name) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve([])
  return m.evictUser(name)
}

function allFiles(mesh, since) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  try {
    return m.discoverFiles(since)
  } catch (e) {
    return Promise.resolve(null)
  }
}

function getFileInfo(mesh, pathname) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  return m.findFile(pathname)
}

function delFileInfo(mesh, pathname) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(false)
  return Promise.resolve(m.deleteFile(pathname))
}

function getFileData(mesh, pathname) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  return m.syncFile(pathname)
}

function setFileData(mesh, pathname, data) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(false)
  return Promise.resolve(m.publishFile(pathname, data))
}

function delFileData(mesh, pathname) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(false)
  return Promise.resolve(m.unpublishFile(pathname))
}

function getFileDataFromEP(mesh, ep, hash) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  return m.downloadFile(ep, hash)
}

function allApps(mesh, ep) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve([])
  return m.discoverApps(ep)
}

function getApp(mesh, ep, provider, app) {
  var m = findMesh(mesh)
  if (!m) return Promise.resolve(null)
  return m.findApp(ep, provider, app)
}

function setApp(mesh, ep, provider, app, state) {
  var m = findMesh(mesh)
  if (!m) return Promise.resolve(null)
  return m.findApp(ep, provider, app).then(function(ret) {
    if (ret) return
    return m.installApp(ep, provider, app)
  }).then(function() {
    if (!('isDisabled' in state)) return
    if (state.isDisabled) {
      return m.disableApp(ep, provider, app)
    } else {
      return m.enableApp(ep, provider, app)
    }
  }).then(function() {
    if (!('isRunning' in state)) return
    if (state.isRunning) {
      return m.startApp(ep, provider, app)
    } else {
      return m.stopApp(ep, provider, app)
    }
  }).then(function() {
    if (!('isPublished' in state)) return
    if (state.isPublished) {
      return m.publishApp(ep, provider, app)
    } else {
      return m.unpublishApp(ep, provider, app)
    }
  }).then(function() {
    return m.findApp(ep, provider, app)
  })
}

function delApp(mesh, ep, provider, app) {
  var m = findMesh(mesh)
  if (!m) return Promise.resolve()
  return m.uninstallApp(ep, provider, app)
}

function getAppLog(mesh, ep, provider, app) {
  var m = findMesh(mesh)
  if (!m) return Promise.resolve()
  return m.dumpAppLog(ep, provider, app)
}

function connectApp(mesh, provider, app) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  return m.connectApp(provider, app, m.username)
}

function getEndpointStats(mesh, ep) {
  var m = findMesh(mesh)
  if (!m) return null
  return m.getEndpointStats(ep)
}

function pingEndpoint(mesh, ep) {
  var m = findMesh(mesh)
  if (!m) return null
  return m.pingEndpoint(ep)
}

function getLocalTemplates() {
  return templates.scanLocalTemplates()
}

function getSharedTemplates() {
  return templates.scanSharedTemplates()
}

function installLocalTemplate(industry, agent, soulContent, agentName) {
  return templates.installTemplate(industry, agent, 'local', soulContent, agentName)
}

function installSharedTemplate(industry, agent, soulContent, agentName) {
  return templates.installTemplate(industry, agent, 'shared', soulContent, agentName)
}

// ── AI-Agent Management ────────────────────────────────────────────────

var agentProcesses = {}

function ensurePairingDisabled(configContent) {
  if (configContent.indexOf('[gateway]') < 0) {
    configContent = configContent + '\n[gateway]\nrequire_pairing = false\n'
  } else if (configContent.indexOf('require_pairing') < 0) {
    configContent = configContent.replace('[gateway]', '[gateway]\nrequire_pairing = false')
  } else {
    configContent = configContent.replaceAll('require_pairing = true', 'require_pairing = false')
  }
  return configContent
}

function allocatePort() {
  var PORT_START = 42618
  var PORT_END = 42700

  // Get used ports from database
  var allAgents = db.allAgents()
  var usedPorts = []
  for (var i = 0; i < allAgents.length; i++) {
    usedPorts.push(allAgents[i].port)
  }

  // Find available port
  for (var port = PORT_START; port <= PORT_END; port++) {
    var skip = false

    if (usedPorts.includes(port)) {
      skip = true
    }

    if (!skip && db.isPortUsed(port)) {
      skip = true
    }

    if (!skip) {
      try {
        var res = http.get('http://127.0.0.1:' + port + '/health', { timeout: 100 })
      } catch (e) {
        return port
      }
    }
  }

  throw 'No available ports in range ' + PORT_START + '-' + PORT_END
}

function makeDisplayDirName(displayName, agentsDir) {
  var baseName = displayName || 'agent'
  var dirPath = os.path.join(agentsDir, baseName)
  try {
    os.mkdir(dirPath)
    return baseName
  } catch (e) {
    // Directory already exists — could be a leftover from a previous
    // failed creation (crash before DB write) or manual cleanup. If it
    // is actually a directory, reuse it instead of throwing.
    try {
      var st = os.stat(dirPath)
      if (st && st.isDirectory()) {
        console.log('[AGENT] Reusing existing directory: ' + dirPath)
        return baseName
      }
    } catch (e2) {}
    throw 'Agent "' + baseName + '" 已经存在，请使用其他名称'
  }
}

function createAgent(agentName, displayName, modelConfig, description, workspaceFiles, templateSource) {
  console.log('[AGENT] Creating agent: ' + agentName)

  // Check if agent already exists — if so, append random 2-digit suffix
  var originalName = agentName
  if (db.getAgent(agentName)) {
    var attempt = 0
    for (attempt = 0; attempt < 100; attempt++) {
      var suffix = String(Math.floor(Math.random() * 90) + 10)
      var candidate = originalName + '-' + suffix
      if (!db.getAgent(candidate) && !db.getGroupChat(candidate)) {
        agentName = candidate
        break
      }
    }
    if (attempt >= 100) {
      console.log('[AGENT] Create failed: could not find unique name for ' + originalName)
      throw 'Could not find unique agent name for ' + originalName
    }
    console.log('[AGENT] Name ' + originalName + ' taken, using ' + agentName)
  }

  // Allocate port
  var port = allocatePort()
  console.log('[AGENT] Allocated port: ' + port)

  // Create directory structure using agent_name (English, safe for Windows FS).
  // display_name (Chinese or localized) is stored in the DB for UI only.
  var agentsDir = os.path.join(rootDir, 'agents')
  var dirName = makeDisplayDirName(agentName, agentsDir)
  var agentDir = os.path.join(agentsDir, dirName)
  var workspaceDir = os.path.join(agentDir, 'workspace')

  os.mkdir(workspaceDir, { recursive: true })
  console.log('[AGENT] Created workspace: ' + workspaceDir)

  // Copy all .md files from template directory
  if (templateSource) {
    var templateBaseDir = templateSource.shared
      ? os.path.join(os.home(), '.clawparty', '.agent-template', '.shared')
      : os.path.join(os.home(), '.clawparty', '.agent-template')
    var tplDir = os.path.join(templateBaseDir, templateSource.industry, templateSource.slug)
    
    var files = []
    try { files = os.readDir(tplDir) } catch (e) { files = [] }
    
    for (var i = 0; i < files.length; i++) {
      var f = files[i]
      if (f.endsWith('.md')) {
        var srcPath = os.path.join(tplDir, f)
        var destPath = os.path.join(workspaceDir, f)
        try {
          os.write(destPath, os.read(srcPath).toString())
          console.log('[AGENT] Copied template file: ' + f)
        } catch (e) {
          console.log('[AGENT] Warning: failed to copy ' + f + ': ' + e)
        }
      }
    }
  }

  // Read template: prefer global-config, fallback to hub-distributed
  // Removed fallback to ~/.zeroclaw/config.toml
  var globalConfigPath = os.path.join(rootDir, 'global-config.toml')
  var hubTemplatePath = os.path.join(rootDir, 'zeroclaw-template.toml')
  var templateContent

  try {
    templateContent = os.read(globalConfigPath).toString()
    console.log('[AGENT] Using global config as template: ' + globalConfigPath)
  } catch (e) {
    console.log('[AGENT] Global config not found, using hub template: ' + hubTemplatePath)
    templateContent = os.read(hubTemplatePath).toString()
  }

  // Patch config to disable pairing (all agents managed by ClawParty)
  var patchedConfig = ensurePairingDisabled(templateContent)

  var configPath = os.path.join(agentDir, 'config.toml')
  os.write(configPath, patchedConfig)
  console.log('[AGENT] Wrote config with pairing disabled: ' + configPath)

  // Build identity header from display_name / description and prepend to SOUL.md
  var identityHeader = ''
  if (displayName || description) {
    identityHeader += '# ' + (displayName || agentName) + '\n\n'
    if (description) identityHeader += description + '\n\n'
  }

  // Write AI-generated workspace files if provided
  if (workspaceFiles && workspaceFiles.soul_md) {
    os.write(os.path.join(workspaceDir, 'SOUL.md'), identityHeader + workspaceFiles.soul_md)
    console.log('[AGENT] Wrote SOUL.md')
  } else if (identityHeader) {
    // No soul_md provided but we have identity info — write it as the initial SOUL.md
    os.write(os.path.join(workspaceDir, 'SOUL.md'), identityHeader)
    console.log('[AGENT] Wrote SOUL.md with identity header')
  }
  if (workspaceFiles && workspaceFiles.agents_md) {
    os.write(os.path.join(workspaceDir, 'AGENTS.md'), workspaceFiles.agents_md)
    console.log('[AGENT] Wrote AGENTS.md')
  }

  // P2: Auto-inject Task Management into SOUL.md
  var soulPath = os.path.join(workspaceDir, 'SOUL.md')
  var soulContent = ''
  try {
    soulContent = os.read(soulPath).toString()
  } catch (e) {
    soulContent = ''
  }
  if (soulContent.indexOf('## 任务管理 (Task Management)') < 0) {
    var taskOverlay = '\n\n## 任务管理 (Task Management)\n\n当你处理用户的请求时，你必须维护一个结构化的任务日志，帮助用户追踪你的工作进度。\n\n### 规则：\n\n1. **当用户要求你做一件大事时**（如写代码、调研、规划等），创建一个顶层任务：\n   ```\n   <task id="task-{timestamp}-{shortid}" title="简短的任务标题" status="running" progress="0">\n   任务描述\n   </task>\n   ```\n\n2. **当你有进展时，更新任务**：\n   ```\n   <task id="task-{timestamp}-{shortid}" status="running" progress="35">\n   更新：完成了某某步骤\n   </task>\n   ```\n\n3. **当任务有子步骤时，创建子任务**：\n   ```\n   <subtask parent="task-{timestamp}-{shortid}" id="subtask-{timestamp}-{shortid}" title="子步骤标题" status="pending">\n   子步骤描述\n   </subtask>\n   ```\n\n4. **当任务完成时，标记完成**：\n   ```\n   <task id="task-{timestamp}-{shortid}" status="completed" progress="100">\n   任务已完成，总结结果\n   </task>\n   ```\n\n5. **如果任务失败，标记失败并说明原因**：\n   ```\n   <task id="task-{timestamp}-{shortid}" status="failed" progress="0">\n   失败原因说明\n   </task>\n   ```\n\n### 状态值：\n- `pending`：尚未开始\n- `running`：正在执行\n- `completed`：已完成\n- `failed`：失败\n\n### 重要提示：\n- 更新任务时**必须使用同一个 id**\n- 进度 progress 是 0-100 的百分比\n- 子任务必须引用父任务的 id\n- 标题保持简洁（50字以内）\n- 这些标记对用户不可见，系统会自动解析\n'
    os.write(soulPath, soulContent + taskOverlay)
    console.log('[AGENT] Injected Task Management into SOUL.md')
  }

  // Record to database
  db.createAgent({
    agent_name: agentName,
    display_name: displayName || null,
    description: description || null,
    directory: agentDir,
    config_path: configPath,
    workspace_dir: workspaceDir,
    port: port
  })

  console.log('[AGENT] Agent created successfully: ' + agentName + ', port=' + port)

  return {
    agent_name: agentName,
    display_name: displayName,
    description: description || null,
    directory: agentDir,
    port: port,
    status: 'created'
  }
}

// Create the 0#Agent using hub-provided config.toml content
// Returns true on success, false on failure (non-blocking)
function createZeroAgentFromConfig(configTomlContent) {
  var agentName = '0#Agent'

  // Skip if already exists
  if (db.getAgent(agentName)) {
    console.log('[AGENT] 0#Agent already exists, skipping creation')
    return false
  }

  try {
    // Allocate port
    var port = allocatePort()
    console.log('[AGENT] Allocated port for 0#Agent: ' + port)

    // Create directory structure using display_name
    var agentsDir = os.path.join(rootDir, 'agents')
    var dirName = makeDisplayDirName('0#Agent', agentsDir)
    var agentDir = os.path.join(agentsDir, dirName)
    var workspaceDir = os.path.join(agentDir, 'workspace')

    os.mkdir(workspaceDir, { recursive: true })
    console.log('[AGENT] Created 0#Agent directory: ' + agentDir)

    // Save hub config as template for future agents
    var templatePath = os.path.join(rootDir, 'zeroclaw-template.toml')
    os.write(templatePath, configTomlContent)
    console.log('[AGENT] Saved hub config as template: ' + templatePath)

    // Patch config to disable pairing for 0#Agent (managed by ClawParty)
    var patchedConfig = ensurePairingDisabled(configTomlContent)

    var configPath = os.path.join(agentDir, 'config.toml')
    os.write(configPath, patchedConfig)
    console.log('[AGENT] Wrote config.toml with pairing disabled: ' + configPath)

    // Record to database
    db.createAgent({
      agent_name: agentName,
      display_name: '0#Agent',
      description: 'System agent created from hub config',
      directory: agentDir,
      config_path: configPath,
      workspace_dir: workspaceDir,
      port: port
    })

    console.info('[AGENT] 0#Agent created successfully')

    // Start the agent
    try {
      startAgent(agentName)
      console.info('[AGENT] 0#Agent started')
    } catch (e) {
      console.error('[AGENT] 0#Agent created but failed to start:', e)
    }

    return true
  } catch (e) {
    console.error('[AGENT] Failed to create 0#Agent:', e)
    return false
  }
}

// Auto-discover existing ZeroClaw instance (e.g. started via start.bat)
// Returns true if a running ZeroClaw was discovered and registered as 0#Agent
// Try to register a pre-existing ZeroClaw daemon (started externally, e.g.
// via Windows start.bat) as 0#Agent.  This is called lazily from
// allAgentStatuses() / getAgentStatus() so it never blocks startup.
function discoverExistingZeroClaw() {
  // Register the external ZeroClaw daemon (started by start.bat) as 0#Agent
  // so that the zAgents sidebar panel shows it.  Called lazily by
  // allAgentStatuses().
  var agentName = '0#Agent'
  var DEFAULT_PORT = 42617

  if (db.getAgent(agentName)) return false

  console.log('[AGENT] Registering discovered 0#Agent on port ' + DEFAULT_PORT)

  // Ensure the per-agent directory tree exists under ~/.clawparty/agents/
  var agentsDir = os.path.join(rootDir, 'agents')
  try { os.mkdir(agentsDir) } catch {}

  var agentDir = os.path.join(agentsDir, '0#Agent')
  var workspaceDir = os.path.join(agentDir, 'workspace')
  try { os.mkdir(agentDir) } catch {}
  try { os.mkdir(workspaceDir, { recursive: true }) } catch {}

  // Try to pull in the real config that start.bat created.
  // If the file is missing we simply write a minimal stub.
  var configPath = os.path.join(agentDir, 'config.toml')
  var configContent = ''
  var globalConfigPath = os.path.join(os.home(), '.zeroclaw', 'config.toml')
  try {
    configContent = os.read(globalConfigPath).toString()
  } catch {
    try {
      var hubTemplatePath = os.path.join(rootDir, 'zeroclaw-template.toml')
      configContent = os.read(hubTemplatePath).toString()
    } catch {
      configContent = '[general]\nmodel = "gpt-4o-mini"\nrequire_pairing = false\n'
    }
  }
  configContent = ensurePairingDisabled(configContent)
  os.write(configPath, configContent)

  // Minimal SOUL.md (so the agent directory looks complete)
  try {
    os.write(os.path.join(workspaceDir, 'SOUL.md'),
             '# 0#Agent\n\nSystem agent for ZeroClaw integration.\n')
  } catch {}

  // Persist into ztm.db — status is "running" because the external
  // zeroclaw process was started *before* ztm by start.bat.
  db.createAgent({
    agent_name: agentName,
    display_name: '0#Agent',
    description: 'System agent discovered from running ZeroClaw instance',
    directory: agentDir,
    config_path: configPath,
    workspace_dir: workspaceDir,
    port: DEFAULT_PORT
  })
  db.updateAgentStatus(agentName, 'running', null, null)

  console.log('[AGENT] 0#Agent registered successfully')
  return true
}

function deleteAgent(agentName) {
  if (agentName === '0#Agent') {
    console.log('[AGENT] Delete rejected: 0#Agent is a system agent and cannot be deleted')
    throw 'Cannot delete system agent: 0#Agent'
  }
  if (db.isGroupOwnerAgent(agentName)) {
    console.log('[AGENT] Delete rejected: ' + agentName + ' is a group owner agent and cannot be deleted')
    throw 'Cannot delete group owner agent: ' + agentName
  }

  console.log('[AGENT] Deleting agent: ' + agentName)

  var agent = db.getAgent(agentName)
  if (!agent) {
    console.log('[AGENT] Delete failed: agent not found: ' + agentName)
    throw 'Agent not found: ' + agentName
  }

  // Stop if running
  if (agent.status === 'running' || agent.status === 'starting') {
    console.log('[AGENT] Stopping agent before delete: ' + agentName)
    stopAgent(agentName)
  }

  // Soft-delete in database (directory is kept for potential recovery)
  db.deleteAgent(agentName)
  console.log('[AGENT] Agent soft-deleted: ' + agentName)

  return { status: 'deleted', agent_name: agentName }
}

function startAgent(agentName) {
  console.log('[AGENT] Starting agent: ' + agentName)
  
  var agent = db.getAgent(agentName)
  if (!agent) {
    console.log('[AGENT] Start failed: agent not found: ' + agentName)
    throw 'Agent not found: ' + agentName
  }
  
  // Fast check: verify PID exists AND holds the expected listen port
  // (prevents false positives from PID reuse)
  var currentPid = agent.pid
  if (currentPid && isAgentProcessAlive(currentPid, agent.port)) {
    console.log('[AGENT] Start skipped: agent already running with PID: ' + currentPid)
    db.updateAgentStatus(agentName, 'running', currentPid, null)
    throw 'Agent already running: ' + agentName
  }
  // Fallback to slow lsof if no pid or fast check failed but process might exist
  if (!currentPid) {
    currentPid = findZeroclawPid(agent.port)
    if (currentPid) {
      console.log('[AGENT] Start skipped: agent already running (found via lsof) PID: ' + currentPid)
      db.updateAgentStatus(agentName, 'running', currentPid, null)
      throw 'Agent already running: ' + agentName
    }
  }
  
  if (agent.status === 'starting' || agent.status === 'running') {
    console.log('[AGENT] Agent was marked ' + agent.status + ' but process is dead, will restart')
    db.updateAgentStatus(agentName, 'stopped', null, null)
  }
  
  // Build command - use array format for pipeline.exec
  var zeroclawBase = os.path.join(os.path.dirname(pipy.argv[0]), 'zeroclaw')
  // Append .exe on Windows (harmless on Unix since path.concat just adds chars)
  var platform = os.platform || 'unknown'
  var isWin = (
    platform === 'win32' ||
    platform === 'win64' ||
    platform === 'windows' ||
    platform === 'win'
  )
  console.log('[AGENT] Detected platform: ' + platform + ', isWin=' + isWin)
  var zeroclawPath = isWin ? zeroclawBase + '.exe' : zeroclawBase
  var cmd = [zeroclawPath, 'daemon', '--config-dir', agent.directory, '-p', agent.port.toString()]
  console.log('[AGENT] Command: ' + cmd.join(' '))
  
  // Create pipeline to execute zeroclaw daemon
  var $zcPid = null
  var $zcExitCode = 0
  var $zcErrorMessage = ''
  var $zcOutput = ''
  var $zcStartTime = Date.now()
  
  var zeroclawPipeline = pipeline($=>$
    .onStart(function() { $zcStartTime = Date.now(); return new Data })
    .exec(function() { return cmd }, {
      stdout: true,
      stderr: true,
      onExit: function(code, err) {
        $zcExitCode = code
        if (err) {
          $zcErrorMessage = err.toString()
          console.error('[AGENT] ZeroClaw error:', $zcErrorMessage)
        }
        console.log('[AGENT] ZeroClaw exited with code: ' + code)
        if ($zcOutput) {
          console.log('[AGENT] ZeroClaw output:\n' + $zcOutput)
        }
        db.updateAgentStatus(agentName, code === 0 ? 'stopped' : 'error', null, $zcErrorMessage || $zcOutput)
        return new StreamEnd
      }
    })
    .replaceStreamStart(function(evt) {
      // Try to get PID when process starts
      $zcPid = findZeroclawPid(agent.port)
      console.log('[AGENT] ZeroClaw started, PID: ' + $zcPid)
      if ($zcPid) {
        db.updateAgentStatus(agentName, 'starting', $zcPid, null)
      }
      return [new MessageStart, evt]
    })
    .replaceStreamEnd(function() { return new MessageEnd })
    .replaceMessage(function(msg) {
      $zcOutput += msg?.body?.toString?.() || ''
      return msg
    })
    .onEnd(function() {
      var durationMs = Date.now() - $zcStartTime
      console.log('[AGENT] ZeroClaw ran for ' + durationMs + 'ms')
      $zcErrorMessage = ''
      $zcOutput = ''
      return 'started'
    })
  )
  
  // Spawn the pipeline (async, non-blocking)
  try {
    zeroclawPipeline.spawn()
    console.log('[AGENT] ZeroClaw pipeline spawned')
    
    // Update status immediately
    db.updateAgentStatus(agentName, 'starting', null, null)
    
    return {
      agent_name: agentName,
      pid: null,  // Will be available asynchronously
      status: 'starting'
    }
  } catch (e) {
    console.log('[AGENT] Start failed: ' + e)
    db.updateAgentStatus(agentName, 'error', null, e.message || e)
    throw 'Failed to start agent: ' + e
  }
}

function _parsePidFromString(str) {
  var num = 0
  for (var k = 0; k < str.length; k++) {
    var digit = str.charCodeAt(k) - 48  // '0' = 48
    if (digit >= 0 && digit <= 9) {
      num = num * 10 + digit
    }
  }
  return num
}

function _manualSplitBySpace(line) {
  // PipyJS does not support .split(/\s+/) — split manually
  var parts = []
  var part = ''
  for (var j = 0; j < line.length; j++) {
    var c = line.charAt(j)
    if (c === ' ' || c === '\t') {
      if (part.length > 0) {
        parts.push(part)
        part = ''
      }
    } else {
      part = part + c
    }
  }
  if (part.length > 0) parts.push(part)
  return parts
}

function isAgentProcessAlive(pid, port) {
  // Combined PID + port verification to avoid false positives from PID reuse.
  // Steps:
  //   1) Basic sanity: pid must be positive and within OS plausible range.
  //   2) kill -0 (Unix) or tasklist (Windows) to confirm PID exists.
  //   3) lsof (Unix) or netstat (Windows) to confirm the PID is listening on the expected port.
  // Any step fails => process is dead or not our agent.
  if (!pid || pid <= 0 || pid > 10000000) return false

  // Step 1: is the PID alive at all?
  var pidAlive = false
  try {
    pipy.exec('kill -0 ' + pid)
    pidAlive = true
  } catch (e) {
    try {
      pipy.exec('tasklist /FI "PID eq ' + pid + '"')
      pidAlive = true
    } catch (e2) {
      return false
    }
  }
  if (!pidAlive) return false

  // Step 2: is this PID listening on the expected port?
  // Try Unix lsof first
  try {
    var out = pipy.exec('lsof -a -p ' + pid + ' -i:' + port + ' -P -n -sTCP:LISTEN')
    if (out && out.toString().indexOf('(LISTEN)') >= 0) return true
  } catch (e) {}

  // Fallback: macOS / older lsof may need different pattern
  try {
    var out2 = pipy.exec('lsof -i:' + port + ' -P -n -sTCP:LISTEN')
    if (out2) {
      var lines = out2.toString().split('\n')
      for (var k = 0; k < lines.length; k++) {
        var line = lines[k]
        if (line.indexOf('(LISTEN)') >= 0 && line.indexOf(':' + port) >= 0) {
          var parts = _manualSplitBySpace(line)
          // lsof format: COMMAND PID USER FD TYPE ... NAME
          if (parts.length >= 2) {
            var pidNum = _parsePidFromString(parts[1])
            if (pidNum === pid) return true
          }
        }
      }
    }
  } catch (e2) {}

  // Windows fallback: netstat
  try {
    var netResult = pipy.exec('netstat -ano')
    if (netResult) {
      var netLines = netResult.toString().split('\n')
      for (var m = 0; m < netLines.length; m++) {
        var netLine = netLines[m]
        if (netLine.indexOf('LISTENING') >= 0 && netLine.indexOf(':' + port) >= 0) {
          var netParts = _manualSplitBySpace(netLine)
          if (netParts.length > 0) {
            var netPid = _parsePidFromString(netParts[netParts.length - 1])
            if (netPid === pid) return true
          }
        }
      }
    }
  } catch (e3) {}

  return false
}

function findZeroclawPid(port) {
  // Fallback: find zeroclaw process by port using lsof (Unix) or netstat (Windows)
  // WARNING: lsof/netstat scans system state and blocks Pipy's single-threaded event loop
  // Only used when pid is not recorded or when startAgent needs to verify process
  try {
    // Fast path: Unix lsof (e.g. "lsof -ti:42617" returns PID directly)
    var execResult = pipy.exec('lsof -ti:' + port)
    if (execResult) {
      var result = execResult.toString().trim()
      if (result && result.length > 0) {
        var num = _parsePidFromString(result)
        if (num > 0) return num
      }
    }
  } catch (e) {
    // lsof not available (Windows), try netstat
  }

  // macOS lsof full output (has (LISTEN), not LISTENING)
  try {
    var execResult3 = pipy.exec('lsof -i:' + port + ' -P -n -sTCP:LISTEN')
    if (execResult3) {
      var lines3 = execResult3.toString().split('\n')
      for (var n = 0; n < lines3.length; n++) {
        var line3 = lines3[n]
        if (line3.indexOf('(LISTEN)') >= 0 && line3.indexOf(':' + port) >= 0) {
          var parts3 = _manualSplitBySpace(line3)
          // lsof format: COMMAND PID USER FD TYPE ... NAME
          if (parts3.length >= 2) {
            var pidNum3 = _parsePidFromString(parts3[1])
            if (pidNum3 > 0) return pidNum3
          }
        }
      }
    }
  } catch (e2) {}

  try {
    var execResult2 = pipy.exec('netstat -ano')
    if (!execResult2) return null
    var lines = execResult2.toString().split('\n')
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i]
      // Windows netstat: "TCP  127.0.0.1:42617  0.0.0.0:0  LISTENING  12345"
      if (line.indexOf('LISTENING') >= 0 && line.indexOf(':' + port) >= 0) {
        var parts = _manualSplitBySpace(line)
        for (var j = parts.length - 1; j >= 0; j--) {
          var maybePid = parts[j].trim()
          if (maybePid.length > 0) {
            var num = _parsePidFromString(maybePid)
            if (num > 0) return num
          }
        }
      }
    }
  } catch (e) {}

  return null
}

function stopAgent(agentName) {
  console.log('[AGENT] Stopping agent: ' + agentName)

  if (agentName === '0#Agent') {
    // 0#Agent is a managed system agent: only mark as stopped in DB,
    // do NOT kill the global zeroclaw daemon process.
    console.log('[AGENT] 0#Agent stop: only updating DB state (not killing process)')
    db.updateAgentStatus(agentName, 'stopped', null, null)
    return { status: 'stopped', agent_name: agentName }
  }

  var agent = db.getAgent(agentName)
  if (!agent) {
    console.log('[AGENT] Stop failed: agent not found: ' + agentName)
    throw 'Agent not found: ' + agentName
  }

  if (agent.status !== 'running' && agent.status !== 'starting') {
    console.log('[AGENT] Stop skipped: agent not running: ' + agentName)
    throw 'Agent not running: ' + agentName
  }

  var pid = agent.pid
  if (!pid) {
    // Try to find PID by port
    pid = findZeroclawPid(agent.port)
    if (!pid) {
      console.log('[AGENT] Stop failed: no PID found: ' + agentName)
      db.updateAgentStatus(agentName, 'stopped', null, null)
      return { status: 'stopped', agent_name: agentName }
    }
  }

  try {
    // Unix: send SIGTERM
    pipy.exec('kill -TERM ' + pid)
  } catch (e) {
    // Fallback: try taskkill on Windows
    try {
      pipy.exec('taskkill /PID ' + pid + ' /F')
    } catch (e2) {
      console.log('[AGENT] Stop failed: ' + e)
      db.updateAgentStatus(agentName, 'error', null, e.message || e)
      throw 'Failed to stop agent: ' + e
    }
  }

  // Update status immediately
  db.updateAgentStatus(agentName, 'stopped', null, null)

  console.log('[AGENT] Agent stop signal sent: ' + agentName)
  return { status: 'stopped', agent_name: agentName }
}

// Sanitize group name to valid agent name
// Note: uses char-by-char loop because PipyJS does not support RegExp
function sanitizeAgentName(name) {
  if (typeof name !== 'string') return 'agent'
  var s = name.toLowerCase()
  var chars = []
  var prevHyphen = false
  for (var i = 0; i < s.length; i++) {
    var c = s.charAt(i)
    var code = c.charCodeAt(0)
    if ((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c === '-' || code >= 128) {
      chars.push(c)
      prevHyphen = false
    } else {
      if (!prevHyphen) {
        chars.push('-')
        prevHyphen = true
      }
    }
  }
  // Trim leading hyphens
  var start = 0
  for (var i = 0; i < chars.length; i++) {
    if (chars[i] !== '-') {
      start = i
      break
    }
  }
  // Trim trailing hyphens
  var end = chars.length - 1
  for (var j = chars.length - 1; j >= 0; j--) {
    if (chars[j] !== '-') {
      end = j
      break
    }
  }
  if (start > end) return 'group'
  // Collapse multiple hyphens (already done during loop via prevHyphen)
  var result = ''
  for (var k = start; k <= end; k++) result += chars[k]
  if (!result) return 'group'
  return result
}

function createGroupOwnerAgent(groupId, groupName, memberAgents) {
  console.log('[GROUP] Creating group owner agent for group: ' + groupName)

  var ownerAgentName = sanitizeAgentName(groupName)

  // De-duplicate: if name exists, append -1, -2, etc.
  var uniqueName = ownerAgentName
  var suffix = 1
  for (suffix = 1; suffix <= 1000; suffix++) {
    if (!db.getAgent(uniqueName) && !db.getGroupChat(uniqueName)) break
    uniqueName = ownerAgentName + '-' + suffix
  }
  if (suffix > 1000) throw 'Could not find unique agent name for group'
  ownerAgentName = uniqueName

  // Create the agent using existing template logic
  var port = allocatePort()
  var agentsDir = os.path.join(rootDir, 'agents')
  var dirName = makeDisplayDirName(groupName, agentsDir)
  var agentDir = os.path.join(agentsDir, dirName)
  var workspaceDir = os.path.join(agentDir, 'workspace')

  os.mkdir(workspaceDir, { recursive: true })

  // Read template: prefer hub-distributed config, fallback to local
  var hubTemplatePath = os.path.join(rootDir, 'zeroclaw-template.toml')
  var localTemplatePath = os.path.join(os.home(), '.zeroclaw', 'config.toml')
  var templateContent

  try {
    templateContent = os.read(hubTemplatePath).toString()
  } catch (e) {
    templateContent = os.read(localTemplatePath).toString()
  }

  // Patch config to disable pairing
  var patchedConfig = ensurePairingDisabled(templateContent)

  var configPath = os.path.join(agentDir, 'config.toml')
  os.write(configPath, patchedConfig)

  // Record to database
  db.createAgent({
    agent_name: ownerAgentName,
    display_name: groupName,
    description: 'Group owner agent for "' + groupName + '"',
    directory: agentDir,
    config_path: configPath,
    workspace_dir: workspaceDir,
    port: port
  })

  // Record group chat
  db.createGroupChat({
    group_id: groupId,
    group_name: groupName,
    owner_agent: ownerAgentName,
    members: memberAgents || [],
    session_id: groupId
  })

  console.log('[GROUP] Group owner agent created: ' + ownerAgentName + ', port=' + port)

  // Start the agent immediately
  try {
    startAgent(ownerAgentName)
    console.log('[GROUP] Group owner agent started: ' + ownerAgentName)
  } catch (e) {
    console.error('[GROUP] Group owner agent created but failed to start:', e)
  }

  return {
    agent_name: ownerAgentName,
    group_id: groupId,
    port: port,
    status: 'created'
  }
}

function _sortAgents(agents) {
  var result = []
  for (var i = 0; i < agents.length; i++) {
    result.push(agents[i])
  }
  result.sort(function(a, b) {
    if (a.agent_name === '0#Agent') return -1
    if (b.agent_name === '0#Agent') return 1
    if (a.agent_name < b.agent_name) return -1
    if (a.agent_name > b.agent_name) return 1
    return 0
  })
  return result
}

function _refreshAgentStatus(agent) {
  if (!agent) return null
  if (agent.status !== 'starting' && agent.status !== 'running') return agent

  // Fast path: verify PID exists AND holds the listen port (prevents PID-reuse false positives)
  var pid = agent.pid
  if (pid && isAgentProcessAlive(pid, agent.port)) {
    // Confirmed as our agent process — unconditionally sync DB
    db.updateAgentStatus(agent.agent_name, 'running', pid, null)
    agent.status = 'running'
    agent.error_msg = null
    return agent
  }

  // Fallback path: recorded PID is dead/wrong, but zeroclaw may still be running on the port
  var foundPid = findZeroclawPid(agent.port)
  if (foundPid) {
    agent.status = 'running'
    agent.pid = foundPid
    agent.error_msg = null
    db.updateAgentStatus(agent.agent_name, 'running', foundPid, null)
  } else if (agent.status === 'starting') {
    // Still starting, keep waiting
    agent.status = 'starting'
  } else {
    // Process genuinely disappeared
    agent.status = 'stopped'
    agent.pid = null
    db.updateAgentStatus(agent.agent_name, 'stopped', null, null)
  }
  return agent
}

function getAgentStatus(agentName) {
  // Lazy discovery: if ZERO agent is requested but not yet registered,
  // try to register the external zeroclaw daemon first.
  if (agentName === '0#Agent' && !db.getAgent(agentName)) {
    discoverExistingZeroClaw()
  }

  var now = Date.now()
  var cached = _agentStatusCache.data[agentName]
  if (cached && (now - cached._ts) < AGENT_STATUS_CACHE_TTL) {
    return cached
  }
  
  var agent = db.getAgent(agentName)
  if (!agent) {
    throw 'Agent not found: ' + agentName
  }
  
  agent = _refreshAgentStatus(agent)
  agent._ts = now
  _agentStatusCache.data[agentName] = agent
  _agentStatusCache.ts = now
  return agent
}

function checkGatewayHealth(port, timeoutMs) {
  timeoutMs = timeoutMs || 2000
  // Try to check health
  try {
    var res = http.get('http://127.0.0.1:' + port + '/health')
    if (res && res.head.status === 200) {
      return true
    }
  } catch (e) {
    // Ignore errors
  }
  return false
}

function allAgentStatuses() {
  // Lazy discovery: if there is no 0#Agent try to register the
  // external zeroclaw daemon that may have been started via start.bat.
  // We must re-fetch db.allAgents() *after* discoverExistingZeroClaw()
  // runs so the newly created record shows up in the returned list.
  if (!db.getAgent('0#Agent')) {
    discoverExistingZeroClaw()
  }

  var now = Date.now()
  var agents = db.allAgents()

  // Refresh each running/starting agent using fast pid checks
  for (var i = 0; i < agents.length; i++) {
    var agent = agents[i]
    var cached = _agentStatusCache.data[agent.agent_name]
    if (cached && (now - cached._ts) < AGENT_STATUS_CACHE_TTL) {
      agents[i] = cached
    } else {
      agent = _refreshAgentStatus(agent)
      agent._ts = now
      _agentStatusCache.data[agent.agent_name] = agent
    }
  }

  _agentStatusCache.ts = now
  return _sortAgents(agents)
}

// Scan ~/.clawparty/agents for directories that look like valid agent
// installations but are missing from the DB.  Add them back so the UI
// can see and manage them.
function _scanOrphanAgents() {
  var agentsDir = os.path.join(rootDir, 'agents')
  var entries = []
  try { entries = os.readDir(agentsDir) } catch (e) { entries = [] }

  var added = 0
  for (var i = 0; i < entries.length; i++) {
    var dirName = entries[i]
    // os.readDir may return trailing slashes for directories
    if (dirName.endsWith('/')) dirName = dirName.substring(0, dirName.length - 1)
    var agentDir = os.path.join(agentsDir, dirName)

    var isDir = false
    try {
      var st = os.stat(agentDir)
      if (st && st.isDirectory()) isDir = true
    } catch (e) {}

    if (isDir) {
      var hasConfig = false
      var hasWorkspace = false
      try {
        os.stat(os.path.join(agentDir, 'config.toml'))
        hasConfig = true
      } catch (e) {}
      try {
        var wsStat = os.stat(os.path.join(agentDir, 'workspace'))
        if (wsStat && wsStat.isDirectory()) hasWorkspace = true
      } catch (e) {}

      if (hasConfig && hasWorkspace) {
        if (db.agentExists(dirName)) {
          console.log('[AGENT] Skipping orphan scan for ' + dirName + ': already in DB (possibly soft-deleted)')
        } else if (db.directoryHasAgent(agentDir)) {
          console.log('[AGENT] Skipping orphan scan for ' + dirName + ': directory already owned by an active agent')
        } else {
          try {
            var port = allocatePort()
            db.createAgent({
              agent_name: dirName,
              display_name: dirName,
              description: null,
              directory: agentDir,
              config_path: os.path.join(agentDir, 'config.toml'),
              workspace_dir: os.path.join(agentDir, 'workspace'),
              port: port
            })
            added++
            console.log('[AGENT] Reconciled orphan agent directory: ' + dirName + ', port=' + port)
          } catch (e) {
            console.log('[AGENT] Failed to reconcile orphan agent ' + dirName + ': ' + e)
          }
        }
      }
    }
  }

  if (added > 0) {
    console.log('[AGENT] Reconciled ' + added + ' orphan agent(s) from filesystem')
  }
}

// Force reconcile: clear caches and verify every agent against actual OS process state
function reconcileAgentStatuses() {
  _scanOrphanAgents()

  var agents = db.allAgents()
  var reconciled = []

  for (var i = 0; i < agents.length; i++) {
    var agent = agents[i]
    // Always force a fresh check (bypass cache)
    delete _agentStatusCache.data[agent.agent_name]
    agent = _refreshAgentStatus(agent)
    agent._ts = Date.now()
    _agentStatusCache.data[agent.agent_name] = agent
    reconciled.push(agent)
  }

  _agentStatusCache.ts = Date.now()

  // Sync desktop symlinks / shortcuts for each agent
  var desktopPath = os.path.join(os.home(), 'Desktop')
  var isWin = (os.platform === 'win32' || os.platform === 'windows')

  try {
    var desktopEntries = []
    try { desktopEntries = os.readDir(desktopPath) } catch (e) { desktopEntries = [] }

    for (var j = 0; j < reconciled.length; j++) {
      var ag = reconciled[j]
      var linkName = (ag.agent_name === '0#Agent') ? 'zAgent_0' : 'zAgent_' + ag.agent_name
      var expectedEntry = isWin ? linkName + '.lnk' : linkName
      var linkPath = os.path.join(desktopPath, linkName)

      // Skip if a file / directory / link with the same name already exists on Desktop
      var conflict = false
      for (var k = 0; !conflict && k < desktopEntries.length; k++) {
        var entry = desktopEntries[k]
        if (entry.endsWith('/')) entry = entry.substring(0, entry.length - 1)
        if (entry === expectedEntry) {
          conflict = true
        }
      }

      if (!conflict) {
        var targetPath = (ag.agent_name === '0#Agent')
          ? os.path.join(os.home(), '.clawparty', '.zeroclaw')
          : ag.directory

        // Skip if target directory does not exist
        var targetExists = false
        try {
          os.stat(targetPath)
          targetExists = true
        } catch (e) { }

        if (targetExists) {
          try {
            if (isWin) {
              var psPath = os.path.join(os.home(), '.clawparty', 'tmp_create_link.ps1')
              var psContent = '$ws = New-Object -ComObject WScript.Shell\n'
              psContent += '$s = $ws.CreateShortcut("' + linkPath.replaceAll('"', '\\"') + '.lnk")\n'
              psContent += '$s.TargetPath = "' + targetPath.replaceAll('"', '\\"') + '"\n'
              psContent += '$s.Save()\n'
              os.write(psPath, psContent)
              pipy.exec('powershell -ExecutionPolicy Bypass -File "' + psPath.replaceAll('"', '\\"') + '"')
              try { os.unlink(psPath) } catch (e) {}
            } else {
              pipy.exec('ln -s "' + targetPath.replaceAll('"', '\\"') + '" "' + linkPath.replaceAll('"', '\\"') + '"')
            }
            console.log('[AGENT] Created desktop link for ' + ag.agent_name + ': ' + linkPath + ' -> ' + targetPath)
          } catch (e) {
            console.error('[AGENT] Failed to create desktop link for ' + ag.agent_name + ': ' + e)
          }
        } else {
          console.log('[AGENT] Skipping desktop link for ' + ag.agent_name + ': target missing: ' + targetPath)
        }
      }
    }
  } catch (e) {
    console.error('[AGENT] Desktop link sync failed:', e)
  }

  return _sortAgents(reconciled)
}

export default {
  init,
  setIdentity,
  getIdentity,
  allMeshes,
  getMesh,
  getMeshLog,
  setMesh,
  delMesh,
  getPermit,
  allHubs,
  setHub,
  getHub,
  getHubLog,
  createInviteCode,
  allEndpoints,
  getEndpoint,
  getEndpointLabels,
  setEndpointLabels,
  getEndpointLog,
  allUsers,
  delUser,
  allFiles,
  getFileInfo,
  delFileInfo,
  getFileData,
  setFileData,
  delFileData,
  getFileDataFromEP,
  allApps,
  getApp,
  setApp,
  delApp,
  getAppLog,
  connectApp,
  getEndpointStats,
  // AI-Agent management
  createAgent,
  createZeroAgentFromConfig,
  discoverExistingZeroClaw,
  deleteAgent,
  startAgent,
  stopAgent,
  getAgentStatus,
  allAgentStatuses,
  reconcileAgentStatuses,
  // Group chat
  createGroupOwnerAgent,
  sanitizeAgentName,
  pingEndpoint,
  getLocalTemplates,
  getSharedTemplates,
  installLocalTemplate,
  installSharedTemplate,
  // Global config management
  saveGlobalConfig: config.saveGlobalConfig,
  loadGlobalConfig: config.loadGlobalConfig,
  mergeConfig: config.mergeConfig,
}
