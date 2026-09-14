import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { HistoryRecord } from '../types'

export const useHistoryStore = defineStore('history', () => {
  // key = `${requestId}:${testCaseId|null}`（1.0.4 起按用例分桶；DB request_history 有
  // test_case_id 列，list_history 按用例过滤，切用例 History 才真正隔离）
  const historyMap = ref<Record<string, HistoryRecord[]>>({})

  function historyKey(requestId: number, testCaseId: number | null): string {
    return `${requestId}:${testCaseId ?? 'raw'}`
  }

  async function loadHistory(requestId: number, testCaseId: number | null = null) {
    const rows = await invoke<HistoryRecord[]>('list_history', { requestId, testCaseId })
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