<template>
  <aside class="dict-sidebar">
    <div class="dict-sidebar__header">
      <span class="dict-sidebar__title">数据字典</span>
      <n-button size="small" secondary :disabled="!currentProjectId" @click="onAddDict">+ 新建</n-button>
    </div>

    <n-empty v-if="!currentProjectId" description="请先选择或创建项目" size="small" style="margin-top: 40px" />
    <div v-else class="dict-sidebar__tree">
      <n-tree
        :data="treeData"
        :render-suffix="renderSuffix"
        default-expand-all
        block-line
        expand-on-click
      />
    </div>

    <!-- 新建/编辑字典弹窗 -->
    <n-modal v-model:show="showDictModal" :preset="'dialog'" :title="editingDictId !== null ? '编辑字典' : '新建字典'">
      <div class="dict-form">
        <n-input v-model:value="dictCode" placeholder="字典编码，如 STATUS_CODE" :disabled="editingDictId !== null" />
        <n-input v-model:value="dictName" placeholder="字典名，如 状态码" />
      </div>
      <template #action>
        <n-button @click="showDictModal = false">取消</n-button>
        <n-button type="primary" :loading="savingDict" @click="saveDict">保存</n-button>
      </template>
    </n-modal>

    <!-- 新建/编辑字典项弹窗 -->
    <n-modal v-model:show="showItemModal" :preset="'dialog'" :title="editingItemId !== null ? '编辑字典项' : '新建字典项'">
      <div class="dict-form">
        <div class="dict-form__hint">{{ activeDictName }}</div>
        <n-input v-model:value="itemLabel" placeholder="展示名，如 成功" />
        <n-input v-model:value="itemValue" placeholder="值，如 0" />
        <n-input v-model:value="itemDescription" placeholder="描述（可选）" />
      </div>
      <template #action>
        <n-button @click="showItemModal = false">取消</n-button>
        <n-button type="primary" :loading="savingItem" @click="saveItem">保存</n-button>
      </template>
    </n-modal>
  </aside>
</template>

<script setup lang="ts">
import { computed, h, onMounted, ref } from 'vue'
import type { TreeOption } from 'naive-ui'
import {
  NButton, NEmpty, NInput, NModal, NTree, NDropdown, useMessage,
} from 'naive-ui'
import { useDictionaryStore } from '../../stores/dictionary'
import { useProjectStore } from '../../stores/project'
import type { DictionaryItem } from '../../types'

const store = useDictionaryStore()
const projectStore = useProjectStore()
const message = useMessage()

const currentProjectId = computed(() => projectStore.currentProjectId)

// ── 字典弹窗状态 ────────────────────────────────────────────
const showDictModal = ref(false)
const savingDict = ref(false)
const dictCode = ref('')
const dictName = ref('')
const editingDictId = ref<number | null>(null)

// ── 字典项弹窗状态 ──────────────────────────────────────────
const showItemModal = ref(false)
const savingItem = ref(false)
const itemLabel = ref('')
const itemValue = ref('')
const itemDescription = ref('')
const editingItemId = ref<number | null>(null)
const itemDictId = ref<number | null>(null)
const activeDictName = computed(() => {
  const d = store.dictionaries.find(x => x.id === itemDictId.value)
  return d ? `${d.code}（${d.name}）` : ''
})

// ── NTree 数据 ──────────────────────────────────────────────
interface DictNode extends TreeOption {
  isLeaf?: boolean
}
const treeData = computed<DictNode[]>(() =>
  store.dictionaries.map(d => ({
    key: `dict-${d.id}`,
    label: `${d.code}（${d.name}）`,
    children: store.itemsMap[d.id]?.map(item => ({
      key: `item-${item.id}`,
      label: `${item.value} = ${item.label}${item.description ? ' — ' + item.description : ''}`,
      isLeaf: true,
    })) ?? [],
  })),
)

