import type { ParamItem } from '../types'

/**
 * 解析 KV 文本（key: value 或 key=value 格式）为 ParamItem[]
 * 跳过空行和纯注释行（# 开头）
 */
export function parseKvText(text: string): ParamItem[] {
  return text
    .split('\n')
    .map(line => line.trim())
    .filter(line => line && !line.startsWith('#'))
    .map(line => {
      // 先尝试 ": " 分隔（HTTP Header 风格），再尝试 "=" 分隔（query string 风格）
      const colonIdx = line.indexOf(': ')
      if (colonIdx !== -1) {
        return { key: line.slice(0, colonIdx).trim(), value: line.slice(colonIdx + 2).trim(), enabled: true }
      }
      const eqIdx = line.indexOf('=')
      if (eqIdx !== -1) {
        return { key: line.slice(0, eqIdx).trim(), value: line.slice(eqIdx + 1).trim(), enabled: true }
      }
      return { key: line, value: '', enabled: true }
    })
    .filter(item => item.key)  // 过滤掉空 key
}

/**
 * ParamItem[] → KV 文本（key: value，每行一条，disabled 行加 # 前缀）
 */
export function toKvText(params: ParamItem[]): string {
  return params
    .filter(p => p.key)
    .map(p => p.enabled ? `${p.key}: ${p.value}` : `# ${p.key}: ${p.value}`)
    .join('\n')
}

/**
 * JSON 对象字符串 → ParamItem[]（value 统一转 string）
 * 解析失败时返回空数组
 */
export function parseJsonToParams(jsonText: string): ParamItem[] {
  try {
    const obj = JSON.parse(jsonText)
    if (typeof obj !== 'object' || Array.isArray(obj) || obj === null) return []
    return Object.entries(obj).map(([key, value]) => ({
      key,
      value: String(value),
      enabled: true,
    }))
  } catch {
    return []
  }
}

/**
 * ParamItem[] → JSON 对象字符串（缩进 2，跳过空 key 和 disabled 行）
 */
export function toJsonText(params: ParamItem[]): string {
  const obj: Record<string, string> = {}
  params.filter(p => p.key && p.enabled).forEach(p => { obj[p.key] = p.value })
  return JSON.stringify(obj, null, 2)
}
