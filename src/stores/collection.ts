import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Collection } from '../types'

export const useCollectionStore = defineStore('collection', () => {
  // projectId → Collection[]
  const collectionMap = ref<Record<number, Collection[]>>({})

  async function loadCollections(projectId: number) {
    const rows = await invoke<Collection[]>('list_collections', { project_id: projectId })
    collectionMap.value[projectId] = rows
  }

  function getCollections(projectId: number): Collection[] {
    return collectionMap.value[projectId] ?? []
  }

  async function createCollection(projectId: number, name: string, parentId?: number) {
    const col = await invoke<Collection>('create_collection', {
      project_id: projectId,
      parent_id: parentId ?? null,
      name,
    })
    const list = collectionMap.value[projectId] ?? []
    collectionMap.value[projectId] = [...list, col]
    return col
  }

  async function renameCollection(id: number, name: string, projectId: number) {
    const updated = await invoke<Collection>('rename_collection', { id, name })
    const list = collectionMap.value[projectId] ?? []
    const idx = list.findIndex(c => c.id === id)
    // 用 splice 替换元素，确保 Vue 响应式正确检测到数组变化
    if (idx !== -1) list.splice(idx, 1, updated)
    return updated
  }

  async function deleteCollection(id: number, projectId: number) {
    await invoke('delete_collection', { id })
    const list = collectionMap.value[projectId] ?? []
    collectionMap.value[projectId] = list.filter(c => c.id !== id)
  }

  return {
    collectionMap,
    loadCollections,
    getCollections,
    createCollection,
    renameCollection,
    deleteCollection,
  }
})
