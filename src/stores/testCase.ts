import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { TestCase } from '../types'

export const useTestCaseStore = defineStore('testCase', () => {
  // requestId → TestCase[]
  const testCaseMap = ref<Record<number, TestCase[]>>({})

  // 当前激活的用例 id（null = 无激活，使用原始参数）
  const activeTestCaseId = ref<number | null>(null)

  function getByRequestId(requestId: number): TestCase[] {
    return testCaseMap.value[requestId] ?? []
  }

  async function loadTestCases(requestId: number) {
    const cases = await invoke<TestCase[]>('list_test_cases', { request_id: requestId })
    testCaseMap.value[requestId] = cases
    // 自动激活第一个收藏用例（仅在无激活时）
    if (activeTestCaseId.value === null) {
      const starred = cases.find(c => c.starred === 1)
      if (starred) activeTestCaseId.value = starred.id
    }
  }

  async function createTestCase(params: {
    requestId: number
    collectionId: number
    name: string
    method?: string | null
    url?: string | null
    headers?: string
    params_?: string
    bodyType?: string | null
    body?: string | null
  }): Promise<TestCase> {
    const tc = await invoke<TestCase>('create_test_case', {
      request_id: params.requestId,
      collection_id: params.collectionId,
      name: params.name,
      method: params.method ?? null,
      url: params.url ?? null,
      headers: params.headers ?? '[]',
      params: params.params_ ?? '[]',
      body_type: params.bodyType ?? null,
      body: params.body ?? null,
    })
    const list = testCaseMap.value[params.requestId] ?? []
    testCaseMap.value[params.requestId] = [...list, tc]
    return tc
  }

  async function updateTestCase(id: number, data: Partial<Pick<TestCase,
    'name' | 'starred' | 'method' | 'url' | 'headers' | 'params' | 'body_type' | 'body'
  >>): Promise<TestCase> {
    // 先取当前值做 fallback
    let current: TestCase | undefined
    for (const list of Object.values(testCaseMap.value)) {
      current = list.find(c => c.id === id)
      if (current) break
    }
    if (!current) throw new Error('TestCase not found')

    const updated = await invoke<TestCase>('update_test_case', {
      id,
      name: data.name ?? current.name,
      starred: data.starred ?? current.starred,
      method: data.method !== undefined ? data.method : current.method,
      url: data.url !== undefined ? data.url : current.url,
      headers: data.headers ?? current.headers,
      params: data.params ?? current.params,
      body_type: data.body_type !== undefined ? data.body_type : current.body_type,
      body: data.body !== undefined ? data.body : current.body,
    })

    const requestId = current.request_id
    if (requestId !== null) {
      const list = testCaseMap.value[requestId] ?? []
      const idx = list.findIndex(c => c.id === id)
      if (idx !== -1) list[idx] = updated
    }
    return updated
  }

  async function deleteTestCase(id: number) {
    await invoke('delete_test_case', { id })
    for (const [rid, list] of Object.entries(testCaseMap.value)) {
      const filtered = list.filter(c => c.id !== id)
      if (filtered.length !== list.length) {
        testCaseMap.value[Number(rid)] = filtered
        break
      }
    }
    if (activeTestCaseId.value === id) activeTestCaseId.value = null
  }

  function clearForRequest(requestId: number) {
    delete testCaseMap.value[requestId]
    activeTestCaseId.value = null
  }

  return {
    testCaseMap,
    activeTestCaseId,
    getByRequestId,
    loadTestCases,
    createTestCase,
    updateTestCase,
    deleteTestCase,
    clearForRequest,
  }
})
