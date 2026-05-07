// Global configuration management for ClawParty
// Handles saving and loading of global LLM configuration from ~/.clawparty/global-config.toml

var rootDir = ''

function init(dirname) {
  rootDir = dirname
  var globalConfigPath = os.path.join(rootDir, 'global-config.toml')
  try {
    os.read(globalConfigPath)
  } catch (e) {
    var defaultContent = 'api_key = ""\n' +
      'api_url = ""\n' +
      'default_provider = "openai"\n' +
      'default_model = "gpt-4o-mini"\n' +
      'default_temperature = 0.7\n' +
      'provider_timeout_secs = 120\n\n' +
      '[gateway]\n' +
      'require_pairing = false\n'
    os.write(globalConfigPath, defaultContent)
    console.log('[AGENT] Created default global-config.toml: ' + globalConfigPath)
  }
}

// Parse TOML content into JavaScript object
function parseToml(content) {
  var result = {}
  var currentSection = result
  var lines = content.split('\n')

  for (var i = 0; i < lines.length; i++) {
    var line = lines[i].trim()

    if (line && !line.startsWith('#')) {
      // Parse section headers like [llm] or [metadata]
      if (line.startsWith('[') && line.endsWith(']')) {
        var sectionName = line.slice(1, -1)
        var parts = sectionName.split('.')
        currentSection = result
        for (var j = 0; j < parts.length; j++) {
          var part = parts[j]
          if (!currentSection[part]) currentSection[part] = {}
          currentSection = currentSection[part]
        }
      } else {
        // Parse key = value pairs
        var eqIndex = line.indexOf('=')
        if (eqIndex > 0) {
          var key = line.slice(0, eqIndex).trim()
          var value = line.slice(eqIndex + 1).trim()

          // Remove quotes from string values
          if (value.startsWith('"') && value.endsWith('"')) {
            value = value.slice(1, -1)
          }

          // Try to parse numbers
          var num = Number(value)
          if (!Number.isNaN(num) && value !== '') {
            value = num
          }

          currentSection[key] = value
        }
      }
    }
  }

  return result
}

function generateToml(config) {
  var lines = []
  var llm = config.llm || {}

  lines.push('api_key = "' + (llm.api_key || '') + '"')
  lines.push('api_url = "' + (llm.api_endpoint || llm.api_url || '') + '"')
  lines.push('default_provider = "' + (llm.provider || 'openai') + '"')
  lines.push('default_model = "' + (llm.model || 'gpt-4o-mini') + '"')
  lines.push('default_temperature = ' + (llm.temperature !== undefined ? llm.temperature : 0.7))
  lines.push('provider_timeout_secs = ' + (llm.timeout_secs !== undefined ? llm.timeout_secs : 120))
  lines.push('')

  if (config.metadata) {
    lines.push('[metadata]')
    if (config.metadata.source) lines.push('source = "' + config.metadata.source + '"')
    if (config.metadata.hub_url) lines.push('hub_url = "' + config.metadata.hub_url + '"')
    if (config.metadata.updated_at !== undefined) lines.push('updated_at = ' + config.metadata.updated_at)
    lines.push('')
  }

  lines.push('[gateway]')
  lines.push('require_pairing = false')
  lines.push('')

  return lines.join('\n')
}

function parseToml(content) {
  var result = {}
  var currentSection = result
  var lines = content.split('\n')

  for (var i = 0; i < lines.length; i++) {
    var line = lines[i].trim()

    if (line && !line.startsWith('#')) {
      if (line.startsWith('[') && line.endsWith(']')) {
        var sectionName = line.slice(1, -1)
        var parts = sectionName.split('.')
        currentSection = result
        for (var j = 0; j < parts.length; j++) {
          var part = parts[j]
          if (!currentSection[part]) currentSection[part] = {}
          currentSection = currentSection[part]
        }
      } else {
        var eqIndex = line.indexOf('=')
        if (eqIndex > 0) {
          var key = line.slice(0, eqIndex).trim()
          var value = line.slice(eqIndex + 1).trim()

          if (value.startsWith('"') && value.endsWith('"')) {
            value = value.slice(1, -1)
          }

          var num = Number(value)
          if (!Number.isNaN(num) && value !== '') {
            value = num
          }

          currentSection[key] = value
        }
      }
    }
  }

  if (!result.llm) result.llm = {}
  if (result.api_key !== undefined) result.llm.api_key = result.api_key
  if (result.api_url !== undefined) result.llm.api_endpoint = result.api_url
  if (result.default_provider !== undefined) result.llm.provider = result.default_provider
  if (result.default_model !== undefined) result.llm.model = result.default_model
  if (result.default_temperature !== undefined) result.llm.temperature = result.default_temperature
  if (result.provider_timeout_secs !== undefined) result.llm.timeout_secs = result.provider_timeout_secs

  return result
}

// Save global configuration to ~/.clawparty/global-config.toml
function saveGlobalConfig(llmConfig, hubUrl) {
  var globalConfigPath = os.path.join(rootDir, 'global-config.toml')

  var config = {
    llm: {
      provider: llmConfig.provider || 'openai',
      api_endpoint: llmConfig.api_endpoint || '',
      api_key: llmConfig.api_key || '',
      model: llmConfig.model || 'gpt-4o-mini',
      temperature: llmConfig.temperature !== undefined ? llmConfig.temperature : 0.7,
      timeout_secs: llmConfig.timeout_secs !== undefined ? llmConfig.timeout_secs : 120
    },
    metadata: {
      source: 'hub',
      hub_url: hubUrl || '',
      updated_at: Math.floor(Date.now() / 1000)
    }
  }

  var content = generateToml(config)
  os.write(globalConfigPath, content)
  console.info('[config] Global config saved to:', globalConfigPath)

  return config
}

// Load global configuration from ~/.clawparty/global-config.toml
function loadGlobalConfig() {
  var globalConfigPath = os.path.join(rootDir, 'global-config.toml')

  try {
    var content = os.read(globalConfigPath).toString()
    var config = parseToml(content)
    console.log('[config] Loaded global config from:', globalConfigPath)
    return config
  } catch (e) {
    console.log('[config] No global config found or failed to load:', e)
    return null
  }
}

// Merge global config with agent-specific config (agent config takes priority)
function mergeConfig(globalConfig, agentConfig) {
  if (!globalConfig || !globalConfig.llm) {
    return agentConfig
  }

  if (!agentConfig) {
    return {
      provider: globalConfig.llm.provider,
      api_endpoint: globalConfig.llm.api_endpoint,
      api_key: globalConfig.llm.api_key,
      model: globalConfig.llm.model,
      temperature: globalConfig.llm.temperature,
      timeout_secs: globalConfig.llm.timeout_secs
    }
  }

  // Agent config takes priority
  return {
    provider: agentConfig.provider || globalConfig.llm.provider,
    api_endpoint: agentConfig.api_endpoint || globalConfig.llm.api_endpoint,
    api_key: agentConfig.api_key || globalConfig.llm.api_key,
    model: agentConfig.model || globalConfig.llm.model,
    temperature: agentConfig.temperature !== undefined ? agentConfig.temperature : globalConfig.llm.temperature,
    timeout_secs: agentConfig.timeout_secs !== undefined ? agentConfig.timeout_secs : globalConfig.llm.timeout_secs
  }
}

export default {
  init,
  saveGlobalConfig,
  loadGlobalConfig,
  mergeConfig,
  parseToml,
  generateToml
}

