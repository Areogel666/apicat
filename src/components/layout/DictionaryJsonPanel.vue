<template>
  <div class="dict-json-panel">
    <!-- 未选中字典 -->
    <div v-if="!store.selectedDict" class="dict-json-placeholder">
      <n-empty description="← 在左侧选择一个字典，即可预览 / 编辑其 JSON" size="small" />
    </div>

    <template v-else>
      <!-- 头部 -->
      <div class="dict-json-head">
        <div class="dict-json-title">
          <span class="dict-json-code">{{ store.selectedDict.code }}</span>
          <span class="dict-json-name">{{ store.selectedDict.name }}</span>
          <n-tag size="small" :bordered="false" type="info">{{ itemCount }} 项</n-tag>
        </div>
        <div class="dict-json-actions">
          <n-button size="small" secondary @click="insertSample">插入示例</n-button>
          <n-button size="small" type="primary" :loading="saving" @click="save">
            💾 保存字典
          </n-button>
        </div>
      </div>

      <!-- JSON 编辑区 -->
      <n-input
        v-model:value="editorText"
        type="textarea"
        :rows="16"
        class="dict-json-textarea"
        placeholder='支持两种格式：\n{"0":"成功","1":"失败"}\n或 [{"value":"0","label":"成功","description":"..."}]\n\n修改后点「保存字典」全量替换'
      />

      <div class="dict-json-hint">
        支持 <code>{"0":"成功"}</code> 键值简写或 <code>[{"value","label","description"}]</code> 完整数组。
        保存为全量替换：将先清空该字典全部条目再写入。
      </div>

      <!-- 逐条列表（只读预览，便于对比） -->
      <div class="dict-json-items">
        <div
          v-for="item in store.itemsMap[store.selectedDict.id] ?? []"
          :key="item.id"
          class="dict-json-item"
        >
          <span class="dict-json-item-value">{{ item.value }}</span>
          <span class="dict-json-item-label">{{ item.label }}</span>
          <span v-if="item.description" class="dict-json-item-desc">{{ item.description }}</span>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { NButton, NInput, NTag, NEmpty, useMessage } from 'naive-ui'
import { useDictionaryStore } from '../../stores/dictionary'
import { parseItemsJson, itemsToJson } from '../../utils/dictionaryJson'

const store = useDictionaryStore()
const message = useMessage()

const editorText = ref('')
const saving = ref(false)

const itemCount = computed(() => (store.itemsMap[store.selectedDict?.id ?? 0] ?? []).length)

// 选中字典变化 → 同步 JSON 编辑内容
watch(() => store.selectedDictId, (id) => {
  if (id == null) {
    editorText.value = ''
    return
  }
  const items = store.itemsMap[id] ?? []
  editorText.value = itemsToJson(items)
}, { immediate: true })

// 字典项被外部修改（如右键新增/删除）后，若当前未在编辑（内容 == 旧 JSON）则跟随刷新
watch(() => itemCount.value, () => {
  if (!store.selectedDict) return
  const id = store.selectedDict.id
  const fresh = itemsToJson(store.itemsMap[id] ?? [])
  // 仅当用户尚未手动改动文本时才覆盖，避免打断编辑
  if (editorText.value.trim() === fresh.trim()) {
    editorText.value = fresh
  }
})

async function save() {
  const dictId = store.selectedDict?.id
  if (dictId == null) return
  const items = parseItemsJson(editorText.value)
  if (items.some(i => !i.value && !i.label)) {
    message.warning('存在无效条目（value 与 label 均为空）')
    return
  }
  saving.value = true
  try {
    await store.replaceDictionaryItems(dictId, items)
    message.success(`字典已保存（${items.length} 项）`)
  } catch (e) {
    message.error(String(e))
  } finally {
    saving.value = false
  }
}

function insertSample() {
  if (!store.selectedDict) return
  const sample = '{\n  "0": "成功",\n  "1": "失败",\n  "2": "处理中"\n}'
  editorText.value = editorText.value.trim()
    ? (editorText.value.endsWith('}') || editorText.value.endsWith(']'))
      ? editorText.value
      : editorText.value + '\n' + sample
    : sample
}
</script>

<style scoped>
.dict-json-panel {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
  padding: var(--spacing-md) var(--spacing-lg);
  overflow-y: auto;
  flex: 1;
  min-height: 0;
  height: 100%;
}

.dict-json-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
}

.dict-json-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-sm);
  flex-shrink: 0;
}
.dict-json-title {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  min-width: 0;
}
.dict-json-code {
  font-family: monospace;
  font-size: var(--font-size-lg);
  font-weight: 700;
  color: var(--color-primary);
}
.dict-json-name {
  font-size: var(--font-size-base);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.dict-json-actions {
  display: flex;
  gap: var(--spacing-xs);
  flex-shrink: 0;
}

.dict-json-textarea {
  font-family: monospace;
  font-size: var(--font-size-base);
}

.dict-json-hint {
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
  line-height: 1.6;
}
.dict-json-hint code {
  background: var(--bg-hover);
  border-radius: 3px;
  padding: 0 4px;
  font-family: monospace;
  font-size: 12px;
}

.dict-json-items {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
  border-top: 1px solid var(--border-base);
  padding-top: var(--spacing-sm);
}
.dict-json-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
}
.dict-json-item-value {
  font-family: monospace;
  color: var(--color-primary);
  flex-shrink: 0;
  min-width: 48px;
}
.dict-json-item-label {
  color: var(--text-primary);
}
.dict-json-item-desc {
  color: var(--text-tertiary);
  margin-left: auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>