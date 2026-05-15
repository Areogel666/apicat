<template>
  <div
    ref="viewerEl"
    class="json-viewer"
    @mouseenter="isHovering = true"
    @mouseleave="isHovering = false"
  >
    <!-- 截断警告条 -->
    <div v-if="isTruncated" class="truncated-warning">
      ⚠️ 响应体过大，仅显示前 2MB
    </div>

    <!-- 工具栏：左=格式选择+复制，右=视图模式切换 -->
    <div class="json-toolbar">
      <FormatSelector
        :model-value="formatOverride"
        :detected="detectedFormat"
        @update:model-value="onFormatChange"
      />
      <n-button size="tiny" quaternary @click="copyContent">📋 复制</n-button>
      <div class="toolbar-spacer" />
      <ViewModeSwitch
        :model-value="viewMode"
        :format="effectiveFormat"
        @update:model-value="onViewModeChange"
      />
    </div>

    <!-- 渲染区：按格式路由到对应子组件 -->
    <!-- JSON -->
    <JsonRenderer
      v-if="effectiveFormat === 'json'"
      :body="body"
      :view-mode="viewMode"
      :is-hovering="isHovering"
      @fallback-to-raw="onFallbackToRaw"
    />
    <!-- Markdown: raw 走 CodeRenderer 高亮源码，preview 走 MarkdownRenderer -->
    <MarkdownRenderer
      v-else-if="effectiveFormat === 'markdown' && viewMode === 'preview'"
      :body="body"
    />
    <CodeRenderer
      v-else-if="effectiveFormat === 'markdown'"
      :body="body"
      language="markdown"
    />
    <!-- XML: pretty 传格式化后的 body，raw 传原文；两者都用 xml 语法高亮 -->
    <CodeRenderer
      v-else-if="effectiveFormat === 'xml'"
      :body="xmlDisplayBody"
      language="xml"
    />
    <!-- YAML: pretty = js-yaml dump 重排；两者都用 yaml 语法高亮 -->
    <CodeRenderer
      v-else-if="effectiveFormat === 'yaml'"
      :body="yamlDisplayBody"
      language="yaml"
    />
    <!-- HTML: preview 走 sandbox iframe；raw 走 xml 高亮（hljs 的 xml 语言覆盖 HTML） -->
    <HtmlPreviewRenderer
      v-else-if="effectiveFormat === 'html' && viewMode === 'preview'"
      :body="body"
    />
    <CodeRenderer
      v-else-if="effectiveFormat === 'html'"
      :body="body"
      language="xml"
    />
    <!-- Text: 纯文本 <pre>，无高亮 -->
    <PlainRenderer v-else-if="effectiveFormat === 'text'" :body="body" />
    <!-- 理论上所有格式都已覆盖；兜底走 PlainRenderer 防呆 -->
    <PlainRenderer v-else :body="body" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NButton, useMessage } from 'naive-ui'
import { useResponseStore, type FormatOverride } from '../../stores/response'
import {
  detectFormat,
  defaultViewMode,
  availableViewModes,
  formatXml,
  formatYaml,
  type ResponseFormat,
  type ViewMode,
} from './useResponseFormat'
import FormatSelector from './FormatSelector.vue'
import ViewModeSwitch from './ViewModeSwitch.vue'
import JsonRenderer from './formatters/JsonRenderer.vue'
import PlainRenderer from './formatters/PlainRenderer.vue'
import CodeRenderer from './formatters/CodeRenderer.vue'
import MarkdownRenderer from './formatters/MarkdownRenderer.vue'
import HtmlPreviewRenderer from './formatters/HtmlPreviewRenderer.vue'

/**
 * 响应体渲染路由器
 *
 * 原来这是一个 JSON 专属渲染器；M2 重构为"路由器"，根据格式分派到对应的子渲染器。
 * 文件名暂保留 JsonViewer，避免上游连锁改名。
 *
 * 职责：
 *   1. detectFormat 识别格式 + 合并用户手动覆盖 → effectiveFormat
 *   2. 根据 format + viewMode 路由到对应子组件
 *   3. 维护 isHovering（供 JsonRenderer 做 Ctrl+F 判断）
 *   4. 格式/视图模式按 requestId 隔离（写入 response store）
 */

const props = defineProps<{
  body: string
  contentType?: string
  isTruncated?: boolean
}>()

