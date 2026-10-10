import { onUnmounted, ref } from 'vue'
import type { Ref, ComputedRef } from 'vue'
import type { TreeOption, TreeDropInfo } from 'naive-ui'

/**
 * 接口树的 pointer 自研拖拽（替代 NTree 的 HTML5 draggable）。
 *
 * 背景：tauri.conf.json dragDropEnabled=true 后（文件拖拽走 Tauri 事件，见
 * MainPanel 注释），WebKit 在 GTK/WebView2 层拦截 HTML5 drag 事件——NTree 的
 * draggable 全平台失效（774f091 当年因 Windows WebView2 拦截而把 conf 设 false，
 * 现在被本方案反向取代：不依赖 HTML5 DnD，pointer 事件不受 dragDropEnabled 影响）。
 *
 * 职责：pointerdown/move/up 状态机 + elementFromPoint 反查落点 + 视觉反馈，
 * 产出与 NTree @drop 等价的 drop 信息（node/dragNode/dropPosition），交由调用方
 * onDrop 执行排序/移动，业务逻辑零改动复用 useCollectionTreeDnd。
 *
 * 落点判定复刻 NTree 语义：目录节点三等分（上 before / 中 inside / 下 after），
 * 接口节点两等分（不允许 inside 落点——与原 allowDrop 静态规则一致）。
 */
export function useTreePointerDnd(
  treeData: Ref<TreeOption[]> | ComputedRef<TreeOption[]>,
  onDrop: (info: Pick<TreeDropInfo, 'node' | 'dragNode' | 'dropPosition'>) => void | Promise<void>,
) {
  /** 拖拽中的节点 key（null = 未拖拽） */
  const draggingKey = ref<string | null>(null)

  let start: { x: number; y: number; option: TreeOption | null } = { x: 0, y: 0, option: null }
  let dragging = false
  let sourceEl: HTMLElement | null = null
  let hoverEl: HTMLElement | null = null
  let hoverKey: string | null = null
  let hoverPos: 'before' | 'after' | 'inside' | null = null
  let suppressNextClick = false

  // ── 工具 ──────────────────────────────────────────────────────

  function findOptionByKey(options: TreeOption[], key: string): TreeOption | null {
    for (const o of options) {
      if (o.key === key) return o
      const children = o.children as TreeOption[] | undefined
      if (children?.length) {
        const hit = findOptionByKey(children, key)
        if (hit) return hit
      }
    }
    return null
  }

  /**
   * 坐标 → 树节点。key 从 renderLabel 渲染的 .node-label[data-key] 反查——
   * 不能挂到 nodeProps：NTree 内部 Object.assign 会用写死的
   * `"data-key": undefined` 覆盖 nodeProps 的同名属性（naive-ui TreeNode.mjs）。
   * 行元素（.n-tree-node）提供 before/after/inside 三段判定的矩形。
   */
  function locateNodeAt(x: number, y: number): { row: HTMLElement; key: string } | null {
    const row = document.elementFromPoint(x, y)?.closest('.n-tree-node') as HTMLElement | null
    if (!row) return null
    const key = row.querySelector('.node-label')?.getAttribute('data-key')
    return key ? { row, key } : null
  }

  /** 目标节点矩形三段判定（col 三段，req 两段——接口不允许 inside 落点） */
  function positionFor(el: HTMLElement, y: number, targetIsReq: boolean): 'before' | 'after' | 'inside' {
    const r = el.getBoundingClientRect()
    if (targetIsReq) return y < r.top + r.height / 2 ? 'before' : 'after'
    const third = r.height / 3
    if (y < r.top + third) return 'before'
    if (y > r.bottom - third) return 'after'
    return 'inside'
  }

  function clearHover() {
    if (hoverEl) {
      hoverEl.classList.remove('ptd-drop-before', 'ptd-drop-after', 'ptd-drop-inside')
    }
    hoverEl = null
    hoverKey = null
    hoverPos = null
  }

  // ── 节点事件（renderLabel 的 VNode 上挂 onMousedown 调进来）────

  function onNodePointerdown(e: MouseEvent, option: TreeOption) {
    // 只响应左键；操作按钮（.node-actions）上按下不启动拖拽
    if (e.button !== 0) return
    if ((e.target as HTMLElement).closest('.node-actions')) return
    start = { x: e.clientX, y: e.clientY, option }
    dragging = false
    sourceEl = (e.currentTarget as HTMLElement).closest('.n-tree-node')
  }

  function onDocPointermove(e: PointerEvent) {
    if (!start.option) return
    if (!dragging) {
      if (Math.hypot(e.clientX - start.x, e.clientY - start.y) < 5) return
      dragging = true
      draggingKey.value = start.option.key as string
      sourceEl?.classList.add('ptd-dragging-src')
      document.body.classList.add('ptd-dragging')
    }
    e.preventDefault()
    const hit = locateNodeAt(e.clientX, e.clientY)
    if (!hit || hit.key === (start.option.key as string)) {
      clearHover()
      return
    }
    const target = findOptionByKey(treeData.value, hit.key)
    if (!target) {
      clearHover()
      return
    }
    const pos = positionFor(hit.row, e.clientY, hit.key.startsWith('req-'))
    hoverKey = hit.key
    hoverPos = pos
    const row = hit.row
    if (hoverEl !== row) {
      clearHover()
      hoverEl = row
      row.classList.add(`ptd-drop-${pos}`)
    } else {
      row.classList.remove('ptd-drop-before', 'ptd-drop-after', 'ptd-drop-inside')
      row.classList.add(`ptd-drop-${pos}`)
    }
  }

  function onDocPointerup() {
    if (!start.option) return
    const dragNode = start.option
    const wasDragging = dragging
    const tKey = hoverKey
    const tPos = hoverPos
    // 状态清理
    start = { x: 0, y: 0, option: null }
    dragging = false
    sourceEl?.classList.remove('ptd-dragging-src')
    document.body.classList.remove('ptd-dragging')
    clearHover()
    draggingKey.value = null
    if (!wasDragging || !tKey || !tPos) return // 普通 click，交回原生选择/展开

    suppressNextClick = true
    const target = findOptionByKey(treeData.value, tKey)
    if (!target) return
    void onDrop({ node: target, dragNode, dropPosition: tPos })
  }

  /** 拖拽结束后吞掉紧随的 click，防止误选中/误展开目标节点 */
  function onDocClickCapture(e: MouseEvent) {
    if (!suppressNextClick) return
    suppressNextClick = false
    e.preventDefault()
    e.stopPropagation()
  }

  document.addEventListener('pointermove', onDocPointermove)
  document.addEventListener('pointerup', onDocPointerup)
  document.addEventListener('click', onDocClickCapture, true)
  onUnmounted(() => {
    document.removeEventListener('pointermove', onDocPointermove)
    document.removeEventListener('pointerup', onDocPointerup)
    document.removeEventListener('click', onDocClickCapture, true)
  })

  return { onNodePointerdown, draggingKey }
}
