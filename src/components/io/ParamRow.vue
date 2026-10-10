<template>
  <div class="param-row">
    <n-checkbox v-model:checked="item.enabled" />
    <n-input v-model:value="item.key" size="small" style="width:140px; flex-shrink:0" :placeholder="keyPlaceholder" />
    <!-- 1.0.4 fix：值紧跟字段名（原排在最后，不直观） -->
    <!-- file 类型行：值 = 本地文件绝对路径，📁 选择 / 可手输粘贴 / ✕ 清除 -->
    <div v-if="isFileField" class="param-row__value">
      <n-input
        v-model:value="item.value"
        size="small"
        style="flex:1"
        placeholder="文件绝对路径（点 📁 选择或直接粘贴）"
      >
        <template #suffix>
          <span class="param-row__filebtns">
            <n-button size="tiny" quaternary title="选择文件" @click="pickFile">📁</n-button>
            <n-button v-if="item.value" size="tiny" quaternary title="清除" @click="item.value = ''">✕</n-button>
          </span>
        </template>
      </n-input>
      <!-- 文件名/大小实时派生（按路径 stat，不落库）；不存在时红字提示 -->
      <div v-if="item.value" class="param-row__filemeta" :class="{ 'file-missing': fileMeta && !fileMeta.exists }">
        {{ fileMetaText }}
      </div>
    </div>
    <n-input v-else v-model:value="item.value" size="small" style="flex:1" :placeholder="valuePlaceholder" />
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
import { ref, computed, watch } from 'vue'
import { NInput, NButton, NSelect, NCheckbox, NPopover } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
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

// ── 文件字段（type = "file"，Form Data 表格）──────────────────
const isFileField = computed(() => props.item.type === 'file')

interface FileMeta { exists: boolean; name: string; size: number }
const fileMeta = ref<FileMeta | null>(null)

// 路径变化 → Rust stat 取文件名/大小/存在性（轻量系统调用，不防抖）
watch(
  () => (isFileField.value ? props.item.value : ''),
  async (path) => {
    if (!path) { fileMeta.value = null; return }
    try {
      fileMeta.value = await invoke<FileMeta>('file_meta', { path })
    } catch {
      fileMeta.value = null
    }
  },
  { immediate: true },
)

const fileMetaText = computed(() => {
  if (!fileMeta.value) return ''
  if (!fileMeta.value.exists) return `⚠ 文件不存在：${fileMeta.value.name}`
  return `${fileMeta.value.name} · ${fmtSize(fileMeta.value.size)}`
})

function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1024 / 1024).toFixed(1)} MB`
}

async function pickFile() {
  const picked = await open({ multiple: false, directory: false })
  if (typeof picked === 'string') props.item.value = picked
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
.param-row__value {
  flex: 1;
  min-width: 0;
}
.param-row__filemeta {
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
  line-height: 1.4;
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.param-row__filemeta.file-missing {
  color: var(--color-error);
}
.param-row__filebtns {
  display: inline-flex;
  align-items: center;
  gap: 0;
}
.fdd-edit-on {
  color: var(--color-primary);
}
</style>
