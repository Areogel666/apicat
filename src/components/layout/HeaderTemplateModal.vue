<template>
  <n-modal :show="show" preset="card" title="公共 Headers 模板" style="width: 640px" :on-update:show="(v: boolean) => emit('update:show', v)">
    <div style="font-size:12px;color:var(--text-tertiary);margin-bottom:10px">
      在此设置公共 Headers（如 Authorization 前缀、Content-Type 等），可一键应用到任意接口的 Headers 编辑区。
    </div>

    <!-- Header 列表 -->
    <n-empty v-if="!store.items.length" description="暂无公共 Header，点击下方「+ 添加」" size="small" />
    <div v-for="item in store.items" :key="item.id" class="tpl-row">
      <n-checkbox
        :checked="item.enabled"
        @update:checked="store.updateItem(item.id, { enabled: $event })"
      />
      <n-input
        :value="item.key"
        size="small"
        style="width:160px;flex-shrink:0"
        placeholder="Header 名"
        @update:value="store.updateItem(item.id, { key: $event })"
      />
      <n-input
        :value="item.value"
        size="small"
        style="flex:1"
        placeholder="值"
        @update:value="store.updateItem(item.id, { value: $event })"
      />
      <n-input
        :value="item.description"
        size="small"
        style="width:120px;flex-shrink:0"
        placeholder="备注（可选）"
        @update:value="store.updateItem(item.id, { description: $event })"
      />
      <n-button size="tiny" quaternary @click="store.removeItem(item.id)">✕</n-button>
    </div>

    <n-button size="small" dashed style="margin-top:8px;width:100%" @click="store.addItem()">
      + 添加
    </n-button>

    <template #footer>
      <div style="display:flex;justify-content:flex-end;gap:8px">
        <n-button @click="emit('update:show', false)">关闭</n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { NModal, NInput, NButton, NEmpty, NCheckbox } from 'naive-ui'
import { useHeaderTemplateStore } from '../../stores/headerTemplate'

defineProps<{ show: boolean }>()
const emit = defineEmits<{ (e: 'update:show', v: boolean): void }>()

const store = useHeaderTemplateStore()
</script>

<style scoped>
.tpl-row {
  display: flex;
  gap: 6px;
  align-items: center;
  margin-bottom: 6px;
}
</style>
