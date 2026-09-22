<template>
  <aside class="dict-sidebar">
    <div class="dict-sidebar__header">
      <span class="dict-sidebar__title">数据字典</span>
      <n-button size="small" secondary :disabled="!currentProjectId" @click="onAddDict">+ 新建</n-button>
    </div>

    <n-empty v-if="!currentProjectId" description="请先选择或创建项目" size="small" style="margin-top: 40px" />
    <div v-else class="dict-sidebar__body">
      <n-input
        v-model:value="dictSearch"
        size="small"
        clearable
        placeholder="搜索字典 / 字典项"
        class="dict-sidebar__search"
      />
      <div class="dict-sidebar__tree">
        <n-tree
          :data="filteredTree"
          :render-label="renderLabel"
          :render-suffix="renderSuffix"
          default-expand-all
          block-line
          expand-on-click
          @update:selected-keys="onTreeSelect"
        />
        <div v-if="treeData.length && !filteredTree.length" class="dict-sidebar__noresult">没有匹配「{{ dictSearch }}」</div>
      </div>
    </div>

    <!-- 新建/编辑字典弹窗（编辑时仅名；新建时可直接填字典项 JSON 一键创建） -->
    <n-modal v-model:show="showDictModal" :preset="'dialog'" :title="editingDictId !== null ? '编辑字典' : '新建字典'">
      <div class="dict-form">
        <n-input v-model:value="dictCode" placeholder="字典编码，如 STATUS_CODE" :disabled="editingDictId !== null" />
        <n-input v-model:value="dictName" placeholder="字典名，如 状态码" />
        <template v-if="editingDictId === null">
          <div class="dict-ops">
            <span class="dict-ops__label">字典项</span>
            <div v-for="(it, i) in newDictItems" :key="i" class="new-item-row">
              <n-input v-model:value="it.value" size="small" placeholder="值，如 0" style="width: 80px; flex-shrink: 0" />
              <n-input v-model:value="it.label" size="small" placeholder="展示名，如 成功" style="flex: 1" />
              <n-input v-model:value="it.description" size="small" placeholder="描述（可选）" style="width: 110px" @keyup.enter="addNewItemRow" />
              <n-button size="tiny" quaternary @click="newDictItems.splice(i, 1)">✕</n-button>
            </div>
            <n-button size="tiny" dashed block @click="addNewItemRow">+ 添加字典项</n-button>
          </div>
        </template>
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

    <!-- 复制字典到项目弹窗 -->
    <n-modal v-model:show="showCopyModal" :preset="'dialog'" title="复制字典到项目">
      <div class="dict-form">
        <div class="dict-form__hint">
          将把字典「{{ copySourceDict?.code }}」及全部字典项复制到所选项目；
          源项目里已绑定该字典的字段规则也会一并复制（目标项目同字段名已有规则则跳过）。
        </div>
        <n-select v-model:value="copyTargetProjectId" :options="copyProjectOptions" placeholder="选择目标项目" />
      </div>
      <template #action>
        <n-button @click="showCopyModal = false">取消</n-button>
        <n-button type="primary" :loading="savingCopy" @click="confirmCopyDict">复制</n-button>
      </template>
    </n-modal>
  </aside>
</template>

<script setup lang="ts">
import { computed, h, ref, watch } from 'vue'
import type { TreeOption } from 'naive-ui'
import {
  NButton, NEmpty, NInput, NModal, NTree, NSelect, NDropdown, NTooltip, useMessage,
} from 'naive-ui'
import { useDictionaryStore } from '../../stores/dictionary'
import { useProjectStore } from '../../stores/project'
import type { DictionaryItem, DataDictionary } from '../../types'

const store = useDictionaryStore()
const projectStore = useProjectStore()
const message = useMessage()

const currentProjectId = computed(() => projectStore.currentProjectId)

// ── 新建/编辑字典弹窗状态 ──────────────────────────────────────
const showDictModal = ref(false)
const savingDict = ref(false)
const dictCode = ref('')
const dictName = ref('')
const editingDictId = ref<number | null>(null)

// 1.0.4 fix：新建字典时用「操作表单」逐条添加字典项（非 JSON 输入）
interface NewDictItem { value: string; label: string; description: string }
const newDictItems = ref<NewDictItem[]>([])

