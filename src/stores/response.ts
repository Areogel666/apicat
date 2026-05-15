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
 * 响应 Store —— 按 requestId 分桶，彻底隔离多 Tab 响应数据。
 *
 * 对外暴露的 `response / loading / error` 是"当前激活接口"的视图，
 * 由 `activeRequestId` 驱动。切换接口只需改 activeRequestId，响应面板自动刷新。
 *
 * 内部存储：
 *   buckets: Map<requestId, ResponseBucket>
 * 读写访问必须通过 getBucket() 确保桶存在；写操作后需要触发 Map 的响应式更新。
 */
export const useResponseStore = defineStore('response', () => {
  const buckets = ref<Map<number, ResponseBucket>>(new Map())
  const activeRequestId = ref<number | null>(null)

  // ── 响应面板视图状态（按 requestId 分桶，内存级隔离，不入 DB） ──
  // 用户手动覆盖的格式：key=requestId，value='auto'|具体格式
  const formatOverrideMap = ref<Record<number, FormatOverride>>({})
  // 用户选择的视图模式：key=requestId，value=raw/pretty/preview
  const viewModeMap = ref<Record<number, ViewMode>>({})

  function getBucket(id: number): ResponseBucket {
    let b = buckets.value.get(id)
    if (!b) {
      b = { response: null, loading: false, error: null }
      buckets.value.set(id, b)
    }
    return b
  }

  /** 触发 Map 响应式更新（Vue 3 对 Map.set/delete 有反应性，但显式替换更稳妥） */
  function touch() {
    buckets.value = new Map(buckets.value)
  }

  // ── 对外视图：当前激活接口的响应/加载/错误 ────────────────────

  const response = computed<HttpResponse | null>(() => {
    const id = activeRequestId.value
    return id != null ? (buckets.value.get(id)?.response ?? null) : null
  })

  const loading = computed<boolean>(() => {
    const id = activeRequestId.value
    return id != null ? (buckets.value.get(id)?.loading ?? false) : false
  })

  const error = computed<string | null>(() => {
    const id = activeRequestId.value
    return id != null ? (buckets.value.get(id)?.error ?? null) : null
  })

  // ── Actions ──────────────────────────────────────────────────

  /**
   * 发送请求并把结果写入对应 requestId 的桶。
   * 其他接口的响应不受影响。
   */
  async function sendRequest(
    requestId: number,
    params: SendRequestParams,
    envId: number | null,
    projectId: number | null,
  ): Promise<HttpResponse | null> {
    const b = getBucket(requestId)
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
   * 清理指定接口的响应桶。
   *
   * - 不传参：清理当前激活接口（兼容原有 `clear()` 语义）
   * - 传入 id：清理指定接口（关闭 Tab、删除接口时调用）
   */
  function clear(requestId?: number): void {
    const targetId = requestId ?? activeRequestId.value
    if (targetId == null) return
    buckets.value.delete(targetId)
    // 同步清理视图状态，避免 Map 泄漏
    delete formatOverrideMap.value[targetId]
    delete viewModeMap.value[targetId]
    touch()
  }

  /** 全量清空所有响应桶（切换项目等场景） */
  function clearAll(): void {
    buckets.value.clear()
    formatOverrideMap.value = {}
    viewModeMap.value = {}
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
    // views
    response,
    loading,
    error,
    // actions
    sendRequest,
    clear,
    clearAll,
    // format/view state
    getFormatOverride,
    setFormatOverride,
    getViewMode,
    setViewMode,
  }
})
