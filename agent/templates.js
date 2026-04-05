var templateDir = ''
var sharedTemplateDir = ''
var dbApi = null

function init(rootDir, db) {
  var home = os.env['HOME'] || '/tmp'
  templateDir = os.path.join(home, '.agent-template')
  sharedTemplateDir = os.path.join(templateDir, '.shared')
  dbApi = db
}

function readTemplateFile(path) {
  try {
    var content = os.read(path)
    if (!content) return null
    return content.toString()
  } catch {
    return null
  }
}

function scanDir(dir) {
  var entries = []
  try {
    entries = os.readDir(dir)
  } catch {
    return []
  }
  return entries.map(function (name) {
    var hasTrailingSlash = name.length > 0 && name.charAt(name.length - 1) === '/'
    if (hasTrailingSlash) {
      return name.substring(0, name.length - 1)
    }
    return name
  }).filter(function (name) {
    if (name.length === 0 || name[0] === '.') return false
    try {
      var st = os.stat(os.path.join(dir, name))
      return st && st.isDirectory()
    } catch {
      return false
    }
  })
}

var industryEmojis = {
  'automation': '[auto]',
  'business': '[biz]',
  'compliance': '[law]',
  'creative': '[art]',
  'customer-success': '[cs]',
  'data': '[data]',
  'development': '[dev]',
  'devops': '[ops]',
  'e-commerce': '[shop]',
  'ecommerce': '[shop]',
  'education': '[edu]',
  'finance': '[fin]',
  'freelance': '[free]',
  'healthcare': '[med]',
  'hr': '[hr]',
  'legal': '[legal]',
  'marketing': '[mkt]',
  'personal': '[me]',
  'productivity': '[prod]',
  'real-estate': '[re]',
  'saas': '[saas]',
  'security': '[sec]',
  'supply-chain': '[sc]',
  'voice': '[voice]',
}

var industryNames = {
  'automation': '自动化',
  'business': '商业',
  'compliance': '合规',
  'creative': '创意',
  'customer-success': '客户成功',
  'data': '数据',
  'development': '开发',
  'devops': '运维',
  'e-commerce': '电商',
  'ecommerce': '电商',
  'education': '教育',
  'finance': '金融',
  'freelance': '自由职业',
  'healthcare': '医疗健康',
  'hr': '人力资源',
  'legal': '法律',
  'marketing': '营销',
  'personal': '个人',
  'productivity': '效率',
  'real-estate': '房地产',
  'saas': 'SaaS',
  'security': '安全',
  'supply-chain': '供应链',
  'voice': '语音',
}

