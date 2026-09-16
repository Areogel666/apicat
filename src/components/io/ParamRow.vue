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
    <!-- 描述列：未绑定字典 → 直接可编辑的手写描述输入；已绑定字典 → 命中展示 + ✎ 编辑手写 -->
    <div class="param-row__desc">
      <template v-if="dictBound">
        <FieldDictDesc :field="item.key" :value="item.value" :manual="item.description ?? ''" />
        <n-popover
          :show="descPop"
          :width="240"
          trigger="click"
          placement="bottom-end"
          @update:show="descPop = $event"
        >
          <template #trigger>
            <n-button size="tiny" quaternary title="编辑手写描述" :class="{ 'fdd-edit-on': item.description }">✎</n-button>
          </template>
          <n-input
            v-model:value="item.description"
            type="textarea"
            :rows="2"
            placeholder="手写描述（可选，展示在字典信息后）"
          />
        </n-popover>
      </template>
      <n-input v-else v-model:value="item.description" size="small" placeholder="字段描述" />
      <n-button
        size="tiny"
        quaternary
        title="绑定 / 换绑 / 解绑字典"
        @click="$emit('pick-dict', item)"
      >📖</n-button>
      <n-button size="tiny" quaternary @click="$emit('remove', item)">✕</n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NInput, NButton, NSelect, NCheckbox, NPopover } from 'naive-ui'
import FieldDictDesc from './FieldDictDesc.vue'
import { useDictionaryStore } from '../../stores/dictionary'
import { useRequestStore } from '../../stores/request'
import type { ParamItem } from '../../types'

const props = withDefaults(defineProps<{
  item: ParamItem
  keyPlaceholder?: string
  valuePlaceholder?: string
  typeOptions: Array<{ label: string; value: string }>
}>(), {
  keyPlaceholder: 'Key',
  valuePlaceholder: 'Value',
})

defineEmits<{
  remove: [item: ParamItem]
  'pick-dict': [item: ParamItem]
}>()

const dictStore = useDictionaryStore()
const requestStore = useRequestStore()

/** 该字段是否有字典绑定（非空）—— 决定描述列展示「命中/Tooltip」还是「直接编辑」 */
const dictBound = computed(() => dictStore.dictIdForField(props.item.key, requestStore.activeRequestId) != null)

// 行内手写描述编辑弹层（每行独立状态）
const descPop = ref(false)
</script>

<style scoped>
.param-row__desc {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 2px;
}
.fdd-edit-on {
  color: var(--color-primary);
}
</style>