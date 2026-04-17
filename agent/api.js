import db from './db.js'
import Mesh from './mesh.js'
import templates from './templates.js'

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
  db.allMeshes().forEach(
    function (mesh) {
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
  )
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
  return Object.values(meshes).map(
    (mesh) => mesh.getStatus()
  )
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

function allEndpoints(mesh, id, name, user, keyword, offset, limit) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve([])
  var idLocal = m.config.agent.id
  return m.discoverEndpoints(id, name, user, keyword, offset, limit).then(
    list => list.map(ep => ({ ...ep, isLocal: (ep.id === idLocal) }))
  )
}

function getEndpoint(mesh, ep) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  return m.findEndpoint(ep).then(
    ep => ({ ...ep, isLocal: (ep.id === m.config.agent.id) })
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
  return m.discoverUsers(name, keyword, offset, limit).then(
    results => {
      var idLocal = m.config.agent.id
      results.forEach(user => {
        user.endpoints?.instances?.forEach?.(
          ep => {
            if (ep.id === idLocal) ep.isLocal = true
          }
        )
      })
      return results
    }
  )
}

function delUser(mesh, name) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve([])
  return m.evictUser(name)
}

function allFiles(mesh, since) {
  var m = meshes[mesh]
  if (!m) return Promise.resolve(null)
  return m.discoverFiles(since)
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
  return m.findApp(ep, provider, app).then(ret => {
    if (ret) return
    return m.installApp(ep, provider, app)
  }).then(() => {
    if (!('isDisabled' in state)) return
    if (state.isDisabled) {
      return m.disableApp(ep, provider, app)
    } else {
      return m.enableApp(ep, provider, app)
    }
  }).then(() => {
    if (!('isRunning' in state)) return
    if (state.isRunning) {
      return m.startApp(ep, provider, app)
    } else {
      return m.stopApp(ep, provider, app)
    }
  }).then(() => {
    if (!('isPublished' in state)) return
    if (state.isPublished) {
      return m.publishApp(ep, provider, app)
    } else {
      return m.unpublishApp(ep, provider, app)
    }
  }).then(() => m.findApp(ep, provider, app))
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
  var PORT_START = 42617
  var PORT_END = 42700
  
  // Get used ports from database
  var usedPorts = db.allAgents().map(function(a) { return a.port })
  
  // Find available port
  for (var port = PORT_START; port <= PORT_END; port++) {
    if (!usedPorts.includes(port) && !db.isPortUsed(port)) {
      return port
    }
  }
  
  throw 'No available ports in range ' + PORT_START + '-' + PORT_END
}

function createAgent(agentName, displayName) {
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
  
  // Copy config template
  var templatePath = os.path.join(rootDir, '.zeroclaw', 'config.toml')
  var configPath = os.path.join(agentDir, 'config.toml')
  
  var templateContent = os.read(templatePath)
  os.write(configPath, templateContent)
  console.log('[AGENT] Copied config: ' + configPath)
  
  // Record to database
  db.createAgent({
    agent_name: agentName,
    display_name: displayName || null,
    directory: agentDir,
    config_path: configPath,
    workspace_dir: workspaceDir,
    port: port
  })
  
  console.log('[AGENT] Agent created successfully: ' + agentName + ', port=' + port)
  
  return {
    agent_name: agentName,
    display_name: displayName,
    directory: agentDir,
    port: port,
    status: 'created'
  }
}

function deleteAgent(agentName) {
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
  
  if (agent.status === 'running') {
    console.log('[AGENT] Start skipped: agent already running: ' + agentName)
    throw 'Agent already running: ' + agentName
  }
  
  if (agent.status === 'starting') {
    console.log('[AGENT] Start skipped: agent already starting: ' + agentName)
    throw 'Agent is starting: ' + agentName
  }
  
  // Build command - use array format for pipeline.exec
  var zeroclawPath = '/Users/caishu/github/clawparty/bin/zeroclaw'
  var cmd = [zeroclawPath, 'daemon', '--config-dir', agent.directory, '-p', agent.port.toString()]
  console.log('[AGENT] Command: ' + cmd.join(' '))
  
  // Create pipeline to execute zeroclaw daemon
  var $zcPid = null
  var $zcExitCode = 0
  var $zcErrorMessage = ''
  var $zcStartTime = Date.now()
  
  var zeroclawPipeline = pipeline($=>$
    .onStart(() => { $zcStartTime = Date.now(); return new Data })
    .exec(() => cmd, {
      stdout: true,
      stderr: true,
      onExit: (code, err) => {
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
    .replaceStreamStart(evt => {
      // Try to get PID when process starts
      $zcPid = findZeroclawPid(agent.port)
      console.log('[AGENT] ZeroClaw started, PID: ' + $zcPid)
      if ($zcPid) {
        db.updateAgentStatus(agentName, 'starting', $zcPid, null)
      }
      return [new MessageStart, evt]
    })
    .replaceStreamEnd(() => new MessageEnd)
    .onEnd(() => {
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
  agents.forEach(function(agent) {
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
  })
  
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
  deleteAgent,
  startAgent,
  stopAgent,
  getAgentStatus,
  allAgentStatuses,
  pingEndpoint,
  getLocalTemplates,
  getSharedTemplates,
  installLocalTemplate,
  installSharedTemplate,
}
