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
          <n-button size="small" quaternary @click="openFile">打开文件</n-button>
          <n-button size="small" quaternary @click="revealFile">打开所在目录</n-button>
        </div>
      </header>
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
    </template>
  </div>
</template>

<script setup lang="ts">
import { NButton, useMessage } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'

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
