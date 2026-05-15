<template>
  <!-- 若该格式只支持一种模式（如 text 只有 raw），不显示切换器 -->
  <n-radio-group
    v-if="modes.length > 1"
    :value="modelValue"
    size="small"
    @update:value="onChange"
  >
    <n-radio-button v-for="m in modes" :key="m" :value="m">
      {{ viewModeLabel(m) }}
    </n-radio-button>
  </n-radio-group>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NRadioGroup, NRadioButton } from 'naive-ui'
import { availableViewModes, viewModeLabel, type ResponseFormat, type ViewMode } from './useResponseFormat'

/**
 * 视图模式切换按钮组
 *
 * 支持的模式由当前格式决定（availableViewModes）：
 *   - json/xml/yaml: pretty | raw
 *   - markdown/html: raw | preview
 *   - text:         raw（单一，不显示切换器）
 */

const props = defineProps<{
  modelValue: ViewMode
  format: ResponseFormat
}>()

const emit = defineEmits<{
  'update:modelValue': [value: ViewMode]
}>()

const modes = computed(() => availableViewModes(props.format))

function onChange(value: ViewMode) {
  emit('update:modelValue', value)
}
</script>
