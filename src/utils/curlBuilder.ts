import type { ParamItem } from '../types'

export interface CurlBuildParams {
  method: string
  url: string
  queryParams?: ParamItem[]
  headers?: ParamItem[]
  bodyType?: string
  body?: string
  authType?: string
  authConfig?: string
}

/**
 * 将请求参数构建为 cURL 命令字符串。
 * 生成规范：
 *  - 启用的 query params 追加到 URL
 *  - 启用的 headers 以 -H 传递
 *  - auth 自动注入对应 header（不重复手动 header）
 *  - body 根据类型选择 --data-raw 或 --data-urlencode
 */
export function buildCurl(params: CurlBuildParams): string {
  const parts: string[] = ['curl']

  // Method（GET 不显示，其余显示）
  if (params.method && params.method.toUpperCase() !== 'GET') {
    parts.push(`-X ${params.method.toUpperCase()}`)
  }

  // 构建 URL（含启用的 query params）
  let finalUrl = params.url || ''
  const enabledQuery = (params.queryParams ?? []).filter(p => p.enabled && p.key)
  if (enabledQuery.length > 0) {
    const queryStr = enabledQuery
      .map(p => `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`)
      .join('&')
    finalUrl += (finalUrl.includes('?') ? '&' : '?') + queryStr
  }
  parts.push(`"${finalUrl}"`)

  // Auth header（自动注入，避免与手动 header 重复）
  const authType = params.authType ?? 'none'
  const authCfg = (() => {
    try { return JSON.parse(params.authConfig ?? '{}') } catch { return {} }
  })()

  let authHeaderKey = ''
  let authHeaderValue = ''
  if (authType === 'bearer' && authCfg.token) {
    authHeaderKey = 'Authorization'
    authHeaderValue = `Bearer ${authCfg.token}`
  } else if (authType === 'basic' && authCfg.username) {
    // btoa 仅支持 Latin-1，需先 UTF-8 编码以支持中文/非 ASCII 字符
    const raw = `${authCfg.username}:${authCfg.password ?? ''}`
    const encoded = btoa(unescape(encodeURIComponent(raw)))
    authHeaderKey = 'Authorization'
    authHeaderValue = `Basic ${encoded}`
  } else if (authType === 'api_key' && authCfg.in === 'header' && authCfg.key) {
    authHeaderKey = authCfg.key
    authHeaderValue = authCfg.value ?? ''
  }

  // 输出启用的 headers（检查是否已包含 auth header，避免重复）
  const enabledHeaders = (params.headers ?? []).filter(h => h.enabled && h.key)
  const headerKeys = new Set(enabledHeaders.map(h => h.key.toLowerCase()))

  if (authHeaderKey && !headerKeys.has(authHeaderKey.toLowerCase())) {
    parts.push(`-H "${escapeShell(authHeaderKey)}: ${escapeShell(authHeaderValue)}"`)
  }

  for (const h of enabledHeaders) {
    parts.push(`-H "${escapeShell(h.key)}: ${escapeShell(h.value)}"`)
  }

  // Body
  const bodyType = params.bodyType ?? 'none'
  const body = params.body ?? ''

  if (bodyType === 'raw_json' && body) {
    parts.push(`-H "Content-Type: application/json"`)
    parts.push(`--data-raw '${body.replace(/'/g, "'\\''")}'`)
  } else if (bodyType === 'raw_text' && body) {
    parts.push(`--data-raw '${body.replace(/'/g, "'\\''")}'`)
  } else if (bodyType === 'form_urlencoded' && body) {
    // body 为 "key=value&..." 格式
    parts.push(`-H "Content-Type: application/x-www-form-urlencoded"`)
    parts.push(`--data-urlencode '${body.replace(/'/g, "'\\''")}'`)
  } else if (bodyType === 'form_data' && body) {
    // body 为 JSON 数组 [{key, value, enabled}]
    try {
      const fields: ParamItem[] = JSON.parse(body)
      for (const f of fields) {
        if (f.enabled && f.key) {
          parts.push(`-F "${escapeShell(f.key)}=${escapeShell(f.value)}"`)
        }
      }
    } catch {
      // 解析失败时静默跳过
    }
  }

  return parts.join(' \\\n  ')
}

/** 转义双引号内的特殊字符（shell 安全） */
function escapeShell(s: string): string {
  return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')
}