const message = useMessage()
const responseStore = useResponseStore()

// 根元素 ref + 鼠标悬停态（传给 JsonRenderer 做 Ctrl+F 范围判断）
const viewerEl = ref<HTMLElement | null>(null)
void viewerEl
const isHovering = ref(false)

/** 根据 Content-Type 和 body 自动识别的格式 */
const detectedFormat = computed<ResponseFormat>(() =>
  detectFormat(props.contentType ?? '', props.body),
)

/** 当前 requestId（可能为 null，例如尚无激活接口） */
const activeId = computed(() => responseStore.activeRequestId)

/** 用户手动覆盖的格式（'auto' 表示跟随自动识别） */
const formatOverride = computed<FormatOverride>(() =>
  activeId.value != null ? responseStore.getFormatOverride(activeId.value) : 'auto',
)

/** 真实生效的格式：override !== 'auto' 时用覆盖值，否则用自动识别 */
const effectiveFormat = computed<ResponseFormat>(() =>
  formatOverride.value === 'auto' ? detectedFormat.value : formatOverride.value,
)

/** 当前视图模式：优先取 store 里用户选过的，否则用该格式的默认模式 */
const viewMode = computed<ViewMode>(() => {
  if (activeId.value == null) return defaultViewMode(effectiveFormat.value)
  const stored = responseStore.getViewMode(activeId.value)
  if (stored !== null && availableViewModes(effectiveFormat.value).includes(stored)) {
    return stored
  }
  return defaultViewMode(effectiveFormat.value)
})

/** 格式变更时重置 viewMode 为该格式的默认值（旧 mode 可能对新格式无意义） */
function onFormatChange(next: FormatOverride) {
  if (activeId.value == null) return
  responseStore.setFormatOverride(activeId.value, next)
  const newFormat: ResponseFormat =
    next === 'auto' ? detectedFormat.value : next
  responseStore.setViewMode(activeId.value, defaultViewMode(newFormat))
}

function onViewModeChange(next: ViewMode) {
  if (activeId.value == null) return
  responseStore.setViewMode(activeId.value, next)
}

/** JsonRenderer 触发 Ctrl+F fallback 时，同步切 raw */
function onFallbackToRaw() {
  if (activeId.value == null) return
  responseStore.setViewMode(activeId.value, 'raw')
}

/** XML/HTML pretty 模式下的格式化 body；raw 模式或非 XML 时返回原 body */
const xmlDisplayBody = computed(() => {
  if (effectiveFormat.value !== 'xml' && effectiveFormat.value !== 'html') return props.body
  if (viewMode.value !== 'pretty') return props.body
  return formatXml(props.body)
})

/** YAML pretty 模式下的格式化 body；raw 模式或非 YAML 时返回原 body */
const yamlDisplayBody = computed(() => {
  if (effectiveFormat.value !== 'yaml') return props.body
  if (viewMode.value !== 'pretty') return props.body
  return formatYaml(props.body)
})

/**
 * 切换接口或响应内容变化时，若已存 viewMode 与新格式不兼容则清掉。
 * 防止"在 JSON 接口 A 选了 pretty，切到 HTML 接口 B 时 pretty 对 HTML 无效"之类的错位。
 */
watch([activeId, effectiveFormat], ([id, fmt]) => {
  if (id == null) return
  const stored = responseStore.getViewMode(id)
  if (stored !== null && !availableViewModes(fmt).includes(stored)) {
    responseStore.setViewMode(id, defaultViewMode(fmt))
  }
})

async function copyContent() {
  const text = props.body
  try {
    await navigator.clipboard.writeText(text)
    message.success('已复制到剪贴板')
  } catch {
    // 降级复制
    const ta = document.createElement('textarea')
    ta.value = text
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
    message.success('已复制到剪贴板')
  }
}
</script>

<style scoped>
.json-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.truncated-warning {
  /* 警告条：浅橙底 + 橙边 + 深橙文字（统一用 warning token） */
  background: rgba(240, 160, 32, 0.10);
  border: 1px solid var(--color-warning);
  border-radius: 4px;
  padding: 4px 10px;
  font-size: 12px;
  color: var(--color-warning);
  margin-bottom: 6px;
  flex-shrink: 0;
}

.json-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
  flex-shrink: 0;
}

.toolbar-spacer {
  flex: 1;
}
</style>
