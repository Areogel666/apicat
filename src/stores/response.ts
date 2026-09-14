import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { HttpResponse, SendRequestParams } from '../types'
import type { ResponseFormat, ViewMode } from '../components/response/useResponseFormat'

/**
 * 用户手动覆盖的响应格式：
 *   - 'auto'      跟随 detectFormat 自动识别
 *   - 其他值      强制按该格式渲染
 *
 * 仅内存级按 requestId 分桶，不持久化（格式选择是"查看态"，重启不记忆符合直觉）。
 */
export type FormatOverride = ResponseFormat | 'auto'

/**
 * 单个接口的响应桶。每个 requestId 一份，互不干扰。
 */
interface ResponseBucket {
  response: HttpResponse | null
  loading: boolean
  error: string | null
}

/**
 * 响应 Store —— 按 requestId + testCaseId 组合 key 分桶，彻底隔离多 Tab 与多用例响应数据。
 *
 * 1.0.4：桶粒度由 requestId 扩展为 requestId + testCaseId。
 * key = `${requestId}:${testCaseId}`，testCaseId 为 null/缺省时用 'raw'（表示"原始参数调试"桶）。
 *
 * 对外暴露的 `response / loading / error` 是"当前激活请求+用例"的视图，
 * 由 `activeRequestId + activeTestCaseId` 驱动。切换用例只需改 activeTestCaseId，响应面板自动刷新。
 */
export const useResponseStore = defineStore('response', () => {
  const buckets = ref<Map<string, ResponseBucket>>(new Map())
  const activeRequestId = ref<number | null>(null)
  const activeTestCaseId = ref<number | null>(null)

  // ── 响应面板视图状态（按 key 分桶，内存级隔离，不入 DB） ──
  // 用户手动覆盖的格式：key=requestId，value='auto'|具体格式
  const formatOverrideMap = ref<Record<number, FormatOverride>>({})
  // 用户选择的视图模式：key=requestId，value=raw/pretty/preview
  const viewModeMap = ref<Record<number, ViewMode>>({})

  function bucketKey(requestId: number, testCaseId: number | null): string {
    return `${requestId}:${testCaseId ?? 'raw'}`
  }

  function getBucket(requestId: number, testCaseId: number | null): ResponseBucket {
    const key = bucketKey(requestId, testCaseId)
    let b = buckets.value.get(key)
    if (!b) {
      b = { response: null, loading: false, error: null }
      buckets.value.set(key, b)
    }
    return b
  }

  /** 触发 Map 响应式更新（Vue 3 对 Map.set/delete 有反应性，但显式替换更稳妥） */
  function touch() {
    buckets.value = new Map(buckets.value)
  }

  /** 设置当前激活桶（切换接口 / 切换用例时调用） */
  function setCurrent(requestId: number | null, testCaseId: number | null) {
    activeRequestId.value = requestId
    activeTestCaseId.value = testCaseId
  }

  const currentKey = computed<string | null>(() =>
    activeRequestId.value == null ? null : bucketKey(activeRequestId.value, activeTestCaseId.value))

  // ── 对外视图：当前激活请求+用例的响应/加载/错误 ──────────

  const response = computed<HttpResponse | null>(() => {
    const k = currentKey.value
    return k ? (buckets.value.get(k)?.response ?? null) : null
  })

  const loading = computed<boolean>(() => {
    const k = currentKey.value
    return k != null ? (buckets.value.get(k)?.loading ?? false) : false
  })

  const error = computed<string | null>(() => {
    const k = currentKey.value
    return k != null ? (buckets.value.get(k)?.error ?? null) : null
  })

  // ── Actions ──────────────────────────────────────────────────

  /**
   * 发送请求并把结果写入对应 requestId + testCaseId 的桶。
   * 其他接口/用例的响应不受影响。
   * testCaseId 传实际值则写入该用例桶；null = 写入 "raw"（原始参数调试）桶。
   */
  async function sendRequest(
    requestId: number,
    testCaseId: number | null,
    params: SendRequestParams,
    envId: number | null,
    projectId: number | null,
  ): Promise<HttpResponse | null> {
    const b = getBucket(requestId, testCaseId)
    b.loading = true
    b.error = null
    touch()
    try {
      // Tauri 2.x #[command] 宏把 Rust snake_case 参数名转为 camelCase IPC key
      b.response = await invoke<HttpResponse>('send_request', {
        requestId,
        params,
        envId,
        projectId,
      })
    } catch (e) {
      b.error = String(e)
      b.response = null
    } finally {
      b.loading = false
      touch()
    }
    return b.response
  }

  /**
   * 清理指定请求+用例的响应桶。
   * 不传参：清理当前激活桶（兼容原有 `clear()` 语义）。
   * 传入 requestId/testCaseId：清理指定桶（关闭 Tab、删除接口/用例时调用）。
   */
  function clear(requestId?: number, testCaseId: number | null = null): void {
    let targetId = requestId
    if (targetId == null) {
      if (activeRequestId.value == null) return
      targetId = activeRequestId.value
    }
    const key = bucketKey(targetId, testCaseId)
    buckets.value.delete(key)
    // 同步清理视图状态，避免 Map 泄漏
    if (requestId != null) {
      delete formatOverrideMap.value[requestId]
      delete viewModeMap.value[requestId]
    }
    touch()
  }

  /**
   * 清空所有响应桶（切换项目等场景） */
  function clearAll(): void {
    buckets.value.clear()
    formatOverrideMap.value = {}
    viewModeMap.value = {}
    touch()
  }

  /**
   * 1.0.4：把某个用例桶的响应迁移到另一个用例桶。
   * 用于「首次发送自动创建用例」场景：发送时没有用例 → 响应写入 raw 桶，
   * 随后创建了「用例 1」并把 activeTestCaseId 指向它 → 把 raw 桶内容搬到新用例桶。
   */
  function moveResponse(sourceTestCaseId: number | null, targetTestCaseId: number): void {
    if (activeRequestId.value == null) return
    const srcKey = bucketKey(activeRequestId.value, sourceTestCaseId)
    const dstKey = bucketKey(activeRequestId.value, targetTestCaseId)
    const src = buckets.value.get(srcKey)
    if (!src) return
    buckets.value.set(dstKey, { ...src })
    buckets.value.delete(srcKey)
    touch()
  }

  // ── 视图状态 getter/setter ───────────────────────────────────

  /** 读取格式覆盖，未设置时返回 'auto' */
  function getFormatOverride(requestId: number): FormatOverride {
    return formatOverrideMap.value[requestId] ?? 'auto'
  }

  function setFormatOverride(requestId: number, format: FormatOverride): void {
    formatOverrideMap.value[requestId] = format
  }

  /** 读取视图模式，未设置时返回 null（由组件根据格式给默认值） */
  function getViewMode(requestId: number): ViewMode | null {
    return viewModeMap.value[requestId] ?? null
  }

  function setViewMode(requestId: number, mode: ViewMode): void {
    viewModeMap.value[requestId] = mode
  }

  return {
    // state
    activeRequestId,
    activeTestCaseId,
    // views
    response,
    loading,
    error,
    // actions
    sendRequest,
    setCurrent,
    moveResponse,
    clear,
    clearAll,
    // format/view state
    getFormatOverride,
    setFormatOverride,
    getViewMode,
    setViewMode,
  }
})
