import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface HeaderTemplateItem {
  id: string       // 唯一 ID（时间戳字符串）
  key: string
  value: string
  enabled: boolean
  description: string
}

const STORAGE_KEY = 'apicat_header_templates'

export const useHeaderTemplateStore = defineStore('headerTemplate', () => {
  const items = ref<HeaderTemplateItem[]>([])

  /** 从 localStorage 加载 */
  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      if (raw) items.value = JSON.parse(raw)
    } catch {
      items.value = []
    }
  }

  /** 持久化到 localStorage */
  function persist() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(items.value))
  }

  /** 新增一条模板 Header */
  function addItem(key = '', value = '', description = '') {
    items.value.push({
      id: String(Date.now()),
      key,
      value,
      enabled: true,
      description,
    })
    persist()
  }

  /** 更新某条 */
  function updateItem(id: string, patch: Partial<Omit<HeaderTemplateItem, 'id'>>) {
    const idx = items.value.findIndex(i => i.id === id)
    if (idx !== -1) {
      items.value[idx] = { ...items.value[idx], ...patch }
      persist()
    }
  }

  /** 删除某条 */
  function removeItem(id: string) {
    items.value = items.value.filter(i => i.id !== id)
    persist()
  }

  /** 返回所有启用的 Header 条目（用于「应用到编辑区」） */
  function getEnabledItems(): HeaderTemplateItem[] {
    return items.value.filter(i => i.enabled && i.key)
  }

  // 初始化时立即加载
  load()

  return {
    items,
    load,
    addItem,
    updateItem,
    removeItem,
    getEnabledItems,
  }
})