function addNewItemRow() {
  newDictItems.value.push({ value: '', label: '', description: '' })
}

// ── 字典项弹窗状态 ──────────────────────────────────────────────
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
  dict?: DataDictionary
  item?: DictionaryItem
}
const treeData = computed<DictNode[]>(() =>
  store.dictionaries.map(d => ({
    key: `dict-${d.id}`,
    label: `${d.code}（${d.name}）`,
    dict: d,
    children: store.itemsMap[d.id]?.map(item => ({
      key: `item-${item.id}`,
      label: `${item.value} = ${item.label}${item.description ? ' — ' + item.description : ''}`,
      isLeaf: true,
      item,
    })) ?? [],
  })),
)

// 1.0.4 fix：字典树搜索（按字典 code/name 或字典项 value/label 过滤）
const dictSearch = ref('')
const filteredTree = computed<DictNode[]>(() => {
  const kw = dictSearch.value.trim().toLowerCase()
  if (!kw) return treeData.value
  return treeData.value.flatMap(d => {
    const dLabel = String(d.label ?? '')
    if (dLabel.toLowerCase().includes(kw)) return [d]
    const kids = (d.children ?? []).filter(c => String(c.label ?? '').toLowerCase().includes(kw))
    return kids.length ? [{ ...d, children: kids }] : []
  })
})

// 1.0.5：节点自定义渲染——字典两行（code 主 / name 副），字典项单行 + 描述 ⓘ 悬停
function renderLabel(info: { option: TreeOption }) {
  const opt = info.option as DictNode
  const key = String(opt.key)
  if (key.startsWith('dict-') && opt.dict) {
    const d = opt.dict
    return h('div', { class: 'dict-node' }, [
      h('div', { class: 'dict-node__code' }, d.code),
      h('div', { class: 'dict-node__name' }, d.name || ' '),
    ])
  }
  if (opt.item) {
    const it = opt.item
    return h('div', { class: 'dict-item' }, [
      h('span', { class: 'dict-item__kv' }, [
        h('code', {}, it.value),
        ` = ${it.label}`,
      ]),
      it.description
        ? h(NTooltip, { trigger: 'hover', placement: 'top' }, {
            trigger: () => h('span', { class: 'dict-item__desc-icon' }, 'ⓘ'),
            default: () => it.description ?? '',
          })
        : null,
    ])
  }
  return String(opt.label ?? '')
}

