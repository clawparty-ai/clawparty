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

function createAgent(agentName, displayName, modelConfig, description, workspaceFiles) {
  console.log('[AGENT] Creating agent: ' + agentName)

  // Check if agent already exists
  if (db.getAgent(agentName)) {
    console.log('[AGENT] Create failed: agent already exists: ' + agentName)
    throw 'Agent already exists: ' + agentName
  }

  // Allocate port
  var port = allocatePort()
  console.log('[AGENT] Allocated port: ' + port)

  // Create directory structure
  var agentsDir = os.path.join(rootDir, 'agents')
  var agentDir = os.path.join(agentsDir, agentName)
  var workspaceDir = os.path.join(agentDir, 'workspace')

  os.mkdir(agentDir, { recursive: true })
  console.log('[AGENT] Created directory: ' + agentDir)
  os.mkdir(workspaceDir, { recursive: true })
  console.log('[AGENT] Created workspace: ' + workspaceDir)

  // Read template: prefer hub-distributed config, fallback to local
  var hubTemplatePath = os.path.join(rootDir, 'zeroclaw-template.toml')
  var localTemplatePath = os.path.join(os.home(), '.zeroclaw', 'config.toml')
  var templatePath = hubTemplatePath
  var templateContent

  try {
    templateContent = os.read(hubTemplatePath).toString()
    console.log('[AGENT] Using hub-distributed config template: ' + hubTemplatePath)
  } catch (e) {
    console.log('[AGENT] Hub template not found, using local template: ' + localTemplatePath)
    templatePath = localTemplatePath
    templateContent = os.read(localTemplatePath).toString()
  }

  // Patch config to disable pairing (all agents managed by ClawParty)
  var patchedConfig = templateContent.replaceAll('require_pairing = true', 'require_pairing = false')

  var configPath = os.path.join(agentDir, 'config.toml')
  os.write(configPath, patchedConfig)
  console.log('[AGENT] Wrote config with pairing disabled: ' + configPath)

  // Write AI-generated workspace files if provided
  if (workspaceFiles) {
    if (workspaceFiles.soul_md) {
      os.write(os.path.join(workspaceDir, 'SOUL.md'), workspaceFiles.soul_md)
      console.log('[AGENT] Wrote SOUL.md')
    }
    if (workspaceFiles.agents_md) {
      os.write(os.path.join(workspaceDir, 'AGENTS.md'), workspaceFiles.agents_md)
      console.log('[AGENT] Wrote AGENTS.md')
    }
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

    // Create directory structure
    var agentsDir = os.path.join(rootDir, 'agents')
    var agentDir = os.path.join(agentsDir, agentName)
    var workspaceDir = os.path.join(agentDir, 'workspace')

    os.mkdir(agentDir, { recursive: true })
    os.mkdir(workspaceDir, { recursive: true })
    console.log('[AGENT] Created 0#Agent directory: ' + agentDir)

    // Save hub config as template for future agents
    var templatePath = os.path.join(rootDir, 'zeroclaw-template.toml')
    os.write(templatePath, configTomlContent)
    console.log('[AGENT] Saved hub config as template: ' + templatePath)

    // Patch config to disable pairing for 0#Agent (managed by ClawParty)
    var patchedConfig = configTomlContent.replaceAll('require_pairing = true', 'require_pairing = false')

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
  
  // Delete directory
  try {
    os.remove(agent.directory, { recursive: true })
    console.log('[AGENT] Deleted directory: ' + agent.directory)
  } catch (e) {
    console.log('[AGENT] Warning: failed to delete directory: ' + e.message)
  }
  
  // Delete from database
  db.deleteAgent(agentName)
  console.log('[AGENT] Agent deleted: ' + agentName)
  
  return { status: 'deleted', agent_name: agentName }
}

function startAgent(agentName) {
  console.log('[AGENT] Starting agent: ' + agentName)
  
  var agent = db.getAgent(agentName)
  if (!agent) {
    console.log('[AGENT] Start failed: agent not found: ' + agentName)
    throw 'Agent not found: ' + agentName
  }
  
  var currentPid = findZeroclawPid(agent.port)
  if (currentPid) {
    console.log('[AGENT] Start skipped: agent already running with PID: ' + currentPid)
    db.updateAgentStatus(agentName, 'running', currentPid, null)
    throw 'Agent already running: ' + agentName
  }
  
  if (agent.status === 'starting' || agent.status === 'running') {
    console.log('[AGENT] Agent was marked ' + agent.status + ' but process is dead, will restart')
    db.updateAgentStatus(agentName, 'stopped', null, null)
  }
  
  // Build command - use array format for pipeline.exec
  var zeroclawPath = os.path.join(os.path.dirname(pipy.argv[0]), 'zeroclaw')
  var cmd = [zeroclawPath, 'daemon', '--config-dir', agent.directory, '-p', agent.port.toString()]
  console.log('[AGENT] Command: ' + cmd.join(' '))
  
  // Create pipeline to execute zeroclaw daemon
  var $zcPid = null
  var $zcExitCode = 0
  var $zcErrorMessage = ''
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
        // Update status when process exits
        db.updateAgentStatus(agentName, code === 0 ? 'stopped' : 'error', null, $zcErrorMessage)
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
    .onEnd(function() {
      var durationMs = Date.now() - $zcStartTime
      console.log('[AGENT] ZeroClaw ran for ' + durationMs + 'ms')
      $zcErrorMessage = ''
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

function findZeroclawPid(port) {
  // Try to find zeroclaw process by port using lsof
  try {
    var execResult = pipy.exec('lsof -ti:' + port)
    if (execResult) {
      var result = execResult.toString().trim()
      if (result && result.length > 0) {
        // Parse number manually (avoid Number.parseInt)
        var num = 0
        for (var k = 0; k < result.length; k++) {
          var digit = result.charCodeAt(k) - 48  // '0' = 48
          if (digit >= 0 && digit <= 9) {
            num = num * 10 + digit
          }
        }
        if (num > 0) {
          return num
        }
      }
    }
  } catch (e) {
    // lsof failed
  }
  
  return null
}

function stopAgent(agentName) {
  console.log('[AGENT] Stopping agent: ' + agentName)
  
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
    // Send SIGTERM using kill command
    console.log('[AGENT] Sending SIGTERM to pid=' + pid)
    pipy.exec('kill -TERM ' + pid)
    
    // Update status immediately
    db.updateAgentStatus(agentName, 'stopped', null, null)
    
    console.log('[AGENT] Agent stop signal sent: ' + agentName)
    return { status: 'stopped', agent_name: agentName }
    
  } catch (e) {
    console.log('[AGENT] Stop failed: ' + e)
    db.updateAgentStatus(agentName, 'error', null, e.message || e)
    throw 'Failed to stop agent: ' + e
  }
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
  var agentDir = os.path.join(agentsDir, ownerAgentName)
  var workspaceDir = os.path.join(agentDir, 'workspace')

  os.mkdir(agentDir, { recursive: true })
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
  var patchedConfig = templateContent.replaceAll('require_pairing = true', 'require_pairing = false')

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

function getAgentStatus(agentName) {
  var agent = db.getAgent(agentName)
  if (!agent) {
    throw 'Agent not found: ' + agentName
  }
  
  // Update status based on current process state
  if (agent.status === 'starting' || agent.status === 'running') {
    var currentPid = findZeroclawPid(agent.port)
    if (currentPid) {
      agent.status = 'running'
      agent.pid = currentPid
      agent.error_msg = null
      db.updateAgentStatus(agentName, 'running', currentPid, null)
    } else if (agent.status === 'starting') {
      // Still starting, keep waiting
      agent.status = 'starting'
    } else {
      // Process disappeared
      agent.status = 'stopped'
      agent.pid = null
      db.updateAgentStatus(agentName, 'stopped', null, null)
    }
  }
  
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
  var agents = db.allAgents()

  // Update status for starting/running agents
  for (var i = 0; i < agents.length; i++) {
    var agent = agents[i]
    if (agent.status === 'starting' || agent.status === 'running') {
      var currentPid = findZeroclawPid(agent.port)
      if (currentPid) {
        agent.status = 'running'
        agent.pid = currentPid
        agent.error_msg = null
        db.updateAgentStatus(agent.agent_name, 'running', currentPid, null)
      } else if (agent.status === 'starting') {
        // Still starting
        agent.status = 'starting'
      } else {
        // Process stopped
        agent.status = 'stopped'
        agent.pid = null
        db.updateAgentStatus(agent.agent_name, 'stopped', null, null)
      }
    }
  }

  return agents
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
  deleteAgent,
  startAgent,
  stopAgent,
  getAgentStatus,
  allAgentStatuses,
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
