import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ApiRequest, ParamItem } from '../types'

/**
 * 接口编辑区草稿（与 MainPanel 编辑区 ref 一一对应）
 *
 * 覆盖 MainPanel 全部组件级编辑状态，切换 Tab 时整体保存/恢复，
 * 避免 Path Params、KV/JSON 模式、Auth 等任一字段跨接口泄漏。
 */
export interface RequestDraft {
  method: string
  url: string
  // Path Params：key 由 URL 解析派生，value 由用户输入，需隔离持久化
  pathParamValues: Record<string, string>
  queryParams: ParamItem[]
  requestHeaders: ParamItem[]
  bodyType: string
  bodyContent: string
  formDataParams: ParamItem[]
  urlencodedParams: ParamItem[]
  // Query Params UI 模式
  queryMode: 'table' | 'kv' | 'json'
  queryKvText: string
  queryJsonText: string
  // Headers UI 模式
  headerMode: 'table' | 'kv' | 'json'
  headerKvText: string
  headerJsonText: string
  // Body: form-urlencoded UI 模式
  urlencodedMode: 'table' | 'kv'
  urlencodedKvText: string
  // 注：Auth 字段不纳入 draftCache —— 现有实现中 Auth 改动通过
  // syncAuthConfig() 即时调用 updateRequest 落库，不存在未保存草稿。
  // 切 Tab 时从 req.auth_type / req.auth_config 按接口重新加载即可。
}

export const useRequestStore = defineStore('request', () => {
  // collectionId → ApiRequest[]
  const requestMap = ref<Record<number, ApiRequest[]>>({})
  const activeRequestId = ref<number | null>(null)
  // 记录已修改但未保存到 DB 的接口 ID（用于左侧树红色小点标记）
  const dirtyRequestIds = ref<Set<number>>(new Set())
  // 记录刚保存成功的接口 ID（用于左侧树绿色小点标记，1.5秒后自动消失）
  const savedRequestIds = ref<Set<number>>(new Set())
  // 编辑区草稿缓存：requestId → RequestDraft（切换接口时暂存未保存内容）
  const draftCache = ref<Record<number, RequestDraft>>({})

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
    // 注意：不在此处设置 activeRequestId，由调用方通过 tabStore.openTab() 驱动激活
    return req
  }

  async function updateRequest(id: number, data: Partial<ApiRequest>) {
    // 优先用 id 查找目标接口（兼容非激活接口保存），再回退到 activeRequest
    const current =
      Object.values(requestMap.value).flat().find(r => r.id === id) ??
      activeRequest.value
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

  /**
   * 将指定接口的当前草稿保存到 DB（等同 Ctrl+S）
   * 若 draftCache 中无草稿则直接返回（无需保存）
   */
  async function saveRequest(id: number): Promise<void> {
    const draft = draftCache.value[id]
    if (!draft) return
    // 将草稿序列化为接口字段
    const req = Object.values(requestMap.value).flat().find(r => r.id === id)
    if (!req) return

    let body = draft.bodyContent
    // form-data 序列化
    if (draft.bodyType === 'form_data') {
      body = JSON.stringify(draft.formDataParams)
    } else if (draft.bodyType === 'form_urlencoded') {
      const enabledFields = draft.urlencodedParams.filter(f => f.enabled && f.key)
      const sp = new URLSearchParams()
      enabledFields.forEach(f => sp.append(f.key, f.value))
      body = sp.toString()
    }

    await updateRequest(id, {
      method: draft.method,
      url: draft.url,
      params: JSON.stringify(draft.queryParams),
      headers: JSON.stringify(draft.requestHeaders),
      body_type: draft.bodyType,
      body,
    })
    // 清除草稿缓存
    const newCache = { ...draftCache.value }
    delete newCache[id]
    draftCache.value = newCache
    // 触发 saved 圆点动画
    const savedSet = new Set(savedRequestIds.value)
    savedSet.add(id)
    savedRequestIds.value = savedSet
    setTimeout(() => {
      const s = new Set(savedRequestIds.value)
      s.delete(id)
      savedRequestIds.value = s
    }, 1500)
  }

  async function deleteRequest(id: number, collectionId: number) {
    await invoke('delete_request', { id })
    const list = requestMap.value[collectionId] ?? []
    requestMap.value[collectionId] = list.filter(r => r.id !== id)
    // 注意：activeRequestId 由 tabStore.closeTab() → MainPanel watch 驱动，此处不再直接重置
    // 清除 dirty 标记
    const cleanSet = new Set(dirtyRequestIds.value)
    cleanSet.delete(id)
    dirtyRequestIds.value = cleanSet
    // 清除草稿缓存（已删除接口无需保留草稿）
    if (draftCache.value[id]) {
      const newCache = { ...draftCache.value }
      delete newCache[id]
      draftCache.value = newCache
    }
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
    draftCache,
    loadRequests,
    createRequest,
    updateRequest,
    deleteRequest,
    duplicateRequest,
    renameRequest,
    saveRequest,
  }
})
