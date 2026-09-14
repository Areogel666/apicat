import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { HistoryRecord } from '../types'

export const useHistoryStore = defineStore('history', () => {
  // key = `${requestId}:${testCaseId|null}`（1.0.4 起按用例分桶；DB 无 testCaseId 列，
  // 内存层先把同一 request 的数据按用例分别缓存，切用例时重载。）
  const historyMap = ref<Record<string, HistoryRecord[]>>({})

  function historyKey(requestId: number, testCaseId: number | null): string {
    return `${requestId}:${testCaseId ?? 'raw'}`
  }

  async function loadHistory(requestId: number, testCaseId: number | null = null) {
    // Tauri 2.x #[command] 宏把 Rust snake_case 参数名转为 camelCase IPC key
    const rows = await invoke<HistoryRecord[]>('list_history', { requestId })
    const key = historyKey(requestId, testCaseId)
    historyMap.value[key] = rows
  }

  function getHistory(requestId: number, testCaseId: number | null = null): HistoryRecord[] {
    return historyMap.value[historyKey(requestId, testCaseId)] ?? []
  }

  // 发送请求成功后，把最新记录插入到对应 key 的列表头部
  function prependRecord(requestId: number, testCaseId: number | null, record: HistoryRecord) {
    const key = historyKey(requestId, testCaseId)
    const list = historyMap.value[key] ?? []
    // 保持最多 20 条
    historyMap.value[key] = [record, ...list].slice(0, 20)
  }

  return { historyMap, loadHistory, getHistory, prependRecord }
})