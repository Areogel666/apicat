<template>
  <n-modal
    :show="show"
    preset="dialog"
    title="编辑断言"
    :show-icon="false"
    style="width: 640px"
    @update:show="(v: boolean) => emit('update:show', v)"
  >
    <div class="assertion-editor">
      <div v-if="rows.length === 0" class="empty-hint">
        暂无断言。点击下方「添加断言」新增一条。
      </div>

      <div v-for="(row, i) in rows" :key="i" class="assertion-row">
        <n-select
          v-model:value="row.type"
          :options="typeOptions"
          size="small"
          style="width: 110px"
          @update:value="onTypeChange(row)"
        />
        <n-input
          v-if="row.type === 'json_path'"
          v-model:value="row.path"
          size="small"
          placeholder="$.code"
          style="width: 140px"
        />
        <span v-else class="path-placeholder"></span>
        <n-select
          v-model:value="row.operator"
          :options="operatorOptions(row.type)"
          size="small"
          style="width: 110px"
        />
        <n-input
          v-model:value="row.expected"
          size="small"
          :placeholder="row.operator === 'not_null' ? '（无需填）' : '期望值'"
          :disabled="row.operator === 'not_null'"
          style="flex: 1; min-width: 80px"
        />
        <n-button size="small" quaternary type="error" @click="rows.splice(i, 1)">✕</n-button>
      </div>

      <n-button size="small" dashed block @click="addRow" style="margin-top: 8px">
        + 添加断言
      </n-button>

      <div class="preset-bar">
        <span class="preset-label">快捷模板：</span>
        <n-button size="tiny" quaternary @click="applyPreset('happy')">Happy Path</n-button>
        <n-button size="tiny" quaternary @click="applyPreset('client_error')">4xx 错误</n-button>
        <n-button size="tiny" quaternary @click="applyPreset('clear')">清空</n-button>
      </div>
    </div>

    <template #action>
      <n-button @click="emit('update:show', false)">取消</n-button>
      <n-button type="primary" @click="onSave">保存</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { NModal, NSelect, NInput, NButton } from 'naive-ui'
import type { Assertion, AssertionType, AssertionOperator } from '../../types'

const props = defineProps<{
  show: boolean
  /** 当前断言 JSON 字符串（来自 test_cases.assertions） */
  modelValue: string
}>()

const emit = defineEmits<{
  'update:show': [v: boolean]
  save: [assertionsJson: string]
}>()

interface Row {
  type: AssertionType
  path: string
  operator: AssertionOperator
  expected: string
}

const rows = ref<Row[]>([])

const typeOptions = [
  { label: '状态码', value: 'status_code' },
  { label: 'JSON 路径', value: 'json_path' },
]

function operatorOptions(type: AssertionType) {
  if (type === 'status_code') {
    return [
      { label: '等于', value: 'eq' },
      { label: '不等于', value: 'ne' },
    ]
  }
  return [
    { label: '等于', value: 'eq' },
    { label: '不等于', value: 'ne' },
    { label: '非空', value: 'not_null' },
    { label: '包含', value: 'contains' },
  ]
}

// 打开时从 modelValue 解析
watch(() => props.show, (v) => {
  if (!v) return
  try {
    const parsed = JSON.parse(props.modelValue || '[]') as Assertion[]
    rows.value = parsed.map(a => ({
      type: (a.type === 'json_path' ? 'json_path' : 'status_code') as AssertionType,
      path: a.path ?? '',
      operator: a.operator as AssertionOperator,
      expected: a.expected ?? '',
    }))
  } catch {
    rows.value = []
  }
})

function addRow() {
  rows.value.push({ type: 'status_code', path: '', operator: 'eq', expected: '200' })
}

function onTypeChange(row: Row) {
  // 切类型时重置不兼容的操作符
  if (row.type === 'status_code' && !['eq', 'ne'].includes(row.operator)) {
    row.operator = 'eq'
    row.expected = '200'
  }
  if (row.type === 'json_path' && !row.path) {
    row.path = '$.'
  }
}

function applyPreset(kind: 'happy' | 'client_error' | 'clear') {
  if (kind === 'clear') {
    rows.value = []
  } else if (kind === 'happy') {
    rows.value = [
      { type: 'status_code', path: '', operator: 'eq', expected: '200' },
      { type: 'json_path', path: '$.code', operator: 'eq', expected: '0' },
    ]
  } else {
    rows.value = [
      { type: 'status_code', path: '', operator: 'eq', expected: '400' },
    ]
  }
}

function onSave() {
  const assertions: Assertion[] = rows.value
    .filter(r => r.type === 'status_code' || r.path.trim().length > 0)
    .map(r => ({
      type: r.type,
      ...(r.type === 'json_path' ? { path: r.path.trim() } : {}),
      operator: r.operator,
      expected: r.operator === 'not_null' ? '' : r.expected,
    }))
  emit('save', JSON.stringify(assertions))
  emit('update:show', false)
}
</script>

<style scoped>
.assertion-editor {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.empty-hint {
  text-align: center;
  color: var(--text-tertiary);
  font-size: 12px;
  padding: 16px 0;
}

.assertion-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.path-placeholder {
  width: 140px;
  flex-shrink: 0;
}

.preset-bar {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 10px;
  padding-top: 8px;
  border-top: 1px dashed var(--border-base);
}

.preset-label {
  font-size: 12px;
  color: var(--text-tertiary);
}
</style>
