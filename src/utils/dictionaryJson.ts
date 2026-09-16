// 数据字典 JSON 编辑共用工具（1.0.4 fix：新建一键导入 / 右侧面板预览编辑）
import type { DictionaryItem } from '../types'

export interface DictItemInput {
  value: string
  label: string
  description: string
}

/**
 * 解析 JSON 文本为字典项：
 * - 键值对象 {"0":"成功","1":"失败"} → value=键, label=值
 * - 数组 [{"value":"0","label":"成功","description":"..."}]
 * 解析失败 / 空输入 → 空数组
 */
export function parseItemsJson(text: string): DictItemInput[] {
  const out: DictItemInput[] = []
  try {
    const data = JSON.parse(text.trim() || '{}')
    if (Array.isArray(data)) {
      for (const it of data) {
        if (it && typeof it === 'object') {
          const value = it.value !== undefined ? String(it.value) : (it.label !== undefined ? String(it.label) : '')
          const label = it.label !== undefined ? String(it.label) : (it.value !== undefined ? String(it.value) : '')
          out.push({ value, label, description: it.description !== undefined ? String(it.description) : '' })
        }
      }
    } else if (data && typeof data === 'object') {
      for (const [k, v] of Object.entries(data)) {
        out.push({ value: k, label: String(v), description: '' })
      }
    }
  } catch { /* 保留解析失败为空，由调用方提示 */ }
  return out
}

/** 字典项列表 → 完整数组 JSON（用于右侧预览编辑，description 为空时省略） */
export function itemsToJson(items: DictionaryItem[]): string {
  if (items.length === 0) return '[]'
  return JSON.stringify(items.map(i => ({
    value: i.value,
    label: i.label,
    description: i.description || undefined,
  })), null, 2)
}