<template>
  <!-- 根容器：承载自实现搜索高亮作用域 -->
  <div ref="rootEl" class="json-renderer-root">
    <!-- 1.0.4：Linux 无原生 find 时的兜底搜索条 -->
    <SearchBar
      v-if="!nativeFindAvailable"
      v-model:open="searchOpen"
      v-model:keyword="searchKeyword"
      :count-text="searchCountText"
      @go-prev="search.prev()"
      @go-next="search.next()"
      @close="closeSearch"
      class="text-search-slot"
    />
    <!-- 解析失败 或 用户切到 raw 模式 → 显示原始文本 -->
    <pre v-if="viewMode === 'raw' || parsedJson === null" class="json-content"><code>{{ body }}</code></pre>
    <!-- 美化模式：自研折叠树（字段命中响应字典时值旁挂字典色微标签） -->
    <div v-else class="json-tree-wrapper">
      <JsonTree :data="parsedJson" :deep="expandLevel" :decorate="decorateField" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useMessage } from 'naive-ui'
import { useDictionaryStore } from '../../../stores/dictionary'
import { useRequestStore } from '../../../stores/request'
import { matchDictMarker, type DictMarker } from './jsonTreeUtils'
import JsonTree from './JsonTree.vue'
import SearchBar from '../SearchBar.vue'
import { useTextSearch } from '../../../composables/useTextSearch'
import type { ViewMode } from '../useResponseFormat'

/**
 * JSON 渲染器
 *
 * 职责：
 *   - pretty 模式：vue-json-pretty 折叠树（默认展开 3 层）
 *   - raw 模式：<pre> 原始文本
 *   - JSON 解析失败时强制走 raw
 *   - 处理 Ctrl+F 搜索（对应 plan: docs/1.0.2/plans/2026-05-12-response-search.md）
 *
 * Ctrl+F 策略（1.0.4 按平台分流）：
 *   - 浏览器原生 find 可用（Windows/macOS）：保持原两段式
 *     · 首次：全展开 JSON 树；再次：切 raw 交给浏览器原生搜索
 *   - 原生 find 不可用（Linux webkit2gtk 无 Find-in-Page UI）：
 *     直接弹出自实现搜索条（SearchBar + useTextSearch 高亮），
 *     不再依赖浏览器原生搜索（那在 Linux 上什么都不做）
 */

const props = defineProps<{
  body: string
  viewMode: ViewMode  // 'raw' | 'pretty'
  /** 是否在响应区域内（由父组件维护鼠标悬停状态传入） */
  isHovering: boolean
}>()

const emit = defineEmits<{
  /** 当需要 fallback 到 raw 模式时通知父组件同步 viewMode */
  'fallback-to-raw': []
}>()

const message = useMessage()

// ── 响应字段字典命中（1.0.5：复用现有字段绑定规则表，仅字段名精确匹配） ──
const dictStore = useDictionaryStore()
const requestStore = useRequestStore()

/** 字段值命中字典 item 时返回微标签（未绑定字典或未命中 → null 不渲染标记） */
function decorateField(key: string, value: unknown): DictMarker | null {
  return matchDictMarker(
    k => dictStore.dictIdForField(k, requestStore.activeRequestId),
    id => {
      const d = dictStore.dictById(id)
      return d ? `${d.code}（${d.name}）` : `#${id}`
    },
    id => dictStore.itemsMap[id] ?? [],
    key,
    value,
  )
}

// vue-json-pretty 的 :deep prop —— 控制初始渲染的展开层级
// 运行时改成 999 触发全展开（该组件没 expose expandAll 方法）
const expandLevel = ref(3)

// 1.0.4：原生 find 探测 —— Linux webkit2gtk 不实现 window.find
const nativeFindAvailable = typeof (window as any).find === 'function'

// 1.0.4：自实现搜索（Linux 兜底）
const rootEl = ref<HTMLElement | null>(null)
const searchOpen = ref(false)
const searchKeyword = ref('')
const search = useTextSearch(rootEl)
// 桥接给模板（composable 返回的 ref 嵌套不解包，需显式取 .value）
const searchCountText = computed(() =>
  search.matchCount.value ? `${search.currentMatch.value}/${search.matchCount.value}` : ''
)

