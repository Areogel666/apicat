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
          @click="showDiff = true"
        >
          Diff 选中两条 ({{ selectedIds.size }}/2)
        </n-button>
      </div>

      <!-- JSON Diff 弹窗 -->
      <n-modal v-model:show="showDiff" preset="card" title="JSON Diff" style="width: 90vw; max-width: 1000px">
        <div class="diff-container">
          <div class="diff-side">
            <div class="diff-label">{{ diffPair[0]?.created_at ?? '' }}</div>
            <pre class="diff-content">{{ prettyBody(diffPair[0]?.response_body) }}</pre>
          </div>
          <div class="diff-divider" />
          <div class="diff-side">
            <div class="diff-label">{{ diffPair[1]?.created_at ?? '' }}</div>
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
import type { HistoryRecord } from '../../types'

const props = defineProps<{
  records: HistoryRecord[]
}>()

const emit = defineEmits<{
  refill: [snapshot: string]  // 回填事件，传 request_snapshot JSON
}>()

const selectedIds = ref<Set<number>>(new Set())
const showDiff = ref(false)

const diffPair = computed<(HistoryRecord | undefined)[]>(() => {
  const ids = Array.from(selectedIds.value)
  return [
    props.records.find(r => r.id === ids[0]),
    props.records.find(r => r.id === ids[1]),
  ]
})

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

function refill(rec: HistoryRecord) {
  emit('refill', rec.request_snapshot)
}

function formatTime(iso: string): string {
  const d = new Date(iso)
  return `${d.getMonth() + 1}/${d.getDate()} ${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}`
}

function statusTagType(code: number | null): 'success' | 'error' | 'warning' | 'default' {
  if (!code) return 'default'
  if (code < 300) return 'success'
  if (code < 400) return 'warning'
  return 'error'
}

function prettyBody(body?: string): string {
  if (!body) return ''
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
  gap: 8px;
}

.history-list {
  flex: 1;
  overflow-y: auto;
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

.history-item:hover { background: var(--n-item-color-hover, rgba(0,0,0,0.05)); }
.history-item.is-selected { background: var(--n-item-color-active, rgba(24,160,88,0.08)); }

.history-time { color: var(--n-text-color-3, #999); font-size: 11px; min-width: 80px; }
.history-ms { color: var(--n-text-color-3, #999); font-size: 11px; min-width: 48px; text-align: right; }

.history-actions {
  padding: 4px 0;
  flex-shrink: 0;
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
  color: var(--n-text-color-3, #999);
  padding: 4px 8px;
  border-bottom: 1px solid var(--n-border-color, #e0e0e6);
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
  background: var(--n-border-color, #e0e0e6);
  flex-shrink: 0;
}
</style>
