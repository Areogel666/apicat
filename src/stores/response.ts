import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { HttpResponse, SendRequestParams } from '../types'

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
    touch()
  }

  /** 全量清空所有响应桶（切换项目等场景） */
  function clearAll(): void {
    buckets.value.clear()
    touch()
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
  }
})