var agentNames = {
  'discord-business': 'Discord 商业助手',
  'flight-scraper': '机票搜索助手',
  'job-applicant': '求职助手',
  'morning-briefing': '晨间简报',
  'negotiation-agent': '谈判助手',
  'overnight-coder': '夜间编程助手',
  'churn-predictor': '流失预测',
  'competitor-pricing': '竞品定价分析',
  'customer-support': '客户支持',
  'deal-forecaster': '成交预测',
  'erp-admin': 'ERP 管理员',
  'invoice-tracker': '发票跟踪',
  'lead-gen': '线索获取',
  'meeting-scheduler': '会议调度',
  'objection-handler': '异议处理',
  'personal-crm': '个人 CRM',
  'radar': '雷达监控',
  'sales-assistant': '销售助手',
  'sdr-outbound': '外呼 SDR',
  'whatsapp-business': 'WhatsApp 商业助手',
  'ai-policy-writer': 'AI 政策撰写',
  'gdpr-auditor': 'GDPR 审计',
  'risk-assessor': '风险评估',
  'soc2-preparer': 'SOC2 准备',
  'ad-copywriter': '广告文案',
  'audio-producer': '音频制作',
  'brand-designer': '品牌设计',
  'copywriter': '文案写手',
  'podcast-producer': '播客制作',
  'proofreader': '校对助手',
  'storyboard-writer': '故事板撰写',
  'thumbnail-designer': '缩略图设计',
  'ux-researcher': 'UX 研究员',
  'video-scripter': '视频脚本',
  'nps-followup': 'NPS 跟进',
  'onboarding-guide': '入职引导',
  'anomaly-detector': '异常检测',
  'dashboard-builder': '仪表盘构建',
  'data-cleaner': '数据清洗',
  'data-entry': '数据录入',
  'etl-pipeline': 'ETL 流水线',
  'report-generator': '报告生成',
  'sql-assistant': 'SQL 助手',
  'survey-analyzer': '问卷分析',
  'transcription': '语音转录',
  'api-documentation': 'API 文档',
  'api-tester': 'API 测试',
  'blockchain-analyst': '区块链分析',
  'bug-hunter': '漏洞猎人',
  'changelog': '更新日志',
  'code-reviewer': '代码审查',
  'dependency-scanner': '依赖扫描',
  'docs-writer': '文档写作',
  'ecommerce-dev': '电商开发',
  'game-designer': '游戏设计',
  'github-issue-triager': 'Issue 分类',
  'github-pr-reviewer': 'PR 审查',
  'migration-helper': '迁移助手',
  'pr-merger': 'PR 合并',
  'qa-tester': 'QA 测试',
  'schema-designer': 'Schema 设计',
  'script-builder': '脚本构建',
  'test-writer': '测试编写',
  'capacity-planner': '容量规划',
  'cost-optimizer': '成本优化',
  'deploy-guardian': '部署守卫',
  'incident-responder': '事故响应',
  'infra-monitor': '基础设施监控',
  'log-analyzer': '日志分析',
  'raspberry-pi': '树莓派助手',
  'runbook-writer': 'Runbook 撰写',
  'self-healing-server': '自愈服务器',
  'sla-monitor': 'SLA 监控',
  'abandoned-cart': '购物车挽回',
  'dropshipping-researcher': '代发货研究',
  'inventory-tracker': '库存跟踪',
  'price-monitor': '价格监控',
  'pricing-optimizer': '定价优化',
  'product-lister': '商品上架',
  'review-responder': '评价回复',
  'curriculum-designer': '课程设计',
  'essay-grader': '作文批改',
  'flashcard-generator': '闪卡生成',
  'language-tutor': '语言家教',
  'quiz-maker': '题目生成',
  'research-assistant': '研究助手',
  'study-planner': '学习规划',
  'tutor': '辅导老师',
  'accounts-payable': '应付账款',
  'copy-trader': '跟单交易',
  'expense-tracker': '费用跟踪',
  'financial-forecaster': '财务预测',
  'fraud-detector': '欺诈检测',
  'invoice-manager': '发票管理',
  'portfolio-rebalancer': '投资组合再平衡',
  'revenue-analyst': '营收分析',
  'tax-preparer': '税务准备',
  'trading-bot': '交易机器人',
  'client-manager': '客户管理',
  'proposal-writer': '提案撰写',
  'time-tracker': '时间跟踪',
  'upwork-proposal': 'Upwork 提案',
  'clinical-notes': '临床笔记',
  'meal-planner': '饮食规划',
  'medication-checker': '用药检查',
  'patient-intake': '患者接诊',
  'symptom-triage': '症状分诊',
  'wellness-coach': '健康教练',
  'workout-tracker': '训练跟踪',
  'benefits-advisor': '福利顾问',
  'compensation-benchmarker': '薪酬基准',
  'exit-interview': '离职访谈',
  'onboarding': '入职流程',
  'performance-reviewer': '绩效评估',
  'recruiter': '招聘助手',
  'resume-optimizer': '简历优化',
  'resume-screener': '简历筛选',
  'compliance-checker': '合规检查',
  'contract-reviewer': '合同审查',
  'legal-brief-writer': '法律简报',
  'nda-generator': 'NDA 生成',
  'patent-analyzer': '专利分析',
  'policy-writer': '政策撰写',
  'ab-test-analyzer': 'A/B 测试分析',
  'book-writer': '书籍写作',
  'brand-monitor': '品牌监控',
  'cold-outreach': '冷门推广',
  'competitor-watch': '竞品监控',
  'content-repurposer': '内容再利用',
  'echo': 'Echo 助手',
  'email-sequence': '邮件序列',
  'geo-agent': '地理定向助手',
  'hackernews-agent': 'HN 助手',
  'influencer-finder': 'KOL 发现',
  'linkedin-content': 'LinkedIn 内容',
  'localization': '本地化',
  'multi-account-social': '多账号社媒',
  'news-curator': '新闻策划',
  'newsletter': '新闻通讯',
  'reddit-scout': 'Reddit 侦察',
  'seo-writer': 'SEO 写作',
  'social-media': '社交媒体',
  'telemarketer': '电话营销',
  'tiktok-repurposer': 'TikTok 再制作',
  'ugc-video': 'UGC 视频',
  'x-twitter-growth': 'X/Twitter 增长',
  'youtube-seo': 'YouTube SEO',
  'community-manager': '社区管理',
  'growth-agent': '增长助手',
  'scout': '侦察助手',
  'daily-planner': '每日规划',
  'family-coordinator': '家庭协调',
  'fitness-coach': '健身教练',
  'home-automation': '家居自动化',
  'journal-prompter': '日记提示',
  'reading-digest': '阅读摘要',
  'travel-planner': '旅行规划',
  'daily-standup': '每日站会',
  'focus-timer': '专注计时',
  'habit-tracker': '习惯跟踪',
  'inbox-zero': '收件箱清零',
  'meeting-notes': '会议记录',
  'meeting-transcriber': '会议转录',
  'metrics': '指标分析',
  'notion-organizer': 'Notion 整理',
  'orion': 'Orion 助手',
  'commercial-re': '商业地产',
  'lead-qualifier': '线索资质',
  'listing-scout': '房源侦察',
  'market-analyzer': '市场分析',
  'property-video': '房产视频',
  'churn-prevention': '流失预防',
  'feature-request': '功能请求',
  'onboarding-flow': '入职流程',
  'product-scrum': '产品 Scrum',
  'release-notes': '发布说明',
  'usage-analytics': '使用分析',
  'access-auditor': '访问审计',
  'incident-logger': '事故记录',
  'phishing-detector': '钓鱼检测',
  'security-hardener': '安全加固',
  'threat-monitor': '威胁监控',
  'vuln-scanner': '漏洞扫描',
  'inventory-forecaster': '库存预测',
  'route-optimizer': '路线优化',
  'vendor-evaluator': '供应商评估',
  'interview-bot': '面试机器人',
  'phone-receptionist': '电话前台',
  'voicemail-transcriber': '语音邮件转录',
}

