import type { ParsedUrl } from '../types'

// 识别为路径参数的规则（设计文档 4.1.1）
const PATH_PARAM_PATTERNS = [
  /^\d+$/,                                                    // 纯数字：123, 1676657
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i,  // UUID
  /^:.+/,                                                     // :id 格式
]

function isPathParam(segment: string): boolean {
  return PATH_PARAM_PATTERNS.some(p => p.test(segment))
}

/**
 * 解析 URL，提取路径参数和 query 参数，生成接口默认名称
 *
 * 例：POST http://localhost:8088/apm/intl/download/1676657?androidVersion=14
 * → displayName: "POST /apm/intl/download"
 * → pathTemplate: "/apm/intl/download/{id}"
 * → pathParams: [{key:"{id}", value:"1676657"}]
 * → queryParams: [{key:"androidVersion", value:"14"}]
 */
export function parseUrl(rawUrl: string, method: string): ParsedUrl {
  let url: URL
  try {
    // 处理没有协议头的 URL
    const normalized = rawUrl.startsWith('http') ? rawUrl : `http://placeholder/${rawUrl}`
    url = new URL(normalized)
  } catch {
    return {
      displayName: `${method} ${rawUrl}`,
      pathTemplate: rawUrl,
      pathParams: [],
      queryParams: [],
    }
  }

  // 解析路径段
  const segments = url.pathname.split('/').filter(Boolean)
  const cleanSegments: string[] = []
  const pathParams: Array<{ key: string; value: string }> = []
  const usedKeys = new Set<string>()

  for (const seg of segments) {
    if (isPathParam(seg)) {
      let key = '{id}'

      // :id 格式用原始名称
      if (seg.startsWith(':')) {
        key = `{${seg.slice(1)}}`
      } else if (/^[0-9a-f]{8}-/i.test(seg)) {
        // UUID 用 {uuid}
        key = '{uuid}'
        let uuidSuffix = 2
        while (usedKeys.has(key)) key = `{uuid${uuidSuffix++}}`
      } else {
        // 纯数字：{id}, {id2}, {id3}...
        let suffix = 2
        while (usedKeys.has(key)) key = `{id${suffix++}}`
      }

      usedKeys.add(key)
      pathParams.push({ key, value: seg.startsWith(':') ? '' : seg })
      cleanSegments.push(key)
    } else {
      cleanSegments.push(seg)
    }
  }

  const cleanPath = '/' + cleanSegments.join('/')
  // displayName 中去掉参数占位符，只保留静态部分
  const displayPath = '/' + cleanSegments.filter(s => !s.startsWith('{')).join('/')

  // 解析 query 参数
  const queryParams: Array<{ key: string; value: string }> = []
  url.searchParams.forEach((value, key) => {
    queryParams.push({ key, value })
  })

  return {
    displayName: `${method} ${displayPath}`,
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
