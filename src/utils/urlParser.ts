import type { ParsedUrl } from '../types'

// ──────────────────────────────────────────────────────────────
// 路径参数识别规则
// ──────────────────────────────────────────────────────────────

/**
 * 旧识别规则 —— 只用于 `displayName` 生成，保持接口默认名称向下兼容。
 * 历史上生成的接口名基于此规则。放宽 Path Params 识别时不动 displayName，
 * 避免破坏已有接口名或改变"从 URL 新建接口"时的默认命名行为。
 */
const LEGACY_PATH_PARAM_PATTERNS = [
  /^\d+$/,                                                    // 纯数字：123, 1676657
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i,  // UUID
  /^:.+/,                                                     // :id 格式
]

function isLegacyPathParam(segment: string): boolean {
  return LEGACY_PATH_PARAM_PATTERNS.some(p => p.test(segment))
}

/**
 * 扩展识别规则 —— 用于 `pathParams` 列表，决定 Path Params 面板里展示哪些段。
 * 相比旧规则补充：
 *  1. 模板占位符 `{xxx}` 花括号形态
 *  2. Mongo ObjectId（24 位十六进制）
 *  3. 含数字混合段（abc123 / usr_9f2a / ORDER-2025-001）
 *
 * 排除：`v\d+`（API 版本号，如 v1/v2/V3），避免误识别。
 */
const TEMPLATE_PARAM_PATTERNS = [
  /^:.+/,                       // :id
  /^\{[^{}]+\}$/,               // {id} / {userId}
]

const V_VERSION_PATTERN = /^v\d+$/i

const LITERAL_PARAM_PATTERNS = [
  /^\d+$/,                                                                 // 纯数字
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i,       // UUID
  /^[0-9a-f]{24}$/i,                                                       // Mongo ObjectId
  /^[A-Za-z][\w.\-]*[0-9][\w.\-]*$/,                                       // 含至少一个数字的混合段
]

/**
 * 判断一个段是参数型 (template) 还是字面量参数 (literal) 还是静态路径。
 * 返回 null 表示静态路径（不进 Path Params 面板）。
 */
function classifySegment(segment: string): 'template' | 'literal' | null {
  if (TEMPLATE_PARAM_PATTERNS.some(p => p.test(segment))) return 'template'
  if (V_VERSION_PATTERN.test(segment)) return null           // v1/v2 视为静态
  if (LITERAL_PARAM_PATTERNS.some(p => p.test(segment))) return 'literal'
  return null
}

/**
 * 在 raw URL 字符串层面提取 path segments。
 *
 * 为什么不用 `new URL(...).pathname.split('/')`：
 *   - WHATWG URL 规范把 `{` `}` 等字符百分号编码成 `%7B` `%7D`
 *   - 导致用户输入 `/users/{id}` 后 pathname 变为 `/users/%7Bid%7D`
 *   - 正则 `/^\{[^{}]+\}$/` 对 `%7Bid%7D` 不匹配 → 占位符永远识别不到
 *
 * 本函数直接在用户输入字符串上分段，保持原始字面形态：
 *   - 若 raw 以 `http(s)://` 开头，剥掉协议 + host
 *   - 剥掉 query / hash
 *   - 按 `/` 切分并过滤空段
 *
 * 与 MainPanel `replacePathSegmentInUrl` 里 `url.value.split('/')` 的分段规则
 * 一致，便于通过 segment 字面量精确反向定位。
 */
