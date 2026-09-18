<template>
  <n-modal v-model:show="show" preset="card" title="压测报告预览" style="width: 780px">
    <div v-if="loading" class="report-state"><n-spin size="small" /> 正在生成报告…</div>
    <div v-else-if="loadError" class="report-state report-state--error">
      ❌ 生成报告失败：{{ loadError }}
    </div>
    <div v-else-if="!markdown" class="report-state">无报告数据</div>
    <div v-else class="report-preview" v-html="previewHtml"></div>

    <template #footer>
      <div class="report-actions">
        <n-button size="small" :disabled="!canExport" @click="onCopy">复制 Markdown</n-button>
        <n-button size="small" :disabled="!canExport" @click="onSaveMd">导出 .md</n-button>
        <n-button size="small" type="primary" :disabled="!canExport" @click="onSaveHtml">导出 .html</n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NModal, NButton, NSpin, useMessage } from 'naive-ui'
import type { StressRun } from '../../types'
import {
  fetchReportMarkdown,
  renderReportHtml,
  wrapReportHtml,
  reportFileName,
  saveReportToFile,
} from './stressReport'

const show = defineModel<boolean>('show', { required: true })

const props = defineProps<{ run: StressRun | null }>()

const message = useMessage()

const markdown = ref('')
const loading = ref(false)
const loadError = ref('')

const previewHtml = computed(() => (markdown.value ? renderReportHtml(markdown.value) : ''))
const canExport = computed(() => Boolean(markdown.value) && !loading.value)

// 打开弹窗或切换记录时拉取报告内容（内容由 Rust 生成，与技能走 bridge 拿到的完全同源）
watch(
  () => [show.value, props.run?.id] as const,
  async ([visible]) => {
    if (!visible || !props.run) return
    loading.value = true
    loadError.value = ''
    markdown.value = ''
    try {
      markdown.value = await fetchReportMarkdown(props.run.id)
    } catch (e) {
      loadError.value = String(e)
    } finally {
      loading.value = false
    }
  },
  { immediate: true }
)

async function onCopy() {
  try {
    await navigator.clipboard.writeText(markdown.value)
    message.success('已复制 Markdown 到剪贴板')
  } catch {
    message.error('复制失败，请手动选中复制')
  }
}

async function onSaveMd() {
  if (!props.run) return
  const name = reportFileName(props.run, 'md')
  message.info(`正在保存 ${name} …`)
  if (await saveReportToFile(markdown.value, name, 'Markdown')) {
    message.success(`已保存 ${name}`)
  }
}

async function onSaveHtml() {
  if (!props.run) return
  const name = reportFileName(props.run, 'html')
  message.info(`正在保存 ${name} …`)
  const title = `${name} 压测报告`
  if (await saveReportToFile(wrapReportHtml(markdown.value, title), name, 'HTML')) {
    message.success(`已保存 ${name}`)
  }
}
</script>

<style scoped>
.report-state {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  color: var(--text-tertiary);
  font-size: var(--font-size-sm);
  padding: var(--spacing-md) 0;
}
.report-state--error { color: var(--color-error); }
.report-preview {
  max-height: 62vh;
  overflow-y: auto;
  padding-right: 4px;
}
/* v-html 内容不受 scoped 约束，需要用 :deep() */
.report-preview :deep(h1) {
  font-size: var(--font-size-lg);
  margin: 0 0 var(--spacing-sm);
}
.report-preview :deep(h2) {
  font-size: var(--font-size-base);
  margin: var(--spacing-md) 0 var(--spacing-xs);
  padding-bottom: 4px;
  border-bottom: 1px solid var(--border-base);
}
.report-preview :deep(table) {
  border-collapse: collapse;
  width: 100%;
  font-size: var(--font-size-sm);
  margin: var(--spacing-xs) 0;
}
.report-preview :deep(th),
.report-preview :deep(td) {
  border: 1px solid var(--border-base);
  padding: 4px 8px;
  text-align: left;
}
.report-preview :deep(th) { background: var(--bg-elevated); font-weight: 600; }
.report-preview :deep(blockquote) {
  margin: 0 0 var(--spacing-xs);
  padding: var(--spacing-xs) var(--spacing-sm);
  background: var(--bg-elevated);
  border-left: 3px solid var(--color-primary);
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
}
.report-preview :deep(blockquote p) { margin: 2px 0; }
.report-preview :deep(code) {
  background: var(--bg-hover);
  padding: 1px 5px;
  border-radius: var(--radius-sm);
  font-family: monospace;
  font-size: 12px;
}
.report-preview :deep(ul) { padding-left: 20px; margin: var(--spacing-xs) 0; }
.report-actions { display: flex; justify-content: flex-end; gap: var(--spacing-xs); }
</style>
