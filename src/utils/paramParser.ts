import type { ParamItem } from '../types'

/**
 * 用旧参数表中同 key 的元数据（type / description / descriptionDictRef）补全解析结果。
 * KV/JSON 文本本身不表达元数据，切换模式时用 prev 保留表格里已填的类型与描述（1.0.4 fix）。
 */
function inheritMeta(item: ParamItem, prev?: ParamItem[]): ParamItem {
  if (!prev) return item
  const src = prev.find(p => p.key === item.key)
  if (!src) return item
  return { ...item, type: src.type, description: src.description, descriptionDictRef: src.descriptionDictRef }
}

/**
 * 解析 KV 文本（key: value 或 key=value 格式）为 ParamItem[]
 * 跳过空行和纯注释行（# 开头）
 * @param prev 切换模式前的旧参数表，用于保留同 key 的类型/描述
 */
export function parseKvText(text: string, prev?: ParamItem[]): ParamItem[] {
  return text
    .split('\n')
    .map(line => line.trim())
    .filter(line => line && !line.startsWith('#'))
    .map(line => {
      // 先尝试 ": " 分隔（HTTP Header 风格），再尝试 "=" 分隔（query string 风格）
      const colonIdx = line.indexOf(': ')
      if (colonIdx !== -1) {
        return inheritMeta({ key: line.slice(0, colonIdx).trim(), value: line.slice(colonIdx + 2).trim(), enabled: true }, prev)
      }
      const eqIdx = line.indexOf('=')
      if (eqIdx !== -1) {
        return inheritMeta({ key: line.slice(0, eqIdx).trim(), value: line.slice(eqIdx + 1).trim(), enabled: true }, prev)
      }
      return inheritMeta({ key: line, value: '', enabled: true }, prev)
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
 * @param prev 切换模式前的旧参数表，用于保留同 key 的类型/描述
 */
export function parseJsonToParams(jsonText: string, prev?: ParamItem[]): ParamItem[] {
  try {
    const obj = JSON.parse(jsonText)
    if (typeof obj !== 'object' || Array.isArray(obj) || obj === null) return []
    return Object.entries(obj).map(([key, value]) =>
      inheritMeta({ key, value: String(value), enabled: true }, prev))
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