function renderSuffix(info: { option: TreeOption }) {
  const key = String(info.option.key)
  if (key.startsWith('dict-')) {
    const d = store.dictionaries.find(x => x.id === Number(key.slice(5)))
    return h(NDropdown, {
      options: [
        { label: '新增字典项', key: 'add-item' },
        { label: '编辑', key: 'edit' },
        { label: '删除', key: 'delete' },
      ],
      onSelect: (action: string) => {
        if (!d) return
        if (action === 'add-item') onAddItem(d.id)
        else if (action === 'edit') onEditDict(d.id)
        else if (action === 'delete') onDeleteDict(d.id)
      },
    }, {
      default: () => h(NButton, { size: 'tiny', quaternary: true }, { default: () => '⋮' }),
    })
  }
  const item = findItem(Number(key.slice(5)))
  return h(NDropdown, {
    options: [
      { label: '编辑', key: 'edit' },
      { label: '删除', key: 'delete' },
    ],
    onSelect: (action: string) => {
      if (!item) return
      if (action === 'edit') onEditItem(item.id)
      else if (action === 'delete') onDeleteItem(item.id)
    },
  }, {
    default: () => h(NButton, { size: 'tiny', quaternary: true }, { default: () => '⋮' }),
  })
}

function findItem(id: number): DictionaryItem | null {
  for (const list of Object.values(store.itemsMap)) {
    const f = list.find(x => x.id === id)
    if (f) return f
  }
  return null
}

// ── 字典操作 ────────────────────────────────────────────────
function onAddDict() {
  editingDictId.value = null
  dictCode.value = ''
  dictName.value = ''
  showDictModal.value = true
}

function onEditDict(id: number) {
  const d = store.dictionaries.find(x => x.id === id)
  if (!d) return
  editingDictId.value = d.id
  dictCode.value = d.code
  dictName.value = d.name
  showDictModal.value = true
}

async function saveDict() {
  if (!dictName.value.trim()) {
    message.warning('字典名必填')
    return
  }
  savingDict.value = true
  try {
    if (editingDictId.value !== null) {
      await store.updateDictionary(editingDictId.value, { name: dictName.value.trim() })
      message.success('已更新')
    } else {
      if (!dictCode.value.trim()) {
        message.warning('字典编码必填')
        return
      }
      await store.createDictionary(dictCode.value.trim(), dictName.value.trim())
      message.success('字典创建成功')
    }
    showDictModal.value = false
  } catch (e) {
    message.error(String(e))
  } finally {
    savingDict.value = false
  }
}

async function onDeleteDict(id: number) {
  try {
    await store.deleteDictionary(id)
    message.success('已删除')
  } catch (e) {
    message.error(String(e))
  }
}

// ── 字典项操作 ──────────────────────────────────────────────
function onAddItem(dictId: number) {
  itemDictId.value = dictId
  editingItemId.value = null
  itemLabel.value = ''
  itemValue.value = ''
  itemDescription.value = ''
  showItemModal.value = true
}

function onEditItem(id: number) {
  const f = findItem(id)
  if (!f) return
  editingItemId.value = id
  itemDictId.value = f.dictionary_id
  itemLabel.value = f.label
  itemValue.value = f.value
  itemDescription.value = f.description
  showItemModal.value = true
}

async function saveItem() {
  if (!itemLabel.value.trim() || !itemValue.value.trim()) {
    message.warning('展示名和值必填')
    return
  }
  if (!itemDictId.value) {
    message.warning('缺少字典上下文')
    return
  }
  savingItem.value = true
  try {
    if (editingItemId.value !== null) {
      await store.updateItem(editingItemId.value, {
        label: itemLabel.value.trim(),
        value: itemValue.value.trim(),
        description: itemDescription.value.trim(),
      })
      message.success('已更新')
    } else {
      await store.createItem(itemDictId.value, itemLabel.value.trim(), itemValue.value.trim(), itemDescription.value.trim())
      message.success('字典项已添加')
    }
    showItemModal.value = false
  } catch (e) {
    message.error(String(e))
  } finally {
    savingItem.value = false
  }
}

async function onDeleteItem(id: number) {
  try {
    await store.deleteItem(id)
    message.success('已删除')
  } catch (e) {
    message.error(String(e))
  }
}

onMounted(() => {
  if (currentProjectId.value) {
    store.loadDictionaries(currentProjectId.value)
  }
})
</script>

<style scoped>
.dict-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  border-right: 1px solid var(--border-base);
  background: var(--bg-surface);
  flex-shrink: 0;
  overflow: hidden;
}
.dict-sidebar__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--spacing-sm);
  flex-shrink: 0;
}
.dict-sidebar__title {
  font-size: var(--font-size-base);
  font-weight: 600;
}
.dict-sidebar__tree {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-xs) 0;
}
.dict-form {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  padding: var(--spacing-md) 0;
}
.dict-form__hint {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
</style>