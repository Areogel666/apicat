<template>
  <div class="history-tab">
    <n-empty v-if="!records.length" description="暂无历史记录" size="small" style="margin-top: 32px" />

    <template v-else>
      <!-- 历史记录列表 -->
      <div class="history-list">
        <div
          v-for="rec in records"
          :key="rec.id"
          class="history-item"
          :class="{ 'is-selected': selectedIds.has(rec.id) }"
          @click="toggleSelect(rec)"
        >
          <n-checkbox
            :checked="selectedIds.has(rec.id)"
            @update:checked="() => toggleSelect(rec)"
            @click.stop
          />
          <span class="history-time">{{ formatTime(rec.created_at) }}</span>
          <n-tag
            size="small"
            :type="statusTagType(rec.status_code)"
            style="min-width: 44px; text-align: center"
          >
            {{ rec.status_code ?? '—' }}
          </n-tag>
          <span class="history-ms">{{ rec.response_time_ms ?? '—' }}ms</span>
          <n-button
            size="tiny"
            quaternary
            style="margin-left: auto"
            @click.stop="refill(rec)"
            title="回填参数到编辑区"
          >
            ↩
          </n-button>
        </div>
      </div>

      <!-- Diff 按钮（选中恰好 2 条时激活）-->
      <div class="history-actions">
        <n-button
          size="small"
          :disabled="selectedIds.size !== 2"
          :loading="diffLoading"
          @click="openDiff"
        >
          Diff 选中两条 ({{ selectedIds.size }}/2)
        </n-button>
      </div>

      <!-- JSON Diff 弹窗 -->
      <n-modal v-model:show="showDiff" preset="card" title="JSON Diff" style="width: 90vw; max-width: 1000px">
        <div class="diff-container">
          <div class="diff-side">
            <div class="diff-label">{{ formatTime(diffPair[0]?.created_at ?? '') }}</div>
            <pre class="diff-content">{{ prettyBody(diffPair[0]?.response_body) }}</pre>
          </div>
          <div class="diff-divider" />
          <div class="diff-side">
            <div class="diff-label">{{ formatTime(diffPair[1]?.created_at ?? '') }}</div>
            <pre class="diff-content">{{ prettyBody(diffPair[1]?.response_body) }}</pre>
          </div>
        </div>
        <template #footer>
          <n-button @click="showDiff = false">关闭</n-button>
        </template>
      </n-modal>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NEmpty, NCheckbox, NTag, NButton, NModal } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import type { HistoryRecord } from '../../types'

defineProps<{
  records: HistoryRecord[]
}>()

const emit = defineEmits<{
  refill: [snapshot: string]  // 回填事件，传 request_snapshot JSON
}>()

const selectedIds = ref<Set<number>>(new Set())
const showDiff = ref(false)
const diffLoading = ref(false)
// list_history 不带 response_body / request_snapshot（避免每次切 Tab 传 20 条完整响应体），
// diff 与回填时按 id 单条补拉并缓存
const fullRecords = ref<Map<number, HistoryRecord>>(new Map())

const diffPair = computed<(HistoryRecord | undefined)[]>(() => {
  const ids = Array.from(selectedIds.value)
  return [
    fullRecords.value.get(ids[0]),
    fullRecords.value.get(ids[1]),
  ]
})

async function ensureFullRecord(id: number): Promise<HistoryRecord> {
  const cached = fullRecords.value.get(id)
  if (cached && cached.response_body != null) return cached
  const full = await invoke<HistoryRecord>('get_history_record', { id })
  const m = new Map(fullRecords.value)
  m.set(id, full)
  fullRecords.value = m
  return full
}

async function openDiff() {
  if (selectedIds.value.size !== 2) return
  diffLoading.value = true
  try {
    await Promise.all(Array.from(selectedIds.value).map(ensureFullRecord))
    showDiff.value = true
  } finally {
    diffLoading.value = false
  }
}

function toggleSelect(rec: HistoryRecord) {
  const s = new Set(selectedIds.value)
  if (s.has(rec.id)) {
    s.delete(rec.id)
  } else {
    if (s.size >= 2) {
      // 超过 2 个时，移除最早加入的那个（Set 迭代顺序为插入顺序）
      s.delete(s.values().next().value as number)
    }
    s.add(rec.id)
  }
  selectedIds.value = s
}

async function refill(rec: HistoryRecord) {
  const full = await ensureFullRecord(rec.id)
  if (full.request_snapshot != null) emit('refill', full.request_snapshot)
}

function formatTime(iso: string): string {
  // SQLite 返回的格式是 "YYYY-MM-DD HH:MM:SS"（UTC，无时区标记）
  // new Date() 对无时区标记的字符串行为不一致，需统一加 Z 转为 UTC 解析
  let dateStr = iso.trim()
  // 如果是 SQLite 格式（含空格而无T），将空格替换为T并追加Z
  if (/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}/.test(dateStr)) {
    dateStr = dateStr.replace(' ', 'T') + 'Z'
  }
  const d = new Date(dateStr)
  if (isNaN(d.getTime())) return iso  // 解析失败降级显示原字符串
  return `${d.getMonth() + 1}/${d.getDate()} ${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}`
}

function statusTagType(code: number | null): 'success' | 'error' | 'warning' | 'default' {
  if (!code) return 'default'
  if (code < 300) return 'success'
  if (code < 400) return 'warning'
  return 'error'
}

function prettyBody(body?: string | null): string {
  if (!body) return ''
  // 大响应文件标记：显示提示而非内容
  if (body.startsWith('@file:')) {
    return '⚠️ 大响应已保存到文件系统，不支持 diff 查看\n请使用"打开文件位置"功能查看完整内容'
  }
  try {
    return JSON.stringify(JSON.parse(body), null, 2)
  } catch {
    return body
  }
}
</script>

<style scoped>
.history-tab {
  display: flex;
  flex-direction: column;
  height: 100%;
  gap: 6px;
  min-height: 0;
}

.history-list {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}

.history-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: background 0.1s;
}

.history-item:hover { background: var(--bg-hover); }
.history-item.is-selected { background: var(--bg-active); }

.history-time { color: var(--text-tertiary); font-size: 11px; min-width: 80px; }
.history-ms { color: var(--text-tertiary); font-size: 11px; min-width: 48px; text-align: right; }

.history-actions {
  padding: 4px 0 2px;
  flex-shrink: 0;   /* 始终显示，不被列表挤走 */
  border-top: 1px solid var(--border-base);
}

.diff-container {
  display: flex;
  gap: 0;
  height: 60vh;
  overflow: hidden;
}

.diff-side {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.diff-label {
  font-size: 11px;
  color: var(--text-tertiary);
  padding: 4px 8px;
  border-bottom: 1px solid var(--border-base);
  flex-shrink: 0;
}

.diff-content {
  flex: 1;
  overflow: auto;
  margin: 0;
  padding: 8px;
  font-family: monospace;
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
}

.diff-divider {
  width: 1px;
  background: var(--border-base);
  flex-shrink: 0;
}
</style>
