<template>
  <div class="docs-panel">
    <div v-if="!file" class="empty">
      <div class="empty-icon">📄</div>
      <div>从左侧选择一个文档文件查看详情</div>
    </div>
    <template v-else>
      <header class="panel-header">
        <span class="file-name">{{ file.name }}</span>
        <div class="actions">
          <n-button size="small" secondary type="primary" @click="openFile">打开文件</n-button>
          <n-button size="small" secondary type="primary" @click="revealFile">打开所在目录</n-button>
        </div>
      </header>

      <!-- Markdown 预览（主区域）-->
      <div class="preview-area">
        <n-spin :show="loading" size="medium" class="preview-spin">
          <div v-if="previewError" class="preview-error">
            <div class="preview-error-icon">⚠️</div>
            <div>{{ previewError }}</div>
          </div>
          <MarkdownRenderer v-else-if="content" :body="content" />
          <div v-else class="preview-hint">该文件为空</div>
        </n-spin>
      </div>

      <!-- 元数据（默认折叠到底部）-->
      <n-collapse class="meta-collapse">
        <n-collapse-item title="文件信息" name="meta">
          <div class="meta-list">
            <div class="meta-row">
              <span class="meta-label">完整路径</span>
              <span class="meta-value path" :title="file.absolute_path">{{ file.absolute_path }}</span>
            </div>
            <div class="meta-row">
              <span class="meta-label">相对路径</span>
              <span class="meta-value">{{ file.relative_path }}</span>
            </div>
            <div class="meta-row">
              <span class="meta-label">大小</span>
              <span class="meta-value">{{ formatSize(file.size) }}</span>
            </div>
            <div class="meta-row">
              <span class="meta-label">修改时间</span>
              <span class="meta-value">{{ formatTime(file.modified_at) }}</span>
            </div>
          </div>
        </n-collapse-item>
      </n-collapse>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { NButton, NSpin, NCollapse, NCollapseItem, useMessage } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import MarkdownRenderer from '../response/formatters/MarkdownRenderer.vue'

interface DocFile {
  relative_path: string
  name: string
  dir: string
  size: number
  modified_at: string
  absolute_path: string
}

const props = defineProps<{
  file: DocFile | null
}>()

const message = useMessage()

// ── Markdown 预览 ─────────────────────────────────────────────
const content = ref('')
const loading = ref(false)
const previewError = ref('')

// 切换文件时异步读取内容。用请求序号防竞态：快速切换文件时
// 只有最后一次请求的结果生效，避免旧文件内容覆盖新文件。
let readSeq = 0
watch(
  () => props.file?.absolute_path,
  async (path) => {
    const seq = ++readSeq
    content.value = ''
    previewError.value = ''
    if (!path) return
    loading.value = true
    try {
      const text = await invoke<string>('read_doc_file', { path })
      if (seq !== readSeq) return  // 已有更新的请求，丢弃本次结果
      content.value = text
    } catch (e) {
      if (seq !== readSeq) return
      previewError.value = String(e)
    } finally {
      if (seq === readSeq) loading.value = false
    }
  },
  { immediate: true },
)

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function formatTime(iso: string): string {
  if (!iso) return '—'
  try {
    const d = new Date(iso)
    return d.toLocaleString('zh-CN', { hour12: false })
  } catch {
    return iso
  }
}

function openFile() {
  if (!props.file) return
  invoke('open_file_with_default', { path: props.file.absolute_path }).catch(e => {
    message.error(`打开文件失败：${e}`)
  })
}

function revealFile() {
  if (!props.file) return
  invoke('reveal_in_explorer', { path: props.file.absolute_path }).catch(e => {
    message.error(`打开资源管理器失败：${e}`)
  })
}
</script>

<style scoped>
.docs-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  background: var(--bg-elevated);
}

.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-tertiary);
  font-size: 13px;
}

.empty-icon {
  font-size: 32px;
  opacity: 0.5;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-base);
  flex-shrink: 0;
}

.file-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

/* Markdown 预览主区域 */
.preview-area {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.preview-spin {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.preview-hint {
  padding: 24px 16px;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 13px;
}

.preview-error {
  padding: 24px 16px;
  text-align: center;
  color: var(--color-error);
  font-size: 13px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.preview-error-icon {
  font-size: 28px;
}

/* 元数据折叠区：固定在底部，不参与滚动 */
.meta-collapse {
  flex-shrink: 0;
  border-top: 1px solid var(--border-base);
  max-height: 40%;
  overflow-y: auto;
}

.meta-list {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.meta-row {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.meta-label {
  font-size: 12px;
  color: var(--text-tertiary);
}

.meta-value {
  font-size: 13px;
  color: var(--text-primary);
  word-break: break-all;
}

.meta-value.path {
  font-family: monospace;
  font-size: 12px;
  background: var(--bg-hover);
  padding: 4px 8px;
  border-radius: 4px;
}
</style>
