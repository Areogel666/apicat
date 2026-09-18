<template>
  <n-modal
    :show="show"
    preset="dialog"
    title="用例执行结果"
    :show-icon="false"
    style="width: 680px"
    @update:show="(v: boolean) => emit('update:show', v)"
  >
    <div v-if="result" class="run-result">
      <!-- 总览 -->
      <div class="summary" :class="`status-${result.status}`">
        <span class="status-icon">{{ statusIcon }}</span>
        <span class="status-text">{{ statusLabel }}</span>
        <span v-if="result.status_code != null" class="status-code">HTTP {{ result.status_code }}</span>
        <span class="elapsed">{{ result.elapsed_ms }}ms</span>
        <span v-if="result.total_count > 0" class="assert-count">
          断言 {{ result.passed_count }}/{{ result.total_count }} 通过
        </span>
      </div>

      <div v-if="result.error_message" class="error-msg">
        {{ result.error_message }}
      </div>

      <!-- 断言明细 -->
      <div v-if="result.assertions.length > 0" class="assert-list">
        <div
          v-for="(a, i) in result.assertions"
          :key="i"
          class="assert-item"
          :class="a.passed ? 'passed' : 'failed'"
        >
          <span class="assert-icon">{{ a.passed ? '✓' : '✗' }}</span>
          <div class="assert-body">
            <div class="assert-title">
              <span class="assert-kind">{{ a.kind === 'status_code' ? '状态码' : a.path }}</span>
              <span class="assert-op">{{ operatorLabel(a.operator) }}</span>
              <span class="assert-expected">{{ a.expected || '（非空）' }}</span>
            </div>
            <div class="assert-actual">
              实际：<code>{{ a.actual }}</code>
            </div>
            <div v-if="!a.passed && a.message" class="assert-msg">{{ a.message }}</div>
          </div>
        </div>
      </div>
      <div v-else class="no-assert">
        该用例未配置断言，仅按 HTTP 状态码判定（2xx = 通过）。
      </div>
    </div>

    <template #action>
      <n-button @click="emit('update:show', false)">关闭</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NModal, NButton } from 'naive-ui'
import type { RunCaseResult } from '../../types'

const props = defineProps<{
  show: boolean
  result: RunCaseResult | null
}>()

const emit = defineEmits<{
  'update:show': [v: boolean]
}>()

const statusIcon = computed(() =>
  props.result?.status === 'passed' ? '✅' : props.result?.status === 'failed' ? '❌' : '⚠️'
)
const statusLabel = computed(() =>
  props.result?.status === 'passed' ? '通过' : props.result?.status === 'failed' ? '失败' : '执行错误'
)

function operatorLabel(op: string): string {
  const map: Record<string, string> = { eq: '=', ne: '≠', not_null: '非空', contains: '包含' }
  return map[op] ?? op
}
</script>

<style scoped>
.run-result {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.summary {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  font-size: 14px;
  font-weight: 600;
}

.summary.status-passed {
  background: color-mix(in srgb, var(--color-success, #18a058) 10%, transparent);
  color: var(--color-success, #18a058);
}

.summary.status-failed {
  background: color-mix(in srgb, var(--color-error, #d03050) 10%, transparent);
  color: var(--color-error, #d03050);
}

.summary.status-error {
  background: color-mix(in srgb, var(--color-warning, #f0a020) 10%, transparent);
  color: var(--color-warning, #f0a020);
}

.elapsed, .assert-count, .status-code {
  font-weight: 400;
  font-size: 12px;
  color: var(--text-secondary);
}

.error-msg {
  padding: 8px 12px;
  background: color-mix(in srgb, var(--color-error, #d03050) 8%, transparent);
  border-radius: var(--radius-sm);
  font-size: 12px;
  color: var(--color-error, #d03050);
  word-break: break-all;
}

.assert-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 320px;
  overflow-y: auto;
}

.assert-item {
  display: flex;
  gap: 8px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border-base);
  font-size: 12px;
}

.assert-item.passed {
  border-color: color-mix(in srgb, var(--color-success, #18a058) 30%, transparent);
}

.assert-item.failed {
  border-color: color-mix(in srgb, var(--color-error, #d03050) 30%, transparent);
}

.assert-icon {
  flex-shrink: 0;
  font-size: 14px;
  line-height: 1.4;
}

.assert-item.passed .assert-icon { color: var(--color-success, #18a058); }
.assert-item.failed .assert-icon { color: var(--color-error, #d03050); }

.assert-body {
  flex: 1;
  min-width: 0;
}

.assert-title {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text-primary);
}

.assert-kind {
  font-weight: 600;
}

.assert-op, .assert-expected {
  color: var(--text-secondary);
}

.assert-actual {
  margin-top: 2px;
  color: var(--text-secondary);
}

.assert-actual code {
  background: var(--bg-hover);
  padding: 1px 4px;
  border-radius: 2px;
  font-size: 11px;
  word-break: break-all;
}

.assert-msg {
  margin-top: 2px;
  color: var(--color-error, #d03050);
  font-size: 11px;
}

.no-assert {
  text-align: center;
  color: var(--text-tertiary);
  font-size: 12px;
  padding: 8px 0;
}
</style>
