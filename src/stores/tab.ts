import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { load } from '@tauri-apps/plugin-store'
import type { ApiRequest } from '../types'

// Tab 数据结构
export interface TabItem {
  requestId: number
  // 显示标题所需的快照（避免每次从 requestStore 查找）
  title: string   // 格式："{Method} {name}"，如 "POST 登录"
}

export const useTabStore = defineStore('tab', () => {
  const openTabs = ref<TabItem[]>([])
  const activeRequestId = ref<number | null>(null)

  // ── 只读计算属性 ──────────────────────────────────────────

  const activeTab = computed<TabItem | null>(() =>
    openTabs.value.find(t => t.requestId === activeRequestId.value) ?? null
  )

  function isOpen(requestId: number): boolean {
    return openTabs.value.some(t => t.requestId === requestId)
  }

  // ── Tab 操作 ──────────────────────────────────────────────

  /** 打开接口：已有则激活，未有则新开 */
  function openTab(request: ApiRequest) {
    if (isOpen(request.id)) {
      activateTab(request.id)
      return
    }
    const tab: TabItem = {
      requestId: request.id,
      title: `${request.method} ${request.name}`,
    }
    openTabs.value.push(tab)
    activeRequestId.value = request.id
  }

  /** 激活指定 Tab */
  function activateTab(requestId: number) {
    if (isOpen(requestId)) {
      activeRequestId.value = requestId
    }
  }

  /**
   * 关闭指定 Tab，处理激活状态迁移：
   * - 关闭非激活 Tab：激活状态不变
   * - 关闭激活 Tab：激活左侧相邻；若为最左则激活新第一个；全关则 null
   */
  function closeTab(requestId: number) {
    const idx = openTabs.value.findIndex(t => t.requestId === requestId)
    if (idx === -1) return

    const wasActive = activeRequestId.value === requestId
    openTabs.value.splice(idx, 1)

    if (wasActive) {
      if (openTabs.value.length === 0) {
        activeRequestId.value = null
      } else {
        // 激活左侧相邻；若关闭的是第一个则激活新第一个
        const newIdx = Math.max(0, idx - 1)
        activeRequestId.value = openTabs.value[newIdx].requestId
      }
    }
  }

  /** 关闭除指定 Tab 以外的所有 Tab（批量，调用方负责 dirty 确认） */
  function closeOtherTabs(keepRequestId: number) {
    openTabs.value = openTabs.value.filter(t => t.requestId === keepRequestId)
    activeRequestId.value = keepRequestId
  }

  /** 关闭指定 Tab 左侧所有 Tab */
  function closeLeftTabs(requestId: number) {
    const idx = openTabs.value.findIndex(t => t.requestId === requestId)
    if (idx <= 0) return
    openTabs.value.splice(0, idx)
    // 若激活 Tab 在被关闭的范围内，切换到 requestId
    const stillActive = openTabs.value.some(t => t.requestId === activeRequestId.value)
    if (!stillActive) activeRequestId.value = requestId
  }

  /** 关闭指定 Tab 右侧所有 Tab */
  function closeRightTabs(requestId: number) {
    const idx = openTabs.value.findIndex(t => t.requestId === requestId)
    if (idx === -1 || idx === openTabs.value.length - 1) return
    openTabs.value.splice(idx + 1)
    const stillActive = openTabs.value.some(t => t.requestId === activeRequestId.value)
    if (!stillActive) activeRequestId.value = requestId
  }

  /** 更新 Tab 标题（接口重命名后调用） */
  function updateTabTitle(requestId: number, method: string, name: string) {
    const tab = openTabs.value.find(t => t.requestId === requestId)
    if (tab) tab.title = `${method} ${name}`
  }

  // ── 持久化（Tauri plugin-store）───────────────────────────

  let _store: Awaited<ReturnType<typeof load>> | null = null

  async function getStore() {
    if (!_store) {
      _store = await load('tab-state.json', { autoSave: true, defaults: {} })
    }
    return _store
  }

  /** 保存当前项目的 Tab 状态到 store 文件 */
  async function saveState(projectId: number) {
    const store = await getStore()
    await store.set(`tabs_project_${projectId}`, {
      openIds: openTabs.value.map(t => t.requestId),
      activeId: activeRequestId.value,
    })
  }

  /**
   * 从 store 文件恢复指定项目的 Tab 状态
   * @param projectId 目标项目 ID
   * @param requestMap 当前项目所有接口（用于校验 ID 有效性和获取标题）
   */
  async function restoreState(
    projectId: number,
    requestMap: Record<number, ApiRequest[]>,
  ) {
    const store = await getStore()
    const saved = await store.get<{ openIds: number[]; activeId: number | null }>(
      `tabs_project_${projectId}`,
    )

    // 拍平所有接口，便于按 ID 查找
    const allRequests: Record<number, ApiRequest> = {}
    for (const reqs of Object.values(requestMap)) {
      for (const r of reqs) allRequests[r.id] = r
    }

    if (!saved) {
      openTabs.value = []
      activeRequestId.value = null
      return
    }

    // 过滤掉已被删除的接口 ID
    const validTabs: TabItem[] = []
    for (const id of saved.openIds) {
      const req = allRequests[id]
      if (req) {
        validTabs.push({ requestId: id, title: `${req.method} ${req.name}` })
      }
    }

    openTabs.value = validTabs
    const activeStillValid = validTabs.some(t => t.requestId === saved.activeId)
    activeRequestId.value = activeStillValid
      ? saved.activeId
      : (validTabs[0]?.requestId ?? null)
  }

  /** 清空当前 Tab 列表（切换项目前调用） */
  function clearTabs() {
    openTabs.value = []
    activeRequestId.value = null
  }

  return {
    openTabs,
    activeRequestId,
    activeTab,
    isOpen,
    openTab,
    activateTab,
    closeTab,
    closeOtherTabs,
    closeLeftTabs,
    closeRightTabs,
    updateTabTitle,
    saveState,
    restoreState,
    clearTabs,
  }
})
