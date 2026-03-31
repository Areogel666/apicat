import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ApiRequest } from '../types'

export const useRequestStore = defineStore('request', () => {
  // collectionId → ApiRequest[]
  const requestMap = ref<Record<number, ApiRequest[]>>({})
  const activeRequestId = ref<number | null>(null)
  // 记录已修改但未保存到 DB 的接口 ID（用于左侧树红色小点标记）
  const dirtyRequestIds = ref<Set<number>>(new Set())
  // 记录刚保存成功的接口 ID（用于左侧树绿色小点标记，1.5秒后自动消失）
  const savedRequestIds = ref<Set<number>>(new Set())

  const activeRequest = computed<ApiRequest | null>(() => {
    if (!activeRequestId.value) return null
    for (const reqs of Object.values(requestMap.value)) {
      const found = reqs.find(r => r.id === activeRequestId.value)
      if (found) return found
    }
    return null
  })

  async function loadRequests(collectionId: number) {
    const rows = await invoke<ApiRequest[]>('list_requests', { collectionId })
    requestMap.value[collectionId] = rows
  }

  async function createRequest(collectionId: number, name: string, method: string, url: string) {
    const req = await invoke<ApiRequest>('create_request', {
      collectionId, name, method, url,
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
      bodyType: data.body_type ?? current.body_type,
      body: data.body ?? current.body,
      authType: data.auth_type ?? current.auth_type,
      authConfig: data.auth_config ?? current.auth_config,
    })
    const list = requestMap.value[current.collection_id] ?? []
    const idx = list.findIndex(r => r.id === id)
    if (idx !== -1) list[idx] = updated
    // 保存成功后清除 dirty 标记（替换整个 Set 以触发 Vue 3 响应式更新）
    const cleanSet = new Set(dirtyRequestIds.value)
    cleanSet.delete(id)
    dirtyRequestIds.value = cleanSet
    return updated
  }

  async function deleteRequest(id: number, collectionId: number) {
    await invoke('delete_request', { id })
    const list = requestMap.value[collectionId] ?? []
    requestMap.value[collectionId] = list.filter(r => r.id !== id)
    if (activeRequestId.value === id) activeRequestId.value = null
    // 删除接口时清除 dirty 标记（替换整个 Set 以触发 Vue 3 响应式更新）
    const cleanSet = new Set(dirtyRequestIds.value)
    cleanSet.delete(id)
    dirtyRequestIds.value = cleanSet
  }

  async function duplicateRequest(id: number) {
    const req = await invoke<ApiRequest>('duplicate_request', { id })
    const list = requestMap.value[req.collection_id] ?? []
    const srcIdx = list.findIndex(r => r.id === id)
    const insertAt = srcIdx >= 0 ? srcIdx + 1 : list.length
    const newList = [...list]
    newList.splice(insertAt, 0, req)
    requestMap.value[req.collection_id] = newList
    return req
  }

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
      bodyType: current.body_type,
      body: current.body,
      authType: current.auth_type,
      authConfig: current.auth_config,
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
    dirtyRequestIds,
    savedRequestIds,
    loadRequests,
    createRequest,
    updateRequest,
    deleteRequest,
    duplicateRequest,
    renameRequest,
  }
})