var roleTranslations = {
  // Roles
  'Personal schedule optimizer and daily planner': '个人日程优化与每日规划助手',
  'Personal fitness coach and nutrition advisor': '个人健身教练与营养顾问',
  'Content curator, article summarizer, knowledge organizer': '内容策展人、文章摘要与知识整理助手',
  'Smart home controller powered by OpenClaw': '智能家居控制助手',
  'Daily journaling prompt generation agent': '每日日记提示生成助手',
  'Itinerary planning and travel logistics agent': '行程规划与旅行后勤助手',
  'Family scheduling and coordination assistant': '家庭日程与协调助手',
  'Personal CRM and relationship manager': '个人CRM与关系管理助手',
  'AI code reviewer and quality gatekeeper': 'AI代码审查与质量把关助手',
  // Personalities
  'Calm, organized, supportive': '冷静、有组织、支持性',
  'Motivational, practical, adaptive': '激励人心、实用、适应性强',
  'Curious, thorough, concise': '好奇、细致、简洁',
}

function slugToName(slug) {
  if (agentNames[slug]) return agentNames[slug]
  var parts = slug.split('-')
  var result = ''
  parts.forEach(function (part, i) {
    if (part.length > 0) {
      if (i > 0) result += ' '
      result += part.charAt(0).toUpperCase() + part.substring(1)
    }
  })
  return result
}

function translateToChinese(text) {
  if (!text) return ''
  // Skip translation if text already contains Chinese characters
  var hasChinese = false
  for (var i = 0; i < text.length; i++) {
    var code = text.charCodeAt(i)
    if (code >= 0x4e00 && code <= 0x9fff) {
      hasChinese = true
      break
    }
  }
  if (hasChinese) return text
  if (roleTranslations[text]) return roleTranslations[text]
  var lower = text.toLowerCase()
  if (lower.indexOf('planner') !== -1 || lower.indexOf('schedule') !== -1) return '规划助手'
  if (lower.indexOf('reviewer') !== -1 || lower.indexOf('review') !== -1) return '审查助手'
  if (lower.indexOf('assistant') !== -1 || lower.indexOf('helper') !== -1) return '助手'
  if (lower.indexOf('coach') !== -1) return '教练'
  if (lower.indexOf('agent') !== -1) return '智能代理'
  if (lower.indexOf('calm') !== -1) return '冷静'
  if (lower.indexOf('organized') !== -1) return '有条理'
  if (lower.indexOf('supportive') !== -1) return '支持性'
  if (lower.indexOf('practical') !== -1) return '实用'
  if (lower.indexOf('adaptive') !== -1) return '适应性强'
  if (lower.indexOf('curious') !== -1) return '好奇'
  if (lower.indexOf('thorough') !== -1) return '细致'
  if (lower.indexOf('concise') !== -1) return '简洁'
  if (text.length > 40) return text.substring(0, 37) + '...'
  return text
}