// 大响应体阈值（500KB，按原 body 字符长度近似判断）
// 超过此值 Ctrl+F 时直接 fallback 到 raw 模式，避免全展开导致 Vue 重渲染卡顿
const LARGE_RESPONSE_THRESHOLD = 500_000

/** 解析 JSON，失败返回 null（上层模板据此走 raw 分支） */
const parsedJson = computed(() => {
  try {
    return JSON.parse(props.body)
  } catch {
    return null
  }
})

/**
 * Ctrl+F / Cmd+F 劫持处理
 * 仅在鼠标悬停在响应区、当前为 JSON 美化模式、JSON 可解析时生效
 */
function handleKeydown(e: KeyboardEvent) {
  const isFindKey = (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f' && !e.shiftKey
  if (!isFindKey) return
  if (!props.isHovering) return              // 其他面板：放行给原生搜索
  if (props.viewMode === 'raw') return       // 已 raw，原生搜索可用（或 SearchBar 高亮由 raw 分支处理）
  if (parsedJson.value === null) return      // JSON 解析失败，走 <pre>，不劫持

  e.preventDefault()

  // 1.0.4：Linux（无原生 find）→ 直接打开自实现搜索条并全展开，保证可搜字段名
  if (!nativeFindAvailable) {
    if (expandLevel.value < 999) {
      expandLevel.value = 999
    }
    openSearch()
    return
  }

  // 平台有原生 find：保持原两段式（原逻辑）
  // 大响应体（>500KB）：全展开代价过高，直接切 raw
  if (props.body.length > LARGE_RESPONSE_THRESHOLD) {
    emit('fallback-to-raw')
    message.info('响应体较大，已切换为原始模式，再次 Ctrl+F 可搜索')
    return
  }

  // 已展开或已提示过 → 切 raw，让浏览器在 <pre> 上原生搜索
  if (expandLevel.value >= 999) {
    emit('fallback-to-raw')
    message.info('已切换为原始模式，现在可用 Ctrl+F 搜索内容')
    return
  }

  // 首次 Ctrl+F：全展开所有节点
  expandLevel.value = 999
  nextTick(() => {
    message.info('已展开所有节点，再次 Ctrl+F 切换为原始模式并搜索')
  })
}

// ── 1.0.4：自实现搜索（Linux 兜底）──────────────────────────
function openSearch() {
  searchOpen.value = true
  // 打开后立即对当前关键词执行一次搜索（可能已有历史词）
  nextTick(() => {
    if (searchKeyword.value) search.search(searchKeyword.value)
  })
}

function closeSearch() {
  searchOpen.value = false
  search.clear()
  searchKeyword.value = ''
}

// 关键词变化时实时搜索
watch(searchKeyword, (kw) => {
  if (!searchOpen.value) return
  search.search(kw)
})

onMounted(() => {
  window.addEventListener('keydown', handleKeydown, true)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown, true)
  search.clear()
})
</script>

<style scoped>
/* 原始文本区域 */
.json-content {
  flex: 1;
  overflow: auto;
  margin: 0;
  padding: 8px;
  background: var(--bg-surface);
  border-radius: 4px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
  font-size: 12.5px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--text-primary);
}

/* 自研 JSON 树容器（树内字号/主题色在 JsonTree.vue 内管理） */
.json-tree-wrapper {
  flex: 1;
  overflow: auto;
  padding: 8px;
  background: var(--bg-surface);
  border-radius: 4px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
  font-size: 12.5px;
  line-height: 1.7;
}

/* 1.0.4：自实现搜索条定位 + 高亮标记 */
.json-renderer-root {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.text-search-slot {
  position: absolute;
  top: 4px;
  right: 8px;
  z-index: 20;
  background: var(--bg-elevated);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-md);
}
:deep(.text-search-hit) {
  background: var(--color-warning-soft, rgba(240, 160, 32, 0.3));
  color: inherit;
  border-radius: 2px;
  padding: 0 1px;
}
:deep(.text-search-hit.current) {
  background: var(--color-warning, #f0a020);
  color: #fff;
}
</style>
