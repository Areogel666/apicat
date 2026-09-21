import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CaseType, RunCaseResult, TestCase, TestCaseHistory } from '../types'

/**
 * 按最近使用降序排列用例。
 *
 * 排序键 = last_run_at（用例被跑过的时刻）。共识 Q11-A：「使用」= 跑过该用例，
 * 单纯点开看看不算。未跑过的用例（last_run_at 为 null）排最后，且彼此保持
 * 原有相对顺序（稳定排序）——避免每次渲染顺序抖动。
 *
 * last_run_at 存的是 SQLite 的 `datetime('now')`（UTC，形如 "2026-09-21 08:00:00"），
 * 无时区标记且用空格分隔，Date.parse 会按本地时区误解析，故统一补 T/Z 转 UTC。
 * 与 HistoryTab.formatTime 的时间处理口径一致。
 */
export function sortCasesByLastRun(cases: TestCase[]): TestCase[] {
  return cases
    .map((c, i) => ({ c, i }))
    .sort((a, b) => {
      const ta = a.c.last_run_at ? Date.parse(a.c.last_run_at.replace(' ', 'T') + 'Z') : null
      const tb = b.c.last_run_at ? Date.parse(b.c.last_run_at.replace(' ', 'T') + 'Z') : null
      // 两者都未跑过：保持原相对顺序
      if (ta === null && tb === null) return a.i - b.i
      // 未跑过的排后面
      if (ta === null) return 1
      if (tb === null) return -1
      // 都跑过：最近的在前
      return tb - ta
    })
    .map(x => x.c)
}

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
    // 1.0.5：不再自动激活任何用例 —— 打开接口停在「原始参数、无激活用例」干净态，
    // 「激活用例」是用户显式点击动作（口径乙 / Q9′-A / Q10）。
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
    caseType?: CaseType
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
      caseType: params.caseType ?? 'happy_path',
    })
    const list = testCaseMap.value[params.requestId] ?? []
    testCaseMap.value[params.requestId] = [...list, tc]
    return tc
  }

  async function updateTestCase(id: number, data: Partial<Pick<TestCase,
    'name' | 'method' | 'url' | 'headers' | 'params' | 'body_type' | 'body' | 'case_type' | 'assertions'
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
      method: data.method !== undefined ? data.method : current.method,
      url: data.url !== undefined ? data.url : current.url,
      headers: data.headers ?? current.headers,
      params: data.params ?? current.params,
      bodyType: data.body_type !== undefined ? data.body_type : current.body_type,
      body: data.body !== undefined ? data.body : current.body,
      caseType: data.case_type !== undefined ? data.case_type : current.case_type,
      assertions: data.assertions !== undefined ? data.assertions : current.assertions,
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
   * 批量删除用例（M3-C）。
   * 后端 FK CASCADE 自动清理 request_history；前端同步清理 testCaseMap + historyMap。
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

  // ── 1.0.5：跑用例（断言求值）───────────────────────────────

  /**
   * 跑单个用例：发请求 → 断言求值 → 后端回写 last_status/last_response。
   * 返回完整结果（含每条断言的期望/实际），调用方负责展示。
   */
  async function runTestCase(testCaseId: number, envId?: number | null): Promise<RunCaseResult> {
    const result = await invoke<RunCaseResult>('run_test_case', {
      testCaseId,
      envId: envId ?? null,
    })
    // 同步本地镜像：用例的执行状态字段
    for (const list of Object.values(testCaseMap.value)) {
      const idx = list.findIndex(c => c.id === testCaseId)
      if (idx !== -1) {
        list[idx] = {
          ...list[idx],
          last_status: result.status,
          last_run_at: new Date().toISOString(),
          last_duration_ms: result.elapsed_ms,
          last_response: result.response_body.slice(0, 1024),
        }
        break
      }
    }
    // 刷新历史镜像（后端已插入一条）
    await loadHistory(testCaseId)
    return result
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
    runTestCase,
  }
})
