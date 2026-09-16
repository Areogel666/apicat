import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
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

  // 1.0.4 fix：字典管理页选中的字典 id（左侧树 → 右侧 JSON 编辑面板联动）
  const selectedDictId = ref<number | null>(null)
  const selectedDict = computed<DataDictionary | null>(() =>
    selectedDictId.value != null
      ? (dictionaries.value.find(d => d.id === selectedDictId.value) ?? null)
      : null,
  )

  function setSelectedDict(id: number | null) {
    selectedDictId.value = id
  }

  async function loadDictionaries(projectId: number | null = null) {
    dictionaries.value = await invoke<DataDictionary[]>('list_dictionaries', { projectId })
    // 预加载所有可视字典的 items（字典量小，全量拉取简单）
    for (const d of dictionaries.value) {
      itemsMap.value[d.id] = await invoke<DictionaryItem[]>('list_dictionary_items', {
        dictionaryId: d.id,
      })
    }
  }

  /** 新建数据字典（全局共享，project 维度留待后续） */
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

  /** 1.0.4 fix：一键新建字典 + 初始字典项（同事务） */
  async function createDictionaryWithItems(
    code: string,
    name: string,
    description: string,
    items: Array<{ label: string; value: string; description?: string }>,
  ) {
    const d = await invoke<DataDictionary>('create_dictionary_with_items', {
      code,
      name,
      description,
      projectId: null,
      items,
    })
    dictionaries.value.push(d)
    itemsMap.value[d.id] = await invoke<DictionaryItem[]>('list_dictionary_items', {
      dictionaryId: d.id,
    })
    return d
  }

  /** 1.0.4 fix：以 JSON 全量替换某字典的字典项（先清空再插入） */
  async function replaceDictionaryItems(
    dictionaryId: number,
    items: Array<{ label: string; value: string; description?: string }>,
  ) {
    const rows = await invoke<DictionaryItem[]>('replace_dictionary_items', { dictionaryId, items })
    itemsMap.value[dictionaryId] = rows
    return rows
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
    selectedDictId,
    selectedDict,
    setSelectedDict,
    loadDictionaries,
    createDictionary,
    createDictionaryWithItems,
    updateDictionary,
    deleteDictionary,
    createItem,
    updateItem,
    deleteItem,
    replaceDictionaryItems,
    getItemById,
  }
})