import { ref } from 'vue'
import type { Ref } from 'vue'

/**
 * 自实现文本搜索高亮（用于浏览器无原生 find 的环境，如 Linux webkit2gtk）。
 *
 * 思路：用 TreeWalker 遍历 root 下的所有文本节点，把匹配文本用 <mark> 包裹实现高亮。
 * - 匹配 0 个 → 返回 false，不破坏 DOM
 * - 支持跳转（next/prev/scrollIntoView），当前匹配标记 .current
 *
 * 注意：
 * - 只在文本节点上做替换，不破坏渲染器结构（vue-json-pretty 树 / <pre>）
 * - 高亮会临时改写 DOM，关闭时需调用 clear() 还原
 * - 大 DOM（>500KB 已由 JsonRenderer 切 raw 分支）性能可控
 */
export function useTextSearch(root: Ref<HTMLElement | null>) {
  const currentMatch = ref(0)
  const matchCount = ref(0)
  /** 当前高亮的 <mark> 元素列表（清空/定位用） */
  let marks: HTMLElement[] = []

  /** 搜索并高亮全部匹配，返回是否有命中 */
  function search(keyword: string): boolean {
    clear()
    const el = root.value
    if (!el || !keyword) {
      matchCount.value = 0
      currentMatch.value = 0
      return false
    }

    const lower = keyword.toLowerCase()
    const want = escapeRegExp(keyword)
    // 收集所有含关键字的文本节点
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT)
    const textNodes: Text[] = []
    let node = walker.nextNode() as Text | null
    while (node) {
      if (node.nodeValue?.toLowerCase().includes(lower)) textNodes.push(node)
      node = walker.nextNode() as Text | null
    }
    if (textNodes.length === 0) return false

    // 逐个节点拆分为「关键字段 + 普通段」，关键字包 <mark>
    for (const tn of textNodes) {
      const text = tn.nodeValue ?? ''
      const segs = text.split(new RegExp(`(${want})`, 'gi'))
      if (segs.length <= 1) continue
      const frag = document.createDocumentFragment()
      for (const seg of segs) {
        if (!seg) continue
        if (seg.toLowerCase() === lower) {
          const mark = document.createElement('mark')
          mark.textContent = seg
          mark.classList.add('text-search-hit')
          frag.appendChild(mark)
          marks.push(mark)
        } else {
          frag.appendChild(document.createTextNode(seg))
        }
      }
      tn.parentNode?.replaceChild(frag, tn)
    }

    matchCount.value = marks.length
    if (marks.length > 0) {
      currentMatch.value = 1
      scrollToCurrent()
    }
    return marks.length > 0
  }

  /** 下一个匹配 */
  function next() {
    if (marks.length === 0) return
    currentMatch.value = (currentMatch.value % marks.length) + 1
    scrollToCurrent()
  }

  /** 上一个匹配 */
  function prev() {
    if (marks.length === 0) return
    currentMatch.value = ((currentMatch.value - 2 + marks.length) % marks.length) + 1
    scrollToCurrent()
  }

  /** 移除全部高亮，恢复原文本 */
  function clear() {
    for (const m of marks) {
      const parent = m.parentNode
      if (!parent) continue
      parent.replaceChild(document.createTextNode(m.textContent ?? ''), m)
    }
    marks = []
    matchCount.value = 0
    currentMatch.value = 0
  }

  function scrollToCurrent() {
    const idx = currentMatch.value - 1
    const mark = marks[idx]
    if (!mark) return
    // 清除旧 focus，标亮当前
    marks.forEach(m => m.classList.remove('current'))
    mark.classList.add('current')
    mark.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  }

  return { currentMatch, matchCount, search, next, prev, clear }
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}