function renderSuffix(info: { option: TreeOption }) {
  const key = String(info.option.key)
  if (key.startsWith('dict-')) {
    const d = store.dictionaries.find(x => x.id === Number(key.slice(5)))
    return h(NDropdown, {
      options: [
        { label: '新增字典项', key: 'add-item' },
        { label: '编辑', key: 'edit' },
        { label: '删除', key: 'delete' },
        { type: 'divider', key: 'd1' },
        { label: '复制到项目…', key: 'copy' },
      ],
      onSelect: (action: string) => {
        if (!d) return
        if (action === 'add-item') onAddItem(d.id)
        else if (action === 'edit') onEditDict(d.id)
        else if (action === 'delete') onDeleteDict(d.id)
        else if (action === 'copy') onShowCopyDict(d.id)
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

// 1.0.4 fix：树选中 → 驱动右侧 JSON 编辑面板（点击字典或其条目均可）
function onTreeSelect(keys: Array<string | number>) {
  const k = keys[0]
  if (k == null) return
  const str = String(k)
  if (str.startsWith('dict-')) {
    store.setSelectedDict(Number(str.slice(5)))
  } else if (str.startsWith('item-')) {
    const item = findItem(Number(str.slice(5)))
    if (item) store.setSelectedDict(item.dictionary_id)
  }
}

// ── 字典操作 ────────────────────────────────────────────────
function onAddDict() {
  editingDictId.value = null
  dictCode.value = ''
  dictName.value = ''
  newDictItems.value = []
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
      // 1.0.4 fix：一键创建（字典 + 操作表单里的字典项）
      const items = newDictItems.value
        .filter(it => it.value.trim() || it.label.trim())
        .map(it => ({ value: it.value.trim(), label: it.label.trim(), description: it.description.trim() }))
      await store.createDictionaryWithItems(
        dictCode.value.trim(),
        dictName.value.trim(),
        '',
        items,
        projectStore.currentProjectId,
      )
      message.success(items.length ? `字典创建成功（${items.length} 项）` : '字典创建成功')
    }
    showDictModal.value = false
  } catch (e) {
    message.error(String(e))
  } finally {
    savingDict.value = false
  }
}

// ── 1.0.4：复制字典到项目 ──────────────────────────────────────
const showCopyModal = ref(false)
const savingCopy = ref(false)
const copySourceDict = ref<DataDictionary | null>(null)
const copyTargetProjectId = ref<number | null>(null)

const copyProjectOptions = computed(() =>
  projectStore.projects
    .filter(p => p.id !== projectStore.currentProjectId)
    .map(p => ({ label: p.name, value: p.id })),
)

function onShowCopyDict(id: number) {
  const d = store.dictionaries.find(x => x.id === id)
  if (!d) return
  copySourceDict.value = d
  copyTargetProjectId.value = copyProjectOptions.value[0]?.value ?? null
  showCopyModal.value = true
}

async function confirmCopyDict() {
  const d = copySourceDict.value
  const pid = projectStore.currentProjectId
  if (!d || pid == null || copyTargetProjectId.value == null) {
    message.warning('缺少目标项目')
    return
  }
  savingCopy.value = true
  try {
    await store.copyDictionaryToProject(d.id, pid, copyTargetProjectId.value)
    message.success(`已复制到项目`)
    showCopyModal.value = false
  } catch (e) {
    message.error(String(e))
  } finally {
    savingCopy.value = false
  }
}

async function onDeleteDict(id: number) {
  try {
    await store.deleteDictionary(id)
    if (store.selectedDictId === id) store.setSelectedDict(null)
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

// 1.0.4 fix：项目变化时重新加载字典（保证左侧栏数据就绪）
watch(currentProjectId, (pid) => {
  if (pid != null) {
    store.loadDictionaries(pid).catch(() => {})
    store.loadFieldBindings(pid).catch(() => {})
  } else {
    store.dictionaries = []
    store.setSelectedDict(null)
    void store.loadFieldBindings(null)   // 内部会清空 rules/overrides
  }
}, { immediate: true })
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
.dict-sidebar__body {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.dict-sidebar__search {
  padding: var(--spacing-xs) var(--spacing-sm) 0;
  flex-shrink: 0;
}
.dict-sidebar__tree {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-xs) 0;
}
.dict-sidebar__noresult {
  padding: var(--spacing-sm) var(--spacing-sm);
  color: var(--text-tertiary);
  font-size: var(--font-size-sm);
}

/* 1.0.5：字典节点两行（code 主标识 / name 描述说明）+ 字典项单行 + 描述 ⓘ 悬停
   层次：code 用主色+加粗（标识符），name 用弱色+小号（描述说明），区分清晰 */
.dict-node {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 3px 0;
  min-width: 0;
}
.dict-node__code {
  font-size: var(--font-size-base);
  line-height: 1.5;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dict-node__name {
  font-size: var(--font-size-sm);
  font-weight: 400;
  line-height: 1.4;
  color: var(--text-tertiary); /* 描述说明：弱色弱化，不与 code 争抢 */
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dict-item {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  font-size: var(--font-size-sm);
  line-height: var(--row-height); /* 单行高度随主题紧凑/宽松档位 */
}
.dict-item__kv {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dict-item__kv code {
  font-size: 0.92em;
  color: var(--color-primary);
  background: var(--bg-hover);
  border-radius: 3px;
  padding: 0 3px;
}
.dict-item__desc-icon {
  flex-shrink: 0;
  color: var(--text-tertiary);
  font-size: 12px;
  cursor: help;
}
.dict-form {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  padding: var(--spacing-md) 0;
}
.dict-form__hint {
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
}

/* 1.0.4 fix：新建字典的操作表单 */
.dict-ops {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}
.dict-ops__label {
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-tertiary);
}
.new-item-row {
  display: flex;
  align-items: center;
  gap: 4px;
}
</style>