function generateDescription(slug, soulContent, isZh) {
  if (!soulContent) {
    return ''
  }
  console.log('[DEBUG genDesc] slug:', slug, 'isZh:', isZh, 'content length:', soulContent.length)
  var lines = soulContent.split('\n')
  var role = ''
  var personality = ''
  
  if (isZh) {
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trim()
      if (line.indexOf('role:') === 0) {
        var idx = line.indexOf('"')
        if (idx >= 0) {
          var endIdx = line.indexOf('"', idx + 1)
          if (endIdx > idx) {
            role = line.substring(idx + 1, endIdx)
          }
        }
      }
      if (line.indexOf('## 性格') >= 0 && i + 1 < lines.length) {
        personality = lines[i + 1].trim()
        if (personality.length > 50) {
          personality = personality.substring(0, 47) + '...'
        }
      }
    }
  } else {
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trim()
      if (line.indexOf('- **角色：') >= 0 || line.indexOf('- **角色:**') >= 0) {
        var idx = line.indexOf('：**')
        if (idx >= 0) {
          role = line.substring(idx + 3).trim()
        }
      }
      if (line.indexOf('- **个性：') >= 0 || line.indexOf('- **个性:**') >= 0) {
        var idx = line.indexOf('：**')
        if (idx >= 0) {
          personality = line.substring(idx + 3).trim()
        }
      }
    }
  }
  
  var desc = ''
  if (role) desc += role
  if (personality) {
    if (desc) desc += '，'
    desc += personality
  }
  console.log('[DEBUG genDesc] result for', slug, ': role=', role, ', personality=', personality, ', desc=', desc)
  return desc
}

function scanTemplatesInDir(baseDir, source) {
  var industries = []
  var dirs = scanDir(baseDir)
  dirs.forEach(function (slug) {
    var industryPath = os.path.join(baseDir, slug)
    var agentDirs = scanDir(industryPath)
    if (agentDirs.length === 0) return
    var agents = []
    agentDirs.forEach(function (agentSlug) {
      var agentPath = os.path.join(industryPath, agentSlug)
      var soulZhPath = os.path.join(agentPath, 'SOUL-zh.md')
      var soulPath = os.path.join(agentPath, 'SOUL.md')
      var readmePath = os.path.join(agentPath, 'README.md')
      var soulZhContent = readTemplateFile(soulZhPath)
      var soulContent = soulZhContent || readTemplateFile(soulPath)
      var readmeContent = readTemplateFile(readmePath)
      if (!soulContent && !readmeContent) return
      var name = slugToName(agentSlug)
      var isZh = soulZhContent ? true : false
      var description = generateDescription(agentSlug, soulContent, isZh)
      agents.push({
        name: name,
        slug: agentSlug,
        emoji: '',
        description: description,
        systemPrompt: soulContent || '',
        tools: [],
        model: '',
        temperature: 0.7,
        version: '',
        installed: false,
      })
    })
    if (agents.length === 0) return
    var industryName = industryNames[slug] || slug.charAt(0).toUpperCase() + slug.substring(1)
    var industryEmoji = industryEmojis[slug] || ''
    industries.push({
      name: industryName,
      slug: slug,
      emoji: industryEmoji,
      description: '',
      color: '',
      version: '',
      author: '',
      source: source,
      agents: agents,
    })
  })
  return industries
}

function scanLocalTemplates() {
  return scanTemplatesInDir(templateDir, 'local')
}

function scanSharedTemplates() {
  return scanTemplatesInDir(sharedTemplateDir, 'shared')
}

function installTemplate(industry, agentSlug, source, soulContent) {
  console.log('[DEBUG installTemplate] industry:', industry, ', agent:', agentSlug, ', source:', source, ', soulContent length:', soulContent ? soulContent.length : 0)
  var baseDir = source === 'shared' ? sharedTemplateDir : templateDir
  var agentPath = os.path.join(baseDir, industry, agentSlug)
  var soulZhPath = os.path.join(agentPath, 'SOUL-zh.md')
  var soulPath = os.path.join(agentPath, 'SOUL.md')
  console.log('[DEBUG installTemplate] baseDir:', baseDir, ', industry:', industry, ', agentSlug:', agentSlug)
  console.log('[DEBUG installTemplate] checking soulZhPath:', soulZhPath)
  var fileContent = readTemplateFile(soulZhPath) || readTemplateFile(soulPath)
  var content = soulContent || fileContent
  console.log('[DEBUG installTemplate] content:', content ? 'found' : 'not found')
  if (!content) {
    return { success: false, message: 'Template not found: ' + soulZhPath + ' or ' + soulPath }
  }
  var openclaws = dbApi.allOpenclaws()
  var exists = false
  openclaws.forEach(function (oc) {
    if (oc.name === agentSlug) exists = true
  })
  if (exists) return { success: false, message: 'Agent already exists' }
  dbApi.setOpenclaw(agentSlug, {
    type: 'openclaw',
    api_url: 'http://127.0.0.1:6789',
    token: 'join-party',
    soul_content: content,
  })
  var name = slugToName(agentSlug)
  return {
    success: true,
    agentId: agentSlug,
    agentName: name,
    message: "Agent '" + name + "' installed",
  }
}

export default {
  init,
  scanLocalTemplates,
  scanSharedTemplates,
  installTemplate,
}