function extractRawPathSegments(rawUrl: string): string[] {
  let path = rawUrl
  const protoMatch = path.match(/^https?:\/\/[^/]+/i)
  if (protoMatch) path = path.slice(protoMatch[0].length)
  const qsIdx = path.search(/[?#]/)
  if (qsIdx >= 0) path = path.substring(0, qsIdx)
  return path.split('/').filter(Boolean)
}

/**
 * 提取 URL 的协议+host 前缀（如 "https://api.example.com:8080"），不含末尾斜杠。
 * 用户输入相对路径时返回空字符串。
 *
 * 与 extractRawPathSegments 共用同一套规则（regex `/^https?:\/\/[^/]+/i`），
 * 保证两者拼回去后等于原始 URL 的 host+path 部分。
 */
function extractRawOrigin(rawUrl: string): string {
  const protoMatch = rawUrl.match(/^https?:\/\/[^/]+/i)
  return protoMatch ? protoMatch[0] : ''
}

/**
 * 解析 URL，提取路径参数和 query 参数，生成接口默认名称
 *
 * 例：POST http://localhost:8088/apm/intl/download/1676657?androidVersion=14
 * → displayName: "/apm/intl/download"
 * → pathTemplate: "/apm/intl/download/{id}"
 * → pathParams: [{key:"{id}", value:"1676657"}]
 * → queryParams: [{key:"androidVersion", value:"14"}]
 */
export function parseUrl(rawUrl: string, _method?: string): ParsedUrl {
  let url: URL
  try {
    // 处理没有协议头的 URL
    const normalized = rawUrl.startsWith('http') ? rawUrl : `http://placeholder/${rawUrl}`
    url = new URL(normalized)
  } catch {
    return {
      displayName: rawUrl,
      pathTemplate: rawUrl,
      pathParams: [],
      queryParams: [],
    }
  }

  // 解析路径段
  //
  // 关键：不使用 url.pathname —— WHATWG URL 规范会把 `{` / `}` 百分号编码成
  // `%7B` / `%7D`，导致 `{id}` 形态的占位符在分类时无法识别。
  // 改为在"原始字符串"层面做 path 切分：
  //   - 去掉协议 + host 前缀（若有）
  //   - 去掉 query / hash 尾巴
  //   - 按 '/' 切分
  // 这样段保持用户输入的字面形态（`{id}` / `:id` / `43251`），与后续
  // MainPanel.replacePathSegmentInUrl 里对 `url.value.split('/')` 的定位
  // 用同一种分段，避免 `new URL` 编码后段文本不一致。
  //
  // 维护两套段分类：
  //  - legacyIsParam[i]：旧规则判定，决定 displayName 去哪些段
  //  - paramMode[i]：新规则判定 (null | 'template' | 'literal')，决定 pathParams 面板列表
  // 两套分类分离，避免放宽识别规则影响"从 URL 新建接口"时的默认命名。
  const segments = extractRawPathSegments(rawUrl)
  const cleanSegments: string[] = []
  const pathParams: Array<{ key: string; value: string; segment: string; mode: 'template' | 'literal' }> = []
  const usedKeys = new Set<string>()

  // 派生 key 名工具：复用原命名策略（:id 用原名，UUID 用 {uuid}，其他用 {id}/{id2}...）
  function deriveKey(seg: string, mode: 'template' | 'literal'): string {
    let key: string

    if (mode === 'template') {
      // :id → {id}；{userId} → {userId}
      if (seg.startsWith(':')) key = `{${seg.slice(1)}}`
      else key = seg          // 已经是 {xxx} 形态
    } else if (/^[0-9a-f]{8}-[0-9a-f]{4}-/i.test(seg)) {
      // UUID
      key = '{uuid}'
      let n = 2
      while (usedKeys.has(key)) key = `{uuid${n++}}`
    } else if (/^[0-9a-f]{24}$/i.test(seg)) {
      // Mongo ObjectId
      key = '{oid}'
      let n = 2
      while (usedKeys.has(key)) key = `{oid${n++}}`
    } else {
      // 其他（纯数字 / 含数字混合） → {id}/{id2}/...
      key = '{id}'
      let n = 2
      while (usedKeys.has(key)) key = `{id${n++}}`
    }
    return key
  }

  for (const seg of segments) {
    const mode = classifySegment(seg)
    if (mode === null) {
      cleanSegments.push(seg)
      continue
    }

    const key = deriveKey(seg, mode)
    usedKeys.add(key)
    pathParams.push({
      key,
      // template 模式下用户还没填值，value 空；literal 模式下 value = 段字面量
      value: mode === 'template' ? '' : seg,
      segment: seg,
      mode,
    })
    cleanSegments.push(key)
  }

  // pathTemplate 保留用户输入的协议+host 前缀（如有），避免在"完整 URL + 无环境 base_url"
  // 场景下 buildUrl 输出相对路径，导致 reqwest::Url::parse 报 "relative URL without a base"。
  // 当用户输的是相对路径时 origin 为空字符串，此时 pathTemplate 仍以 '/' 开头（旧行为）。
  const origin = extractRawOrigin(rawUrl)
  const cleanPath = origin + '/' + cleanSegments.join('/')

  // displayName 专用：用旧规则单独算一遍（仅考虑旧规则识别的段，排除它们得到的路径）
  // 与 pathParams 使用的新规则分离，确保接口默认名称行为向下兼容。
  // displayName 不带协议+host —— 它是树节点的展示文本，简短的相对路径更易读。
  const displaySegments: string[] = []
  for (const seg of segments) {
    if (!isLegacyPathParam(seg)) displaySegments.push(seg)
  }
  const displayPath = '/' + displaySegments.join('/')

  // 解析 query 参数
  const queryParams: Array<{ key: string; value: string }> = []
  url.searchParams.forEach((value, key) => {
    queryParams.push({ key, value })
  })

  return {
    // displayName 只保留路径部分，不含 method
    // 因为树节点 label 会单独拼 "${r.method} ${r.name}"，避免重复出现两个 method
    displayName: displayPath,
    pathTemplate: cleanPath,
    pathParams,
    queryParams,
  }
}

/**
 * 将路径模板 + pathParams 值还原为真实 URL
 * "/apm/intl/download/{id}" + [{key:"{id}", value:"9999"}] → "/apm/intl/download/9999"
 */
export function buildUrl(template: string, pathParams: Array<{ key: string; value: string }>): string {
  let result = template
  for (const { key, value } of pathParams) {
    result = result.replace(key, value || key)
  }
  return result
}

/**
 * 将相对路径 URL 与环境 base_url 拼接为可直接调用的完整 URL。
 *
 * 规则：
 * - 入参已含 `http(s)://` 协议头 → 原样返回
 * - 无 base_url 或 base_url 为空 → 原样返回（发请求时可能报相对路径错误，但属于用户配置问题）
 * - 否则：去掉 base_url 末尾斜杠 + 补齐 raw 开头斜杠后拼接
 *
 * 使用场景：
 * - MainPanel `effectiveUrl`：发请求前拼接
 * - Sidebar `copyAsCurl`：复制 cURL 时拼接
 * 两处必须用同一套规则，避免"发请求能通但 cURL 拷贝出来没域名"这种不一致。
 */
export function resolveEffectiveUrl(raw: string, baseUrl: string | null | undefined): string {
  if (!raw) return raw
  if (/^https?:\/\//i.test(raw)) return raw
  if (!baseUrl) return raw
  const base = baseUrl.replace(/\/$/, '')
  const path = raw.startsWith('/') ? raw : `/${raw}`
  return `${base}${path}`
}

/**
 * 检测 URL 中是否存在未替换的 path param 占位符（:id / {id}）。
 * 用于 cURL 复制等"需要真实可运行 URL"的场景给用户预警。
 *
 * 仅识别 path 段中的占位符，避免误判 query/hash 中的冒号或花括号。
 */
export function hasUnresolvedPlaceholder(raw: string): boolean {
  if (!raw) return false
  // 截断 query / hash
  const qsIdx = raw.search(/[?#]/)
  const pathPart = qsIdx >= 0 ? raw.substring(0, qsIdx) : raw
  const segments = pathPart.split('/')
  for (const seg of segments) {
    if (!seg) continue
    if (/^:[^/]+/.test(seg)) return true          // :id 样式
    if (/^\{[^{}]+\}$/.test(seg)) return true     // {id} 花括号
  }
  return false
}
