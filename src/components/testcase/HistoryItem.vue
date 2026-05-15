<template>
  <div class="history-item" :class="`bucket-${bucket}`">
    <div class="header-row">
      <span class="status-badge">
        <span class="status-icon">{{ badge.icon }}</span>
        <span class="status-text">{{ badge.label }}</span>
      </span>
      <span class="duration">{{ durationText }}</span>
      <span class="time" :title="record.created_at">{{ relativeTime }}</span>
      <n-button
        v-if="summary"
        size="tiny"
        quaternary
        class="copy-btn"
        title="复制摘要"
        @click="copySummary"
      >📋</n-button>
    </div>
    <div v-if="summary" class="summary" :title="summary">{{ summary }}</div>
    <div v-else class="summary summary-empty">（无响应内容）</div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton, useMessage } from 'naive-ui'
import type { TestCaseHistory } from '../../types'

const props = defineProps<{
  record: TestCaseHistory
}>()

const message = useMessage()

// 状态徽章 + 颜色桶
const badge = computed(() => {
  const sc = props.record.status_code
  if (sc === null) return { icon: '⚠', label: '网络' }
  if (sc < 300) return { icon: '✓', label: String(sc) }
  if (sc < 400) return { icon: '↪', label: String(sc) }
  if (sc < 500) return { icon: '⚠', label: String(sc) }
  return { icon: '✗', label: String(sc) }
})

const bucket = computed<'2xx' | '3xx' | '4xx' | '5xx' | 'network'>(() => {
  const sc = props.record.status_code
  if (sc === null) return 'network'
  if (sc < 300) return '2xx'
  if (sc < 400) return '3xx'
  if (sc < 500) return '4xx'
  return '5xx'
})

const durationText = computed(() => {
  const d = props.record.duration_ms
  return d === null ? '-- ms' : `${d} ms`
})

const summary = computed(() => {
  return props.record.response_preview ?? props.record.error_message ?? ''
})

// 简易"多久之前"
const relativeTime = computed(() => {
  const t = Date.parse(props.record.created_at + (props.record.created_at.endsWith('Z') ? '' : 'Z'))
  if (Number.isNaN(t)) return props.record.created_at
  const diff = Date.now() - t
  if (diff < 60_000) return '刚刚'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`
  if (diff < 7 * 86_400_000) return `${Math.floor(diff / 86_400_000)} 天前`
  // 超过 7 天显示日期
  const d = new Date(t)
  return `${d.getMonth() + 1}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
})

async function copySummary() {
  try {
    await navigator.clipboard.writeText(summary.value)
    message.success('已复制摘要')
  } catch {
    message.error('复制失败')
  }
}
</script>

<style scoped>
.history-item {
  padding: 8px 10px;
  border-radius: 4px;
  border: 1px solid var(--border-base);
  background: var(--bg-surface);
  font-size: 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.header-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-weight: 600;
  font-size: 12px;
  min-width: 56px;
}

/* 状态色按桶映射，使用 M3-B 的 status token */
.bucket-2xx     .status-badge { color: var(--status-2xx); }
.bucket-3xx     .status-badge { color: var(--status-3xx); }
.bucket-4xx     .status-badge { color: var(--status-4xx); }
.bucket-5xx     .status-badge { color: var(--status-5xx); }
.bucket-network .status-badge { color: var(--status-network); }

.status-icon {
  font-size: 13px;
  line-height: 1;
}

.duration {
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
  min-width: 60px;
}

.time {
  color: var(--text-tertiary);
  flex: 1;
  text-align: right;
}

.copy-btn {
  flex-shrink: 0;
  font-size: 11px !important;
}

.summary {
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: ui-monospace, 'SF Mono', Consolas, monospace;
  font-size: 11px;
}

.summary-empty {
  color: var(--text-disabled);
  font-style: italic;
}
</style>
