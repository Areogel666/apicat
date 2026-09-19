<template>
  <div class="token-row">
    <div class="picker-wrap">
      <n-color-picker
        :value="currentValue"
        :modes="['rgb', 'hex']"
        :show-alpha="true"
        size="small"
        @update:value="onColorChange"
      />
    </div>
    <div class="label">
      <span class="name">{{ tokenKey }}</span>
      <span class="desc">{{ description }}</span>
    </div>
    <span class="value">{{ currentValue }}</span>
    <button
      v-if="isModified"
      class="reset-btn"
      title="重置为默认值"
      @click="$emit('reset')"
    >↺</button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NColorPicker } from 'naive-ui'

const props = defineProps<{
  tokenKey: string
  currentValue: string
  defaultValue: string
  description: string
}>()

const emit = defineEmits<{
  'update:value': [val: string]
  'reset': []
}>()

const isModified = computed(() => props.currentValue !== props.defaultValue)

function onColorChange(val: string) {
  emit('update:value', val)
}
</script>

<style scoped>
.token-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 5px 12px;
  border-radius: var(--radius-sm);
  font-size: 12px;
}
.token-row:hover {
  background: var(--bg-hover);
}
/* n-color-picker 渲染 fragment，无法直接继承 style，用 wrapper 控制尺寸 */
.picker-wrap {
  width: 22px;
  height: 22px;
  flex-shrink: 0;
}
/* 剥掉 trigger 默认的边框/圆角/内边距，只留纯色块 */
.picker-wrap :deep(.n-color-picker) {
  width: 100%;
  height: 100%;
}
.picker-wrap :deep(.n-color-picker-trigger) {
  width: 100%;
  height: 100%;
  padding: 0;
  border: none;
  border-radius: 4px;
  box-shadow: none;
}
.picker-wrap :deep(.n-color-picker-fill) {
  border-radius: 4px;
}
/* 隐藏 picker 内部的色值文字，右侧 .value span 已展示，22px 色块塞不下 */
.picker-wrap :deep(.n-color-picker__value) {
  display: none;
}
.label {
  flex: 1;
  min-width: 0;
}
.label .name {
  color: var(--text-primary);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  display: block;
}
.label .desc {
  color: var(--text-tertiary);
  font-size: 10px;
}
.value {
  color: var(--text-secondary);
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  width: 68px;
  text-align: right;
  flex-shrink: 0;
}
.reset-btn {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: transparent;
  border: 1px solid var(--border-base);
  color: var(--text-tertiary);
  font-size: 10px;
  cursor: pointer;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
}
.reset-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
</style>
