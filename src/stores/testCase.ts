import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { TestCase, TestCaseHistory } from '../types'

export const useTestCaseStore = defineStore('testCase', () => {
  // requestId → TestCase[]
  const testCaseMap = ref<Record<number, TestCase[]>>({})

  // 当前激活的用例 id（null = 无激活，使用原始参数）
  const activeTestCaseId = ref<number | null>(null)

  // testCaseId → TestCaseHistory[]（按时间倒序，至多 10 条；M3-C 新增）
  const historyMap = ref<Record<number, TestCaseHistory[]>>({})

  function getByRequestId(requestId: number): TestCase[] {
    return testCaseMap.value[requestId] ?? []
  }

  async function loadTestCases(requestId: number) {
    // Tauri 2.x #[command] 宏把 Rust snake_case 参数名转为 camelCase IPC key
    const cases = await invoke<TestCase[]>('list_test_cases', { requestId })
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
      requestId: params.requestId,
      collectionId: params.collectionId,
      name: params.name,
      method: params.method ?? null,
      url: params.url ?? null,
      headers: params.headers ?? '[]',
      params: params.params_ ?? '[]',
      bodyType: params.bodyType ?? null,
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
      bodyType: data.body_type !== undefined ? data.body_type : current.body_type,
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
    // 同步清理本地历史镜像（DB 的 history 行已由 FK CASCADE 自动删除）
    delete historyMap.value[id]
    if (activeTestCaseId.value === id) activeTestCaseId.value = null
  }

  function clearForRequest(requestId: number) {
    const cases = testCaseMap.value[requestId] ?? []
    delete testCaseMap.value[requestId]
    // 同步清理这些用例的历史镜像
    for (const c of cases) delete historyMap.value[c.id]
    activeTestCaseId.value = null
  }

  // ── M3-C：用例执行历史 ─────────────────────────────────────────

  function getHistory(testCaseId: number): TestCaseHistory[] {
    return historyMap.value[testCaseId] ?? []
  }

  /** 拉取某用例的历史（最多 10 条，按时间倒序） */
  async function loadHistory(testCaseId: number): Promise<TestCaseHistory[]> {
    const list = await invoke<TestCaseHistory[]>('list_test_case_history', { testCaseId })
    historyMap.value[testCaseId] = list
    return list
  }

  /**
   * 写入一条历史。后端触发器自动滚动淘汰，前端做镜像同步：
   * 头插 + 截断 10。
   *
   * @param responsePreview 已由调用方裁剪到 ≤1KB
   */
  async function recordHistory(params: {
    testCaseId: number
    statusCode: number | null
    durationMs: number | null
    responsePreview: string | null
    errorMessage: string | null
  }): Promise<TestCaseHistory> {
    const row = await invoke<TestCaseHistory>('add_test_case_history', {
      testCaseId: params.testCaseId,
      statusCode: params.statusCode,
      durationMs: params.durationMs,
      responsePreview: params.responsePreview,
      errorMessage: params.errorMessage,
    })
    const list = historyMap.value[params.testCaseId] ?? []
    historyMap.value[params.testCaseId] = [row, ...list].slice(0, 10)
    return row
  }

  /**
   * 批量删除用例（M3-C）。
   * 后端 FK CASCADE 自动清理 test_case_history；前端同步清理 testCaseMap + historyMap。
   */
  async function deleteTestCases(ids: number[]): Promise<number> {
    if (ids.length === 0) return 0
    const affected = await invoke<number>('delete_test_cases', { ids })
    // 从所有 requestId 的列表中过滤
    for (const rid of Object.keys(testCaseMap.value)) {
      const r = Number(rid)
      const list = testCaseMap.value[r]
      const filtered = list.filter(c => !ids.includes(c.id))
      if (filtered.length !== list.length) {
        testCaseMap.value[r] = filtered
      }
    }
    // 清理历史镜像
    for (const id of ids) delete historyMap.value[id]
    if (activeTestCaseId.value !== null && ids.includes(activeTestCaseId.value)) {
      activeTestCaseId.value = null
    }
    return affected
  }

  return {
    testCaseMap,
    activeTestCaseId,
    historyMap,
    getByRequestId,
    loadTestCases,
    createTestCase,
    updateTestCase,
    deleteTestCase,
    deleteTestCases,
    clearForRequest,
    getHistory,
    loadHistory,
    recordHistory,
  }
})
