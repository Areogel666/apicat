import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { HeaderTemplate } from '../types'
import { useProjectStore } from './project'

/**
 * 公共 Headers 模板 —— 按项目落库（表 header_templates）。
 *
 * 编辑期仍是内存态：add/update/remove 后防抖落盘，整份列表一次提交；
 * 切换项目前先 flush 旧项目，避免防抖窗口内的编辑写到新项目头上。
 */
export type HeaderTemplateItem = HeaderTemplate

/** 旧版（按项目落库之前）存 localStorage 的全局模板，首次加载空项目时迁入一次 */
const LEGACY_STORAGE_KEY = 'apicat_header_templates'
const SAVE_DEBOUNCE_MS = 400

export const useHeaderTemplateStore = defineStore('headerTemplate', () => {
  const items = ref<HeaderTemplateItem[]>([])
  const projectStore = useProjectStore()

  /** 当前 items 所属项目：切项目时 currentProjectId 已变，flush 必须用这个 */
  let loadedProjectId: number | null = null
  let saveTimer: ReturnType<typeof setTimeout> | null = null
  /** 快速连续切项目时丢弃旧响应（同 stats 的 seq 守卫） */
  let loadSeq = 0
  let nextLocalId = Date.now()

  async function load(projectId: number | null) {
    const seq = ++loadSeq
    loadedProjectId = projectId
    if (!projectId) {
      items.value = []
      return
    }
    try {
      const rows = await invoke<HeaderTemplateItem[]>('list_header_templates', { projectId })
      if (seq !== loadSeq) return
      items.value = rows
      await migrateLegacy(projectId)
    } catch (e) {
      console.error('[headerTemplate] 加载失败:', e)
      items.value = []
    }
  }

  /** 老版本 localStorage 全局模板 → 迁入首个加载到的项目，随后删除旧 key（只迁一次） */
  async function migrateLegacy(projectId: number) {
    const raw = localStorage.getItem(LEGACY_STORAGE_KEY)
    if (!raw || items.value.length > 0) return
    try {
      const parsed = JSON.parse(raw) as Array<Partial<HeaderTemplateItem>>
      localStorage.removeItem(LEGACY_STORAGE_KEY)
      if (!Array.isArray(parsed) || parsed.length === 0) return
      items.value = parsed.map(p => ({
        id: nextLocalId++,
        key: p.key ?? '',
        value: p.value ?? '',
        enabled: p.enabled ?? true,
        description: p.description ?? '',
      }))
      await saveNow(projectId)
    } catch {
      // 旧数据损坏：直接丢弃，不阻断加载
    }
  }

  function persist() {
    if (loadedProjectId === null) return
    if (saveTimer) clearTimeout(saveTimer)
    const projectId = loadedProjectId
    saveTimer = setTimeout(() => {
      saveTimer = null
      void saveNow(projectId)
    }, SAVE_DEBOUNCE_MS)
  }

  async function saveNow(projectId: number) {
    // 快照必须在 await 前取，避免与后续 load 的新项目数据串味
    const snapshot = items.value.map(i => ({
      key: i.key,
      value: i.value,
      enabled: i.enabled,
      description: i.description,
    }))
    try {
      await invoke('save_header_templates', { projectId, items: snapshot })
    } catch (e) {
      console.error('[headerTemplate] 保存失败:', e)
    }
  }

  /** 切项目前把防抖中未发的编辑立即写回旧项目 */
  function flush(projectId: number | null) {
    if (saveTimer) {
      clearTimeout(saveTimer)
      saveTimer = null
    }
    if (projectId !== null) void saveNow(projectId)
  }

  // 首次 projectId 就绪与每次切项目都会触发
  watch(
    () => projectStore.currentProjectId,
    (pid, oldPid) => {
      if (oldPid) flush(oldPid)
      void load(pid)
    },
    { immediate: true },
  )

  /** 新增一条模板 Header */
  function addItem(key = '', value = '', description = '') {
    items.value.push({
      id: nextLocalId++,
      key,
      value,
      enabled: true,
      description,
    })
    persist()
  }

  /** 更新某条 */
  function updateItem(id: number, patch: Partial<Omit<HeaderTemplateItem, 'id'>>) {
    const idx = items.value.findIndex(i => i.id === id)
    if (idx !== -1) {
      items.value[idx] = { ...items.value[idx], ...patch }
      persist()
    }
  }

  /** 删除某条 */
  function removeItem(id: number) {
    items.value = items.value.filter(i => i.id !== id)
    persist()
  }

  /** 返回所有启用的 Header 条目（用于「应用到编辑区」） */
  function getEnabledItems(): HeaderTemplateItem[] {
    return items.value.filter(i => i.enabled && i.key)
  }

  return {
    items,
    load,
    addItem,
    updateItem,
    removeItem,
    getEnabledItems,
  }
})
