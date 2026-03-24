import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { HistoryRecord } from '../types'

export const useHistoryStore = defineStore('history', () => {
  // requestId → HistoryRecord[]（最近 20 条）
  const historyMap = ref<Record<number, HistoryRecord[]>>({})

  async function loadHistory(requestId: number) {
    const rows = await invoke<HistoryRecord[]>('list_history', { request_id: requestId })
    historyMap.value[requestId] = rows
  }

  function getHistory(requestId: number): HistoryRecord[] {
    return historyMap.value[requestId] ?? []
  }

  // 发送请求成功后，把最新记录插入到对应列表头部
  function prependRecord(requestId: number, record: HistoryRecord) {
    const list = historyMap.value[requestId] ?? []
    // 保持最多 20 条
    historyMap.value[requestId] = [record, ...list].slice(0, 20)
  }

  return { historyMap, loadHistory, getHistory, prependRecord }
})
