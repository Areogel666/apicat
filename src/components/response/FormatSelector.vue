<template>
  <n-select
    :value="modelValue"
    :options="options"
    size="tiny"
    style="width: 120px"
    @update:value="onChange"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NSelect } from 'naive-ui'
import { formatLabel, type ResponseFormat } from './useResponseFormat'
import type { FormatOverride } from '../../stores/response'

/**
 * 格式选择下拉
 *
 * 选项：Auto（自动检测，显示已识别格式） + 6 种手动格式
 * 用户手动选择后，由父组件（JsonViewer 路由器）写入 store 的 formatOverrideMap
 */

const props = defineProps<{
  /** 当前选中的值：'auto' 或具体格式 */
  modelValue: FormatOverride
  /** detectFormat 识别的格式，用于 Auto 选项标注"自动：JSON" */
  detected: ResponseFormat
}>()

const emit = defineEmits<{
  'update:modelValue': [value: FormatOverride]
}>()

const options = computed(() => [
  { label: `Auto (${formatLabel(props.detected)})`, value: 'auto' as const },
  { label: 'JSON', value: 'json' as const },
  { label: 'XML', value: 'xml' as const },
  { label: 'HTML', value: 'html' as const },
  { label: 'YAML', value: 'yaml' as const },
  { label: 'Markdown', value: 'markdown' as const },
  { label: 'Text', value: 'text' as const },
])

function onChange(value: FormatOverride) {
  emit('update:modelValue', value)
}
</script>
