<template>
  <div class="response-panel">
    <!-- 状态栏 -->
    <div class="response-status-bar">
      <template v-if="responseStore.loading">
        <n-spin size="small" />
        <span class="status-text">发送中...</span>
      </template>
      <template v-else-if="responseStore.error">
        <span class="status-error">❌ {{ responseStore.error }}</span>
      </template>
      <template v-else-if="resp">
        <n-tag :type="statusTagType(resp.status_code)" size="small">
          {{ resp.status_code }} {{ resp.status_text }}
        </n-tag>
        <!-- 非 2xx 时明确标注请求失败 -->
        <n-tag v-if="resp.status_code >= 400" type="error" size="small" :bordered="false">
          ❌ 请求失败
        </n-tag>
        <span class="status-meta">{{ resp.elapsed_ms }}ms</span>
        <span class="status-meta">{{ formatSize(resp.body_size) }}</span>
      </template>
      <template v-else>
        <span class="status-placeholder">响应</span>
      </template>
    </div>

    <!-- 响应 Tabs -->
    <n-tabs type="line" size="small" class="response-tabs">
      <!-- Body Tab -->
      <n-tab-pane name="body" tab="Body">
        <div class="tab-content">
          <n-empty v-if="!resp && !responseStore.loading" description="发送请求后，响应内容将在这里显示" style="margin-top:40px" />
          <n-spin v-else-if="responseStore.loading" style="margin-top:40px; display:flex; justify-content:center" />
          <JsonViewer
            v-else-if="resp"
            :body="resp.body"
            :content-type="responseContentType"
            :is-truncated="resp.is_truncated"
          />
        </div>
      </n-tab-pane>

      <!-- Headers Tab -->
      <n-tab-pane name="headers" tab="Headers">
        <div class="tab-content">
          <n-empty v-if="!resp" description="暂无响应 Headers" size="small" style="margin-top:40px" />
          <div v-else class="headers-list">
            <div v-for="([k, v], idx) in resp.headers" :key="idx" class="header-row">
              <span class="header-key">{{ k }}</span>
              <span class="header-value">{{ v }}</span>
            </div>
          </div>
        </div>
      </n-tab-pane>

      <!-- History Tab -->
      <n-tab-pane name="history" :tab="`History${historyCount ? ` (${historyCount})` : ''}`">
        <div class="tab-content">
          <HistoryTab
            :records="historyRecords"
            @refill="onRefill"
          />
        </div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NTabs, NTabPane, NEmpty, NSpin, NTag } from 'naive-ui'
import { useResponseStore } from '../../stores/response'
import { useHistoryStore } from '../../stores/history'
import { useRequestStore } from '../../stores/request'
import JsonViewer from './JsonViewer.vue'
import HistoryTab from './HistoryTab.vue'

const emit = defineEmits<{
  refill: [snapshot: string]
}>()

const responseStore = useResponseStore()
const historyStore = useHistoryStore()
const requestStore = useRequestStore()

const resp = computed(() => responseStore.response)

// 从响应头中提取 Content-Type
const responseContentType = computed(() => {
  if (!resp.value) return ''
  const ct = resp.value.headers.find(([k]) => k.toLowerCase() === 'content-type')
  return ct ? ct[1] : ''
})

// 当前激活接口的历史记录
const historyRecords = computed(() => {
  const id = requestStore.activeRequestId
  return id ? historyStore.getHistory(id) : []
})

const historyCount = computed(() => historyRecords.value.length)

function statusTagType(code: number): 'success' | 'error' | 'warning' | 'default' {
  if (code < 300) return 'success'
  if (code < 400) return 'warning'
  return 'error'
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`
  return `${(bytes / 1024 / 1024).toFixed(1)}MB`
}

// 历史回填：向上传递给 MainPanel
function onRefill(snapshot: string) {
  emit('refill', snapshot)
}
</script>

<style scoped>
.response-panel {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  padding: 0 var(--spacing-md) var(--spacing-sm);
  min-height: 180px;
}

.response-status-bar {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm) 0 var(--spacing-xs);
  font-size: var(--font-size-base);
  flex-shrink: 0;
}

.status-text { color: var(--text-tertiary); }
.status-meta { color: var(--text-tertiary); font-size: var(--font-size-sm); }
.status-error { color: var(--color-error); font-size: var(--font-size-base); }
.status-placeholder { font-weight: 600; font-size: var(--font-size-base); color: var(--text-secondary); }

.response-tabs {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/* 强制 n-tabs 内部的 tab-pane wrapper 撑开并允许 overflow */
.response-tabs :deep(.n-tabs-pane-wrapper) {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.response-tabs :deep(.n-tab-pane) {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
}

.tab-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.headers-list { overflow-y: auto; padding: var(--spacing-xs) 0; }

.header-row {
  display: flex;
  gap: var(--spacing-sm);
  padding: var(--spacing-xs) 0;
  font-size: var(--font-size-base);
  border-bottom: 1px solid var(--border-base);
}

.header-key {
  font-weight: 600;
  color: var(--text-secondary);
  min-width: 180px;
  word-break: break-all;
}

.header-value {
  color: var(--text-primary);
  word-break: break-all;
}
</style>
