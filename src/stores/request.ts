import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ApiRequest } from '../types'

export const useRequestStore = defineStore('request', () => {
  // collectionId → ApiRequest[]
  const requestMap = ref<Record<number, ApiRequest[]>>({})
  const activeRequestId = ref<number | null>(null)

  const activeRequest = computed<ApiRequest | null>(() => {
    if (!activeRequestId.value) return null
    for (const reqs of Object.values(requestMap.value)) {
      const found = reqs.find(r => r.id === activeRequestId.value)
      if (found) return found
    }
    return null
  })

  async function loadRequests(collectionId: number) {
    const rows = await invoke<ApiRequest[]>('list_requests', { collection_id: collectionId })
    requestMap.value[collectionId] = rows
  }

  async function createRequest(collectionId: number, name: string, method: string, url: string) {
    const req = await invoke<ApiRequest>('create_request', {
      collection_id: collectionId, name, method, url,
    })
    const list = requestMap.value[collectionId] ?? []
    requestMap.value[collectionId] = [...list, req]
    activeRequestId.value = req.id
    return req
  }

  async function updateRequest(id: number, data: Partial<ApiRequest>) {
    const current = activeRequest.value
    if (!current) throw new Error('No active request')
    const updated = await invoke<ApiRequest>('update_request', {
      id,
      name: data.name ?? current.name,
      method: data.method ?? current.method,
      url: data.url ?? current.url,
      params: data.params ?? current.params,
      headers: data.headers ?? current.headers,
      body_type: data.body_type ?? current.body_type,
      body: data.body ?? current.body,
      auth_type: data.auth_type ?? current.auth_type,
      auth_config: data.auth_config ?? current.auth_config,
    })
    const list = requestMap.value[current.collection_id] ?? []
    const idx = list.findIndex(r => r.id === id)
    if (idx !== -1) list[idx] = updated
    return updated
  }

  async function deleteRequest(id: number, collectionId: number) {
    await invoke('delete_request', { id })
    const list = requestMap.value[collectionId] ?? []
    requestMap.value[collectionId] = list.filter(r => r.id !== id)
    if (activeRequestId.value === id) activeRequestId.value = null
  }

  /** 复制接口（克隆所有字段，名称自动追加「副本」） */
  async function duplicateRequest(id: number) {
    const req = await invoke<ApiRequest>('duplicate_request', { id })
    const list = requestMap.value[req.collection_id] ?? []
    // 插入到原接口后面
    const srcIdx = list.findIndex(r => r.id === id)
    const insertAt = srcIdx >= 0 ? srcIdx + 1 : list.length
    const newList = [...list]
    newList.splice(insertAt, 0, req)
    requestMap.value[req.collection_id] = newList
    return req
  }

  /** 重命名接口（复用 updateRequest，只改 name） */
  async function renameRequest(id: number, name: string) {
    const current = Object.values(requestMap.value).flat().find(r => r.id === id)
    if (!current) throw new Error('Request not found')
    const updated = await invoke<ApiRequest>('update_request', {
      id,
      name,
      method: current.method,
      url: current.url,
      params: current.params,
      headers: current.headers,
      body_type: current.body_type,
      body: current.body,
      auth_type: current.auth_type,
      auth_config: current.auth_config,
    })
    const list = requestMap.value[current.collection_id] ?? []
    const idx = list.findIndex(r => r.id === id)
    if (idx !== -1) list[idx] = updated
    return updated
  }

  return {
    requestMap,
    activeRequestId,
    activeRequest,
    loadRequests,
    createRequest,
    updateRequest,
    deleteRequest,
    duplicateRequest,
    renameRequest,
  }
})
