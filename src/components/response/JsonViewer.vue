<template>
  <div class="json-viewer">
    <!-- 截断警告条 -->
    <div v-if="isTruncated" class="truncated-warning">
      ⚠️ 响应体过大，仅显示前 2MB
    </div>

    <!-- 操作栏 -->
    <div class="json-toolbar">
      <!-- 内容类型标签 -->
      <n-tag v-if="contentTypeLabel" size="tiny" :bordered="false" style="opacity:0.7">
        {{ contentTypeLabel }}
      </n-tag>
      <n-button size="tiny" quaternary @click="copyContent">
        📋 复制
      </n-button>
      <!-- JSON 模式：美化/原始切换；非 JSON 无此按钮 -->
      <n-button v-if="isJsonContent" size="tiny" quaternary @click="toggleFormat">
        {{ isRaw ? '美化/折叠' : '原始' }}
      </n-button>
      <!-- HTML/XML：格式化按钮 -->
      <n-button v-if="isHtmlOrXml" size="tiny" quaternary @click="toggleFormat">
        {{ isRaw ? '语法高亮' : '原始' }}
      </n-button>
    </div>

    <!-- JSON 内容：原始模式用 <pre>，美化模式用 vue-json-pretty 折叠树 -->
    <template v-if="isJsonContent">
      <pre v-if="isRaw || parsedJson === null" class="json-content"><code>{{ body }}</code></pre>
      <div v-else class="json-tree-wrapper">
        <VueJsonPretty
          :data="parsedJson"
          :deep="3"
          :show-length="true"
          :show-line="true"
          :collapsed-on-click-brackets="true"
          :show-icon="true"
        />
      </div>
    </template>

    <!-- HTML/XML：highlight.js 语法高亮 -->
    <pre
      v-else-if="isHtmlOrXml"
      class="json-content hljs"
    ><code v-html="highlightedCode" /></pre>

    <!-- 其他：纯文本 -->
    <pre v-else class="json-content"><code>{{ body }}</code></pre>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NButton, NTag, useMessage } from 'naive-ui'
import VueJsonPretty from 'vue-json-pretty'
import 'vue-json-pretty/lib/styles.css'
import hljs from 'highlight.js/lib/core'
import xml from 'highlight.js/lib/languages/xml'  // covers html + xml
import 'highlight.js/styles/github.css'

// 仅注册需要的语言，避免引入整个 hljs 包
hljs.registerLanguage('xml', xml)

const props = defineProps<{
  body: string
  contentType?: string  // 来自响应头 Content-Type
  isTruncated?: boolean
}>()

const message = useMessage()
const isRaw = ref(false)

const isJsonContent = computed(() => {
  const ct = props.contentType ?? ''
  const trimmed = props.body.trimStart()
  return ct.includes('json') || trimmed.startsWith('{') || trimmed.startsWith('[')
})

const isHtmlOrXml = computed(() => {
  const ct = props.contentType ?? ''
  return ct.includes('html') || ct.includes('xml') || props.body.trimStart().startsWith('<')
})

/** 显示在工具栏的内容类型简短标签 */
const contentTypeLabel = computed(() => {
  const ct = props.contentType ?? ''
  if (ct.includes('json')) return 'JSON'
  if (ct.includes('html')) return 'HTML'
  if (ct.includes('xml')) return 'XML'
  if (ct.includes('text/plain')) return 'TEXT'
  if (ct.includes('text/')) return 'TEXT'
  if (ct) return ct.split(';')[0].trim()
  if (props.body.trimStart().startsWith('{') || props.body.trimStart().startsWith('[')) return 'JSON'
  if (props.body.trimStart().startsWith('<')) return 'HTML/XML'
  return ''
})

/** 解析 JSON，解析失败返回 null */
const parsedJson = computed(() => {
  try {
    return JSON.parse(props.body)
  } catch {
    return null
  }
})

/** highlight.js 高亮 HTML/XML 内容，返回带 span 的 HTML 字符串 */
const highlightedCode = computed(() => {
  if (!props.body) return ''
  try {
    return hljs.highlight(props.body, { language: 'xml' }).value
  } catch {
    return escapeHtml(props.body)
  }
})

function toggleFormat() {
  isRaw.value = !isRaw.value
}

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

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
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
  background: #fff7e6;
  border: 1px solid #ffd591;
  border-radius: 4px;
  padding: 4px 10px;
  font-size: 12px;
  color: #d46b08;
  margin-bottom: 6px;
  flex-shrink: 0;
}

.json-toolbar {
  display: flex;
  gap: 4px;
  margin-bottom: 4px;
  flex-shrink: 0;
}

/* 原始文本 / highlight.js 区域 */
.json-content {
  flex: 1;
  overflow: auto;
  margin: 0;
  padding: 8px;
  background: var(--n-color-embedded, #f9f9f9);
  border-radius: 4px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
  font-size: 12.5px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--n-text-color-1, #333);
}

/* vue-json-pretty 折叠树容器 */
.json-tree-wrapper {
  flex: 1;
  overflow: auto;
  padding: 8px;
  background: var(--n-color-embedded, #f9f9f9);
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
:deep(.vjs-tree .vjs-key) { color: #0550ae; }
:deep(.vjs-tree .vjs-value-string) { color: #0a3069; }
:deep(.vjs-tree .vjs-value-number) { color: #0550ae; }
:deep(.vjs-tree .vjs-value-boolean) { color: #8250df; }
:deep(.vjs-tree .vjs-value-null) { color: #999; }
</style>
