<template>
  <!-- 解析失败 或 用户切到 raw 模式 → 显示原始文本 -->
  <pre v-if="viewMode === 'raw' || parsedJson === null" class="json-content"><code>{{ body }}</code></pre>
  <!-- 美化模式：vue-json-pretty 折叠树 -->
  <div v-else class="json-tree-wrapper">
    <VueJsonPretty
      :data="parsedJson"
      :deep="expandLevel"
      :show-length="true"
      :show-line="true"
      :collapsed-on-click-brackets="true"
      :show-icon="true"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useMessage } from 'naive-ui'
import VueJsonPretty from 'vue-json-pretty'
import 'vue-json-pretty/lib/styles.css'
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
 * Ctrl+F 逻辑由本组件负责（原在 JsonViewer 顶层，现下沉到此）：
 *   - 第一次按 Ctrl+F：全展开所有节点 / 或切 raw（fallback）
 *   - 第二次按 Ctrl+F：不劫持，由浏览器原生搜索接管
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

// vue-json-pretty 的 :deep prop —— 控制初始渲染的展开层级
// 运行时改成 999 触发全展开（该组件没 expose expandAll 方法）
const expandLevel = ref(3)

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
  if (!props.isHovering) return              // 鼠标不在响应区
  if (props.viewMode === 'raw') return       // 已是原始模式，原生搜索即可
  if (parsedJson.value === null) return      // JSON 解析失败，走 <pre>，不劫持

  e.preventDefault()

  // 大响应体：全展开代价过高，直接 fallback 到 raw
  if (props.body.length > LARGE_RESPONSE_THRESHOLD) {
    emit('fallback-to-raw')
    message.info('响应体较大，已切换为原始模式，再次按 Ctrl+F 可搜索')
    return
  }

  // 强制重新展开所有节点（处理用户手动收起后再 Ctrl+F 的场景）
  expandLevel.value = 3
  nextTick(() => {
    expandLevel.value = 999
    nextTick(() => {
      // 展开完成后，主动调 window.find() 弹出搜索 UI
      if (typeof (window as any).find === 'function') {
        const selection = window.getSelection()
        const selectedText = selection?.toString() || ''
        ;(window as any).find(selectedText || '')
      } else {
        emit('fallback-to-raw')
        message.info('已切换为原始模式，现在可按 Ctrl+F 搜索')
      }
    })
  })
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown, true)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown, true)
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

/* vue-json-pretty 折叠树容器 */
.json-tree-wrapper {
  flex: 1;
  overflow: auto;
  padding: 8px;
  background: var(--bg-surface);
  border-radius: 4px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
  font-size: 12.5px;
  line-height: 1.6;
}

/* 覆盖 vue-json-pretty 默认主题色，与 Naive UI 融合 */
:deep(.vjs-tree) {
  font-size: 12.5px !important;
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace !important;
}
:deep(.vjs-tree .vjs-key)           { color: var(--json-key); }
:deep(.vjs-tree .vjs-value-string)  { color: var(--json-string); }
:deep(.vjs-tree .vjs-value-number)  { color: var(--json-number); }
:deep(.vjs-tree .vjs-value-boolean) { color: var(--json-boolean); }
:deep(.vjs-tree .vjs-value-null)    { color: var(--json-null); }
</style>
