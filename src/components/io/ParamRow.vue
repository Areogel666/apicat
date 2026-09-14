<template>
  <div class="param-row">
    <n-checkbox v-model:checked="item.enabled" />
    <n-input v-model:value="item.key" size="small" style="width:140px; flex-shrink:0" :placeholder="keyPlaceholder" />
    <!-- 1.0.4：类型列 -->
    <n-select
      v-model:value="item.type"
      size="small"
      style="width:100px; flex-shrink:0"
      placeholder="类型"
      :options="typeOptions"
      clearable
      tag
    />
    <!-- 1.0.4：描述列（可手输 / 选字典） -->
    <div class="param-row__desc">
      <n-input v-model:value="item.description" size="small" placeholder="字段描述" />
      <n-button size="tiny" quaternary title="选择字典项" @click="$emit('pick-dict', item)">📖</n-button>
    </div>
    <n-input v-model:value="item.value" size="small" style="flex:1" :placeholder="valuePlaceholder" />
    <n-button size="tiny" quaternary @click="$emit('remove', item)">✕</n-button>
  </div>
</template>

<script setup lang="ts">
import { NInput, NButton, NSelect, NCheckbox } from 'naive-ui'
import type { ParamItem } from '../../types'

defineProps<{
  item: ParamItem
  keyPlaceholder?: string
  valuePlaceholder?: string
  typeOptions: Array<{ label: string; value: string }>
}>()

defineEmits<{
  remove: [item: ParamItem]
  'pick-dict': [item: ParamItem]
}>()
</script>

<style scoped>
.param-row__desc {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 2px;
}
</style>