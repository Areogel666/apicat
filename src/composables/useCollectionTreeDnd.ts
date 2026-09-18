import { invoke } from '@tauri-apps/api/core'
import { useMessage } from 'naive-ui'
import type { TreeOption, TreeDropInfo } from 'naive-ui'
import type { Ref } from 'vue'
import { useCollectionStore } from '../stores/collection'
import { useRequestStore } from '../stores/request'

/**
 * 接口树的拖拽排序 / 跨目录移动。
 *
 * 从 Sidebar.vue 抽出：这段逻辑只依赖 store 与 IPC，与模板零耦合，
 * 独立出来后拖拽回归可单独验证。
 *
 * ⚠️ allowDrop 的回调参数**不要**解构 dragNode —— Naive UI 的 NTree
 * AllowDrop 类型不含该字段（见根 AGENTS.md），其余规则在 onDrop 兜底。
 */
export function useCollectionTreeDnd(currentProjectId: Ref<number | null>) {
  const collectionStore = useCollectionStore()
  const requestStore = useRequestStore()
  const message = useMessage()

  /** 从 node key 解析类型和 ID（约定："col-{id}" / "req-{id}"） */
  function parseNodeKey(key: string): { type: 'req' | 'col'; id: number } {
    if (key.startsWith('req-')) return { type: 'req', id: parseInt(key.slice(4)) }
    return { type: 'col', id: parseInt(key.slice(4)) }
  }

  /** 查找接口所属的 collection ID */
  function findRequestOwner(reqId: number): number | null {
    for (const [colId, reqs] of Object.entries(requestStore.requestMap)) {
      if (reqs.some(r => r.id === reqId)) return parseInt(colId)
    }
    return null
  }

  /** 在数组中将 dragIdx 处元素移到 targetIdx 的 before/after 位置，返回新数组 */
  function reorderArray<T>(items: T[], dragIdx: number, targetIdx: number, position: 'before' | 'after'): T[] {
    const arr = [...items]
    const [moved] = arr.splice(dragIdx, 1)
    const insertAt = position === 'before' ? targetIdx : targetIdx + 1
    // splice 后 targetIdx 可能偏移：拖拽源在目标前面时需要 -1
    arr.splice(insertAt > dragIdx ? insertAt - 1 : insertAt, 0, moved)
    return arr
  }

  /**
   * 判断 candidateId 是否是 ancestorId 的后代（BFS）
   * 用于 onDrop 中防止目录循环引用
   */
  function isDescendant(ancestorId: number, candidateId: number): boolean {
    const pid = currentProjectId.value
    if (!pid) return false
    const cols = collectionStore.getCollections(pid)
    const visited = new Set<number>()
    const queue = [ancestorId]
    while (queue.length) {
      const cur = queue.shift()!
      if (visited.has(cur)) continue
      visited.add(cur)
      for (const c of cols) {
        if (c.parent_id === cur) {
          if (c.id === candidateId) return true
          queue.push(c.id)
        }
      }
    }
    return false
  }

  /**
   * allowDrop：仅基于目标节点做静态判断，不依赖 dragNode。
   * Naive UI NTree AllowDrop 回调参数不含 dragNode，其余规则在 onDrop 中兜底。
   */
  function allowDrop({ dropPosition, node }: { dropPosition: string; node: TreeOption }) {
    const targetKey = node.key as string
    // 接口节点不能作为 inside 的放置目标（接口不是容器）
    if (targetKey.startsWith('req-') && dropPosition === 'inside') return false
    return true
  }

  async function onDrop(info: TreeDropInfo) {
    const { node, dragNode, dropPosition } = info
    const pid = currentProjectId.value
    if (!pid) return

    const drag = parseNodeKey(dragNode.key as string)
    const target = parseNodeKey(node.key as string)
    if (drag.type === target.type && drag.id === target.id) return

    // ── 接口拖拽 ─────────────────────────────────────────────
    if (drag.type === 'req') {
      const srcColId = findRequestOwner(drag.id)
      if (srcColId === null) return

      // 目标 collection ID：拖到接口上 → 接口所在目录；拖到目录上 → 该目录
      let dstColId: number
      let insertIdx: number  // 在目标列表中的插入位置

      if (target.type === 'req') {
        // 接口 → 接口：插入到目标接口的 before/after
        const ownerColId = findRequestOwner(target.id)
        if (ownerColId === null) return
        dstColId = ownerColId
        const dstItems = requestStore.requestMap[dstColId] ?? []
        const targetIdx = dstItems.findIndex(r => r.id === target.id)
        insertIdx = dropPosition === 'before' ? targetIdx : targetIdx + 1
      } else {
        // 接口 → 目录
        if (dropPosition === 'before' || dropPosition === 'after') return // 无语义
        // inside：移入目录末尾
        dstColId = target.id
        insertIdx = (requestStore.requestMap[dstColId] ?? []).length
      }

      if (srcColId === dstColId && target.type === 'col') return // 已在该目录

      if (srcColId === dstColId) {
        // 同 collection 内重排
        const items = [...(requestStore.requestMap[srcColId] ?? [])]
        const dragIdx = items.findIndex(r => r.id === drag.id)
        const targetIdx = items.findIndex(r => r.id === target.id)
        const sorted = reorderArray(items, dragIdx, targetIdx, dropPosition as 'before' | 'after')
          .map((r, i) => ({ ...r, sort_order: i }))

        requestStore.requestMap[srcColId] = sorted
        try {
          await invoke('update_request_sort', {
            items: sorted.map(r => [r.id, r.sort_order] as [number, number]),
          })
        } catch (e) {
          message.error('排序保存失败: ' + String(e))
          await collectionStore.loadCollections(pid)
        }
      } else {
        // 跨 collection 移动
        const srcItems = [...(requestStore.requestMap[srcColId] ?? [])]
        const dstItems = [...(requestStore.requestMap[dstColId] ?? [])]
        const srcSnap = [...srcItems], dstSnap = [...dstItems]

        const dragIdx = srcItems.findIndex(r => r.id === drag.id)
        const [moved] = srcItems.splice(dragIdx, 1)
        dstItems.splice(insertIdx, 0, moved)

        const sortedSrc = srcItems.map((r, i) => ({ ...r, sort_order: i }))
        const sortedDst = dstItems.map((r, i) => ({ ...r, sort_order: i }))

        requestStore.requestMap[srcColId] = sortedSrc
        requestStore.requestMap[dstColId] = sortedDst

        try {
          await invoke('move_request', { id: drag.id, newCollectionId: dstColId, sortOrder: insertIdx })
          await invoke('update_request_sort', {
            items: sortedDst.map(r => [r.id, r.sort_order] as [number, number]),
          })
          if (sortedSrc.length > 0) {
            await invoke('update_request_sort', {
              items: sortedSrc.map(r => [r.id, r.sort_order] as [number, number]),
            })
          }
        } catch (e) {
          message.error('移动失败: ' + String(e))
          requestStore.requestMap[srcColId] = srcSnap
          requestStore.requestMap[dstColId] = dstSnap
        }
      }
      return
    }

    // ── 目录拖拽（目录只能拖到目录上）──────────────────────────
    if (drag.type === 'col' && target.type === 'col') {
      if (dropPosition === 'inside' && isDescendant(drag.id, target.id)) return

      const allCols = [...collectionStore.getCollections(pid)]
      const colsSnap = [...allCols]
      const dragCol = allCols.find(c => c.id === drag.id)
      const targetCol = allCols.find(c => c.id === target.id)
      if (!dragCol || !targetCol) return

      if (dropPosition === 'inside') {
        // 移入目标目录成为子目录（追加到末尾）
        const newSortOrder = allCols.filter(c => c.parent_id === target.id).length
        const idx = allCols.findIndex(c => c.id === drag.id)
        allCols[idx] = { ...dragCol, parent_id: target.id, sort_order: newSortOrder }
        collectionStore.collectionMap[pid] = allCols

        try {
          await invoke('move_collection', { id: drag.id, newParentId: target.id, sortOrder: newSortOrder })
        } catch (e) {
          message.error('移动失败: ' + String(e))
          collectionStore.collectionMap[pid] = colsSnap
        }
      } else {
        // before / after：移到与目标同一层级
        const newParentId = targetCol.parent_id
        const isSameLevel = dragCol.parent_id === newParentId

        // 取同层兄弟（排除被拖拽的目录自身）
        const siblings = allCols.filter(c => c.parent_id === newParentId && c.id !== drag.id)
        const targetIdx = siblings.findIndex(c => c.id === target.id)
        const insertAt = dropPosition === 'before' ? targetIdx : targetIdx + 1
        siblings.splice(insertAt, 0, { ...dragCol, parent_id: newParentId })
        const sortedSiblings = siblings.map((c, i) => ({ ...c, sort_order: i }))

        // 写回 allCols
        for (const s of sortedSiblings) {
          const i = allCols.findIndex(c => c.id === s.id)
          if (i !== -1) allCols[i] = s
        }
        collectionStore.collectionMap[pid] = allCols

        try {
          if (isSameLevel) {
            // 同层重排只需更新排序
            await invoke('update_collection_sort', {
              items: sortedSiblings.map(c => [c.id, c.sort_order] as [number, number]),
            })
          } else {
            // 跨层：先移动再更新排序
            await invoke('move_collection', { id: drag.id, newParentId: newParentId ?? null, sortOrder: insertAt })
            await invoke('update_collection_sort', {
              items: sortedSiblings.map(c => [c.id, c.sort_order] as [number, number]),
            })
          }
        } catch (e) {
          message.error('排序保存失败: ' + String(e))
          collectionStore.collectionMap[pid] = colsSnap
        }
      }
    }
  }

  return { allowDrop, onDrop, parseNodeKey }
}
