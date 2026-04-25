// Fallback: deterministic emoji from any name via hash
const EMOJIS = [
  '🐱', '🐶', '🦊', '🐼', '🐨', '🐯', '🦁', '🐷', '🐸', '🐵',
  '🐙', '🦋', '🦄', '🐲', '🐳', '🦅', '🦉', '🐢', '🦕', '🐘',
  '🔥', '⚡️', '🌟', '✨', '🌈', '🌊', '🌸', '🍀', '🌙', '☀️',
  '🎸', '🎨', '🎭', '🎬', '🎪', '🎲', '🎯', '🧩', '🎱', '🎳',
  '🤖', '👾', '🚀', '🛸', '🛰️', '🔮', '💎', '🧬', '🕹️', '🦾',
  '🍎', '🍊', '🍋', '🍇', '🍓', '🍑', '🍍', '🥝', '🥑', '🥕',
  '⚓️', '🗿', '🏰', '🗽', '🎡', '🌋', '🏔️', '🏕️', '🏝️', '🏜️',
  '🦀', '🦞', '🦐', '🦑', '🐡', '🐠', '🐟', '🐬', '🐋', '🦈',
  '🦓', '🦌', '🐕', '🐈', '🐇', '🐿️', '🦔', '🐁', '🐀', '🦦',
]

export function getAgentEmoji(name) {
  if (!name) return '🤖'
  let hash = 0
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash)
  }
  return EMOJIS[Math.abs(hash) % EMOJIS.length]
}

// Semantic emoji mapping based on agent name/display_name
export function getSemanticEmoji(name) {
  if (!name) return '🤖'
  const lower = name.toLowerCase()

  if (lower.includes('导演') || lower.includes('director')) return '🎬'
  if (lower.includes('销售') || lower.includes('sales')) return '💰'
  if (lower.includes('病例') || lower.includes('medical')) return '🏥'
  if (lower.includes('检测') || lower.includes('test')) return '🔬'
  if (lower.includes('能耗') || lower.includes('energy')) return '⚡'
  if (lower.includes('产品') || lower.includes('cp') || lower.includes('gtm')) return '📦'
  if (lower.includes('dbs') || lower.includes('bank')) return '🏦'
  if (lower.includes('法律') || lower.includes('law')) return '⚖'
  if (lower.includes('安全') || lower.includes('security')) return '🛡️'
  if (lower.includes('前端') || lower.includes('frontend')) return '💻'
  if (lower.includes('后端') || lower.includes('backend')) return '⚙️'
  if (lower.includes('数据') || lower.includes('data')) return '📊'
  if (lower.includes('运维') || lower.includes('ops')) return '📡'
  if (lower.includes('设计') || lower.includes('design')) return '🎨'
  if (lower.includes('翻译') || lower.includes('translator')) return '🌐'
  if (lower.includes('写作') || lower.includes('writer')) return '✍️'
  if (lower.includes('问答') || lower.includes('q&a')) return '❓'

  // Fallback to hash-based emoji
  return getAgentEmoji(name)
}
