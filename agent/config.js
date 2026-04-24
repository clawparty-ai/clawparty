// Global configuration management for ClawParty
// Handles saving and loading of global LLM configuration from ~/.clawparty/global-config.toml

var rootDir = ''

function init(dirname) {
  rootDir = dirname
}

// Parse TOML content into JavaScript object
function parseToml(content) {
  var result = {}
  var currentSection = result
  var lines = content.split('\n')

  for (var i = 0; i < lines.length; i++) {
    var line = lines[i].trim()

    // Skip comments and empty lines
    if (!line || line.startsWith('#')) continue

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
      continue
    }

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
      if (!isNaN(value) && value !== '') {
        value = parseFloat(value)
      }

      currentSection[key] = value
    }
  }

  return result
}

// Generate TOML content from JavaScript object
function generateToml(config) {
  var lines = []

  // Generate [llm] section
  if (config.llm) {
    lines.push('[llm]')
    if (config.llm.provider) lines.push('provider = "' + config.llm.provider + '"')
    if (config.llm.api_endpoint) lines.push('api_endpoint = "' + config.llm.api_endpoint + '"')
    if (config.llm.api_key) lines.push('api_key = "' + config.llm.api_key + '"')
    if (config.llm.model) lines.push('model = "' + config.llm.model + '"')
    if (config.llm.temperature !== undefined) lines.push('temperature = ' + config.llm.temperature)
    if (config.llm.timeout_secs !== undefined) lines.push('timeout_secs = ' + config.llm.timeout_secs)
    lines.push('')
  }

  // Generate [metadata] section
  if (config.metadata) {
    lines.push('[metadata]')
    if (config.metadata.source) lines.push('source = "' + config.metadata.source + '"')
    if (config.metadata.hub_url) lines.push('hub_url = "' + config.metadata.hub_url + '"')
    if (config.metadata.updated_at !== undefined) lines.push('updated_at = ' + config.metadata.updated_at)
    lines.push('')
  }

  return lines.join('\n')
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

