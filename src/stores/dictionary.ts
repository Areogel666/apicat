import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { DataDictionary, DictionaryItem } from '../types'

/**
 * 数据字典 Store —— 全局共享字典（builtin）与项目自定义字典。
 * items 按 dictionary_id 分桶缓存。
 */
export const useDictionaryStore = defineStore('dictionary', () => {
  const dictionaries = ref<DataDictionary[]>([])
  // dictionaryId → DictionaryItem[]
  const itemsMap = ref<Record<number, DictionaryItem[]>>({})

  async function loadDictionaries(projectId: number | null = null) {
    dictionaries.value = await invoke<DataDictionary[]>('list_dictionaries', { projectId })
    // 预加载所有可视字典的 items（字典量小，全量拉取简单）
    for (const d of dictionaries.value) {
      itemsMap.value[d.id] = await invoke<DictionaryItem[]>('list_dictionary_items', {
        dictionaryId: d.id,
      })
    }
  }

  /** 按 code 或 name 模糊查找字典（用于显示描述时）. */
  async function createDictionary(code: string, name: string, description = '') {
    const d = await invoke<DataDictionary>('create_dictionary', {
      code,
      name,
      description,
      projectId: null,
    })
    dictionaries.value.push(d)
    itemsMap.value[d.id] = []
    return d
  }

  async function updateDictionary(id: number, data: { name?: string; description?: string }) {
    const d = await invoke<DataDictionary>('update_dictionary', { id, ...data })
    const i = dictionaries.value.findIndex(x => x.id === id)
    if (i >= 0) dictionaries.value[i] = d
    return d
  }

  async function deleteDictionary(id: number) {
    await invoke('delete_dictionary', { id })
    dictionaries.value = dictionaries.value.filter(x => x.id !== id)
    delete itemsMap.value[id]
  }

  async function createItem(dictionaryId: number, label: string, value: string, description = '') {
    const item = await invoke<DictionaryItem>('create_dictionary_item', {
      dictionaryId,
      label,
      value,
      description,
    })
    itemsMap.value[dictionaryId].push(item)
    return item
  }

  async function updateItem(
    id: number,
    data: { label?: string; value?: string; description?: string; sortOrder?: number },
  ) {
    const item = await invoke<DictionaryItem>('update_dictionary_item', { id, ...data })
    for (const list of Object.values(itemsMap.value)) {
      const i = list.findIndex(x => x.id === id)
      if (i >= 0) { list[i] = item; break }
    }
    return item
  }

  async function deleteItem(id: number) {
    await invoke('delete_dictionary_item', { id })
    for (const list of Object.values(itemsMap.value)) {
      const i = list.findIndex(x => x.id === id)
      if (i >= 0) { list.splice(i, 1); break }
    }
  }

  /** 通过 item id 反查字典项（用于展示 descriptionDictRef 指向的描述） */
  function getItemById(id: number | null | undefined): DictionaryItem | null {
    if (id == null) return null
    for (const list of Object.values(itemsMap.value)) {
      const found = list.find(x => x.id === id)
      if (found) return found
    }
    return null
  }

  return {
    dictionaries,
    itemsMap,
    loadDictionaries,
    createDictionary,
    updateDictionary,
    deleteDictionary,
    createItem,
    updateItem,
    deleteItem,
    getItemById,
  }
})