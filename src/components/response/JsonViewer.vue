<template>
  <div class="json-viewer">
    <!-- 截断警告条 -->
    <div v-if="isTruncated" class="truncated-warning">
      ⚠️ 响应体过大，仅显示前 2MB
    </div>

    <!-- 操作栏 -->
    <div class="json-toolbar">
      <n-button size="tiny" quaternary @click="copyContent">
        📋 复制
      </n-button>
      <n-button size="tiny" quaternary @click="toggleFormat">
        {{ isRaw ? '美化' : '原始' }}
      </n-button>
    </div>

    <!-- 内容区 -->
    <pre class="json-content" :class="contentClass"><code>{{ displayContent }}</code></pre>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NButton, useMessage } from 'naive-ui'

const props = defineProps<{
  body: string
  contentType?: string  // 来自响应头 Content-Type
  isTruncated?: boolean
}>()

const message = useMessage()
const isRaw = ref(false)

const isJsonContent = computed(() => {
  const ct = props.contentType ?? ''
  return ct.includes('json') || props.body.trimStart().startsWith('{') || props.body.trimStart().startsWith('[')
})

// 尝试解析并美化 JSON
const displayContent = computed(() => {
  if (isRaw.value) return props.body
  if (isJsonContent.value) {
    try {
      return JSON.stringify(JSON.parse(props.body), null, 2)
    } catch {
      return props.body
    }
  }
  return props.body
})

// CSS class for content type hint
const contentClass = computed(() => {
  if (isJsonContent.value) return 'lang-json'
  const ct = props.contentType ?? ''
  if (ct.includes('html')) return 'lang-html'
  if (ct.includes('xml')) return 'lang-xml'
  return 'lang-text'
})

function toggleFormat() {
  isRaw.value = !isRaw.value
}

async function copyContent() {
  try {
    await navigator.clipboard.writeText(displayContent.value)
    message.success('已复制到剪贴板')
  } catch {
    message.error('复制失败')
  }
}
</script>

<style scoped>
.json-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
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
</style>
