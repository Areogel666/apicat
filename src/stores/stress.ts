import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { StressStats, StressConfig, StressChartPoint, StressRun } from '../types'

export const useStressStore = defineStore('stress', () => {
  const isRunning = ref(false)
  const stats = ref<StressStats | null>(null)
  const chartPoints = ref<StressChartPoint[]>([])
  const error = ref<string | null>(null)
  // 1.0.4：压测历史（当前接口）
  const history = ref<StressRun[]>([])

  // 取消订阅函数（组件卸载时调用）
  let unlistenProgress: UnlistenFn | null = null
  let unlistenDone: UnlistenFn | null = null

  const successRate = computed(() => stats.value?.success_rate ?? 0)
  const isDone = computed(() => stats.value?.done ?? false)

  /**
   * 启动压测
   * requestId：当前接口 id（结果落库关联）
   * params：SendRequestParams 结构（与普通发请求相同）
   */
  async function startStress(
    requestId: number,
    params: {
      method: string
      url: string
      query_params: Array<{ key: string; value: string; enabled: boolean }>
      headers: Array<{ key: string; value: string; enabled: boolean }>
      body_type: string
      body: string
      path_params: Array<{ key: string; value: string; enabled: boolean }>
    },
    config: StressConfig
  ) {
    // 清理可能残留的旧监听器（防止连续调用时监听器重复积累）
    cleanup()

    // 重置状态
    stats.value = null
    chartPoints.value = []
    error.value = null
    isRunning.value = true

    try {
      // 监听器注册在 try 内：失败时状态机能正常回滚
      unlistenProgress = await listen<StressStats>('stress://progress', (event) => {
        stats.value = event.payload
        // 追加折线图数据点，限制最大长度防止长压测内存持续涨
        // 200ms 一个点，保留最近 1500 个点 = 约 5 分钟可视窗口
        const MAX_CHART_POINTS = 1500
        if (chartPoints.value.length >= MAX_CHART_POINTS) {
          chartPoints.value.shift()
        }
        chartPoints.value.push({
          time: event.payload.elapsed_sec,
          tps: event.payload.tps,
          avg_ms: event.payload.avg_ms,
          p95_ms: event.payload.p95_ms,
        })
      })

      // 订阅完成事件
      unlistenDone = await listen<StressStats>('stress://done', (event) => {
        stats.value = event.payload
        isRunning.value = false
        cleanup()
      })

      await invoke('start_stress', {
        requestId,
        params,
        concurrent: config.concurrent,
        mode: config.mode,
        value: config.value,
        expectStatus: config.expect_status,
        p95ThresholdMs: config.p95_threshold_ms ?? null,
        p99ThresholdMs: config.p99_threshold_ms ?? null,
      })
    } catch (e) {
      error.value = String(e)
      isRunning.value = false
      cleanup()
    }
  }

  // 1.0.4：加载某接口的压测历史 + 删除单条
  async function loadHistory(requestId: number) {
    history.value = await invoke<StressRun[]>('list_stress_runs', { requestId })
  }

  async function removeRun(id: number) {
    await invoke('delete_stress_run', { id })
    history.value = history.value.filter(r => r.id !== id)
  }

  function cleanup() {
    unlistenProgress?.()
    unlistenDone?.()
    unlistenProgress = null
    unlistenDone = null
  }

  function reset() {
    stats.value = null
    chartPoints.value = []
    error.value = null
    isRunning.value = false
    cleanup()
  }

  return {
    isRunning,
    stats,
    chartPoints,
    error,
    history,
    successRate,
    isDone,
    startStress,
    loadHistory,
    removeRun,
    reset,
    cleanup,
  }
})
