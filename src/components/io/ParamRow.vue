<template>
  <div class="param-row">
    <n-checkbox v-model:checked="item.enabled" />
    <n-input v-model:value="item.key" size="small" style="width:140px; flex-shrink:0" :placeholder="keyPlaceholder" />
    <!-- 1.0.4 fix：值紧跟字段名（原排在最后，不直观） -->
    <n-input v-model:value="item.value" size="small" style="flex:1" :placeholder="valuePlaceholder" />
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
    <!-- 1.0.4：描述列（可手输 / 选字典；引用字典时显示最新值） -->
    <div class="param-row__desc">
      <!-- 已引用字典：显示字典最新值 + 断开引用 -->
      <n-tag v-if="isDictRef" size="small" :bordered="false" class="param-row__dict-tag" closable
        @close="clearDictRef">
        📖 {{ dictDisplay }}
      </n-tag>
      <n-input v-else v-model:value="item.description" size="small" placeholder="字段描述" />
      <n-button size="tiny" quaternary title="选择字典项" @click="$emit('pick-dict', item)">📖</n-button>
    </div>
    <n-button size="tiny" quaternary @click="$emit('remove', item)">✕</n-button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NInput, NButton, NSelect, NCheckbox, NTag } from 'naive-ui'
import type { ParamItem } from '../../types'

const props = withDefaults(defineProps<{
  item: ParamItem
  keyPlaceholder?: string
  valuePlaceholder?: string
  typeOptions: Array<{ label: string; value: string }>
  /** 已引用字典项的展示文本（最新值），有值表示该行描述来自字典引用 */
  dictDisplay?: string
}>(), {
  keyPlaceholder: 'Key',
  valuePlaceholder: 'Value',
})

const emit = defineEmits<{
  remove: [item: ParamItem]
  'pick-dict': [item: ParamItem]
  'clear-dict-ref': [item: ParamItem]
}>()

const isDictRef = computed(() => Boolean(props.item.descriptionDictRef))

function clearDictRef() {
  emit('clear-dict-ref', props.item)
}
</script>

<style scoped>
.param-row__desc {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 2px;
}
.param-row__dict-tag {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>