<template>
  <div class="testcase-manager">
    <!-- 单列表面板：左右分栏已移除，历史统一去下方 Response → History 区 -->
    <div class="manager-body">
      <div v-if="cases.length === 0" class="empty">
        当前接口暂无用例。<br>
        <span class="hint">发起一次请求会自动生成用例。</span>
      </div>
      <div v-else class="case-list">
        <div class="list-toolbar">
          <n-select
            v-model:value="typeFilter"
            :options="typeFilterOptions"
            size="tiny"
            placeholder="全部类型"
            clearable
            class="type-filter"
          />
          <n-select
            v-model:value="viewFilter"
            :options="viewFilterOptions"
            size="tiny"
            class="view-filter"
          />
        </div>
        <n-data-table
          :columns="columns"
          :data="filteredCases"
          :row-key="rowKey"
          v-model:checked-row-keys="checkedIds"
          :pagination="false"
          :flex-height="true"
          :row-class-name="rowClassName"
          :row-props="rowProps"
          size="small"
          class="case-table"
        />
      </div>
    </div>

    <!-- 底部操作栏 -->
    <footer class="manager-footer">
      <span class="footer-info">
        <template v-if="checkedIds.length > 0">
          已选 {{ checkedIds.length }} / {{ filteredCases.length }}
        </template>
        <template v-else>
          共 {{ cases.length }} 个用例
        </template>
      </span>
      <div class="footer-actions">
        <n-button size="small" :disabled="checkedIds.length === 0" @click="onBatchHide(true)">
          隐藏
        </n-button>
        <n-button size="small" :disabled="checkedIds.length === 0" @click="onBatchHide(false)">
          取消隐藏
        </n-button>
        <n-popconfirm
          :show-icon="false"
          :positive-button-props="{ type: 'error' }"
          @positive-click="onBatchDelete"
        >
          <template #trigger>
            <n-button
              type="error"
              size="small"
              :disabled="checkedIds.length === 0"
            >
              批量删除（{{ checkedIds.length }}）
            </n-button>
          </template>
          确定删除选中的 {{ checkedIds.length }} 个用例？此操作不可恢复。
        </n-popconfirm>
      </div>
    </footer>

    <!-- 重命名弹窗（双击行 / 右键菜单 / 行内 ✎ 触发） -->
    <n-modal v-model:show="showRenameModal" preset="dialog" title="重命名用例" :show-icon="false">
      <n-input
        ref="renameInputRef"
        v-model:value="renameInput"
        placeholder="输入新名称"
        @keyup.enter="confirmRename"
      />
      <template #action>
        <n-button @click="showRenameModal = false">取消</n-button>
        <n-button type="primary" @click="confirmRename">确定</n-button>
      </template>
    </n-modal>

    <!-- 右键菜单 -->
    <n-dropdown
      :show="contextMenuVisible"
      :x="contextMenuX"
      :y="contextMenuY"
      :options="contextMenuOptions"
      placement="bottom-start"
      @clickoutside="contextMenuVisible = false"
      @select="handleContextMenuSelect"
    />

    <!-- 1.0.5：断言编辑器 / 跑用例结果 -->
    <AssertionEditor
      v-model:show="showAssertionEditor"
      :model-value="editingAssertions"
      @save="onSaveAssertions"
    />
    <RunResultPanel
      v-model:show="showRunResult"
      :result="runResult"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, h, ref, watch, nextTick } from 'vue'
import {
  NDataTable, NButton, NPopconfirm, NModal, NInput, NDropdown, NSelect,
  useMessage,
  type DataTableColumns,
  type DataTableRowKey,
} from 'naive-ui'
import { useTestCaseStore } from '../../stores/testCase'
import { CASE_TYPE_LABELS, type CaseType, type RunCaseResult, type TestCase } from '../../types'
import AssertionEditor from './AssertionEditor.vue'
import RunResultPanel from './RunResultPanel.vue'

const props = defineProps<{
  /** 当前接口 id（0 表示无激活接口） */
  requestId: number
}>()

const testCaseStore = useTestCaseStore()
const message = useMessage()

// ── 数据 ──────────────────────────────────────────────────────

const cases = computed<TestCase[]>(() => {
  if (!props.requestId) return []
  return testCaseStore.getByRequestId(props.requestId)
})

// ── 筛选（1.0.5 类型；1.0.10 显示状态）────────────────────────
const typeFilter = ref<CaseType | null>(null)

const typeFilterOptions = Object.entries(CASE_TYPE_LABELS).map(([value, label]) => ({
  value, label,
}))

type ViewFilter = 'all' | 'visible' | 'hidden'
const viewFilter = ref<ViewFilter>('all')

const viewFilterOptions: Array<{ value: ViewFilter; label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'visible', label: '仅显示' },
  { value: 'hidden', label: '仅隐藏' },
]

const filteredCases = computed<TestCase[]>(() => {
  let list = cases.value
  if (typeFilter.value != null) {
    list = list.filter(c => c.case_type === typeFilter.value)
  }
  if (viewFilter.value === 'visible') {
    list = list.filter(c => c.hidden !== 1)
  } else if (viewFilter.value === 'hidden') {
    list = list.filter(c => c.hidden === 1)
  }
  return list
})

// ── 1.0.5：跑用例 + 断言编辑 ───────────────────────────────
const showAssertionEditor = ref(false)
const editingAssertions = ref('[]')
const editingCaseId = ref<number | null>(null)

const showRunResult = ref(false)
const runResult = ref<RunCaseResult | null>(null)
const runningCaseId = ref<number | null>(null)

function openAssertionEditor(row: TestCase) {
  editingCaseId.value = row.id
  editingAssertions.value = row.assertions || '[]'
  showAssertionEditor.value = true
}

async function onSaveAssertions(json: string) {
  if (editingCaseId.value == null) return
  try {
    await testCaseStore.updateTestCase(editingCaseId.value, { assertions: json })
    message.success('断言已保存')
  } catch (e) {
    message.error(`保存断言失败：${e}`)
  }
}

async function runCase(row: TestCase) {
  if (runningCaseId.value != null) return
  runningCaseId.value = row.id
  try {
    // 1.0.10：store 内部会把本次响应写入下方响应区并刷新 History
    const result = await testCaseStore.runTestCase(row.id)
    runResult.value = result
    showRunResult.value = true
  } catch (e) {
    message.error(`执行失败：${e}`)
  } finally {
    runningCaseId.value = null
  }
}

// 多选（批量删除 / 批量隐藏）
const checkedIds = ref<DataTableRowKey[]>([])

// ── 行配置 ────────────────────────────────────────────────────

function rowKey(row: TestCase): number {
  return row.id
}

function rowClassName(row: TestCase): string {
  return row.hidden === 1 ? 'hidden-row' : ''
}

function rowProps(row: TestCase) {
  return {
    onContextmenu: (e: MouseEvent) => openContextMenu(e, row),
    onDblclick: (e: MouseEvent) => {
      e.stopPropagation()
      startRename(row)
    },
  }
}

/** 解析用例的断言数量（0 = 未配置） */
function assertionCount(row: TestCase): number {
  try {
    const arr = JSON.parse(row.assertions || '[]')
    return Array.isArray(arr) ? arr.length : 0
  } catch { return 0 }
}

const columns: DataTableColumns<TestCase> = [
  { type: 'selection', width: 36 },
  {
    title: '名称',
    key: 'name',
    minWidth: 120,
    render: (row) => h('span', { class: 'cell-name' }, [
      h('span', { class: `type-badge type-${row.case_type}` }, CASE_TYPE_LABELS[row.case_type] ?? row.case_type),
      row.name,
      row.hidden === 1
        ? h('span', { class: 'hidden-badge', title: '不在下方用例条展示' }, '隐藏')
        : null,
      (() => {
        const n = assertionCount(row)
        return n > 0
          ? h('span', {
              class: 'assert-count',
              title: `${n} 条断言（点击编辑）`,
              onClick: (e: MouseEvent) => { e.stopPropagation(); openAssertionEditor(row) },
            }, ` [${n}]`)
          : null
      })(),
    ]),
    sorter: (a, b) => a.name.localeCompare(b.name),
  },
  {
    title: '状态',
    key: 'last_status',
    width: 62,
    render: (row) => {
      const map: Record<string, { icon: string; cls: string }> = {
        passed: { icon: '✓', cls: 'st-passed' },
        failed: { icon: '✗', cls: 'st-failed' },
        error: { icon: '⚠', cls: 'st-error' },
        pending: { icon: '·', cls: 'st-pending' },
      }
      const s = map[row.last_status] ?? map.pending
      return h('span', { class: `status-dot ${s.cls}`, title: row.last_status }, s.icon)
    },
  },
  {
    title: '',
    key: 'actions',
    width: 84,
    render: (row) => h('span', { class: 'action-btns' }, [
      h('button', {
        class: 'run-btn',
        title: '运行用例（含断言）',
        disabled: runningCaseId.value != null,
        onClick: (e: MouseEvent) => { e.stopPropagation(); runCase(row) },
      }, runningCaseId.value === row.id ? '…' : '▶'),
      h('button', {
        class: 'run-btn',
        title: '编辑断言',
        onClick: (e: MouseEvent) => { e.stopPropagation(); openAssertionEditor(row) },
      }, '⚖'),
      h('button', {
        class: 'run-btn',
        title: '重命名',
        onClick: (e: MouseEvent) => { e.stopPropagation(); startRename(row) },
      }, '✎'),
    ]),
  },
  {
    title: '更新时间',
    key: 'updated_at',
    width: 110,
    render: (row) => formatRelative(row.updated_at),
    sorter: (a, b) => Date.parse(a.updated_at) - Date.parse(b.updated_at),
    defaultSortOrder: 'descend',
  },
]

function formatRelative(s: string): string {
  const t = Date.parse(s + (s.endsWith('Z') ? '' : 'Z'))
  if (Number.isNaN(t)) return s
  const diff = Date.now() - t
  if (diff < 60_000) return '刚刚'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`
  if (diff < 30 * 86_400_000) return `${Math.floor(diff / 86_400_000)} 天前`
  const d = new Date(t)
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

// ── 行为 ──────────────────────────────────────────────────────

async function onBatchDelete() {
  const ids = checkedIds.value.map(Number)
  if (ids.length === 0) return
  try {
    const affected = await testCaseStore.deleteTestCases(ids)
    message.success(`已删除 ${affected} 个用例`)
    checkedIds.value = []
  } catch (e) {
    message.error(`删除失败：${e}`)
  }
}

/** 批量隐藏/取消隐藏：只影响下方用例条，管理面板始终可见 */
async function onBatchHide(hidden: boolean) {
  const ids = checkedIds.value.map(Number)
  if (ids.length === 0) return
  try {
    const affected = await testCaseStore.setTestCasesHidden(ids, hidden)
    message.success(hidden ? `已隐藏 ${affected} 个用例` : `已取消隐藏 ${affected} 个用例`)
    checkedIds.value = []
  } catch (e) {
    message.error(`${hidden ? '隐藏' : '取消隐藏'}失败：${e}`)
  }
}

// ── 重命名 ────────────────────────────────────────────────────

const showRenameModal = ref(false)
const renameInput = ref('')
const renameTargetId = ref<number | null>(null)
const renameInputRef = ref<{ focus: () => void } | null>(null)

function startRename(row: TestCase) {
  renameTargetId.value = row.id
  renameInput.value = row.name
  showRenameModal.value = true
  nextTick(() => renameInputRef.value?.focus())
}

async function confirmRename() {
  const id = renameTargetId.value
  const name = renameInput.value.trim()
  if (id === null || !name) {
    showRenameModal.value = false
    return
  }
  try {
    await testCaseStore.updateTestCase(id, { name })
    message.success('已重命名')
  } catch (e) {
    message.error(`重命名失败：${e}`)
  }
  showRenameModal.value = false
}

// ── 右键菜单 ──────────────────────────────────────────────────

const contextMenuVisible = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextTargetId = ref<number | null>(null)

const contextMenuOptions = computed(() => {
  if (contextTargetId.value === null) return []
  const tc = cases.value.find(c => c.id === contextTargetId.value)
  if (!tc) return []
  return [
    { label: '▶ 运行用例', key: 'run' },
    { label: '编辑断言', key: 'assertions' },
    { type: 'divider' as const, key: 'd0' },
    { label: '重命名', key: 'rename' },
    { type: 'divider' as const, key: 'd1' },
    { label: '删除', key: 'delete' },
  ]
})

function openContextMenu(e: MouseEvent, row: TestCase) {
  e.preventDefault()
  contextTargetId.value = row.id
  contextMenuX.value = e.clientX
  contextMenuY.value = e.clientY
  contextMenuVisible.value = true
}

async function handleContextMenuSelect(key: string) {
  contextMenuVisible.value = false
  const id = contextTargetId.value
  if (id === null) return
  const tc = cases.value.find(c => c.id === id)
  if (!tc) return

  if (key === 'run') {
    runCase(tc)
  } else if (key === 'assertions') {
    openAssertionEditor(tc)
  } else if (key === 'rename') {
    startRename(tc)
  } else if (key === 'delete') {
    try {
      await testCaseStore.deleteTestCase(id)
      message.success('已删除')
    } catch (e) {
      message.error(`删除失败：${e}`)
    }
  }
}

// ── 跨接口切换：重置选中与筛选 ────────────────────────────────

watch(() => props.requestId, () => {
  checkedIds.value = []
  typeFilter.value = null
  viewFilter.value = 'all'
})

// 列表数据或筛选变化时，清掉已不可见行的勾选（批量操作只作用于可见行）
watch(() => filteredCases.value.map(c => c.id), (ids) => {
  const visible = new Set(ids)
  checkedIds.value = checkedIds.value.filter(id => visible.has(Number(id)))
})
</script>

<style scoped>
.testcase-manager {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.manager-body {
  flex: 1;
  display: flex;
  overflow: hidden;
  padding: 8px 4px 4px;
  min-height: 0;
}

/* 单列表面板（原左右分栏已移除） */
.case-list {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
}

/* 筛选工具条 */
.list-toolbar {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding: var(--spacing-xs) var(--spacing-sm);
  border-bottom: 1px solid var(--border-base);
  flex-shrink: 0;
}

.list-toolbar .type-filter {
  flex: 1;
}

.list-toolbar .view-filter {
  width: 88px;
}

/* NDataTable 在 flex-height 模式下需要显式高度才能正确计算 tbody 滚动容器
 * （否则 wrapper.height = NDataTable.height - thead.height = 0，tbody 不可见）。
 * 用 flex:1 + min-height:0 让它撑满父容器分配的剩余高度。 */
.case-list :deep(.case-table) {
  flex: 1;
  min-height: 0;
}

/* 修复 Linux WebKitGTK flex-height 高度为 0 的问题：
   强制 NDataTable 的 scroll-wrapper 占满父容器 */
.case-list :deep(.n-data-table) {
  height: 100%;
}
.case-list :deep(.n-data-table .n-data-table__main) {
  height: 100%;
}
.case-list :deep(.n-data-table .n-data-table-base-table) {
  height: 100%;
}

.empty {
  padding: 24px 16px;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 13px;
  line-height: 1.8;
}

.empty .hint {
  font-size: 12px;
  color: var(--text-disabled);
}

/* 底部操作栏 */
.manager-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  border-top: 1px solid var(--border-base);
  background: var(--bg-surface);
  flex-shrink: 0;
}

.footer-info {
  font-size: 12px;
  color: var(--text-secondary);
}

.footer-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}
</style>

<!-- 全局样式：隐藏行置灰 + 类型 badge（NDataTable render 函数产物拿不到 scoped 属性） -->
<style>
.testcase-manager .case-list .n-data-table-tr.hidden-row > td {
  opacity: 0.55;
}

.testcase-manager .case-list .cell-name .type-badge {
  display: inline-block;
  font-size: 10px;
  line-height: 1;
  padding: 2px 4px;
  border-radius: 3px;
  margin-right: 4px;
  vertical-align: 1px;
  white-space: nowrap;
  background: var(--bg-hover);
  color: var(--text-secondary);
  border: 1px solid var(--border-base);
}

.testcase-manager .case-list .type-happy_path {
  background: color-mix(in srgb, var(--color-success, #18a058) 12%, transparent);
  color: var(--color-success, #18a058);
  border-color: color-mix(in srgb, var(--color-success, #18a058) 30%, transparent);
}

.testcase-manager .case-list .type-missing_required,
.testcase-manager .case-list .type-type_error {
  background: color-mix(in srgb, var(--color-warning, #f0a020) 12%, transparent);
  color: var(--color-warning, #f0a020);
  border-color: color-mix(in srgb, var(--color-warning, #f0a020) 30%, transparent);
}

.testcase-manager .case-list .type-unauthorized,
.testcase-manager .case-list .type-invalid_chars {
  background: color-mix(in srgb, var(--color-error, #d03050) 12%, transparent);
  color: var(--color-error, #d03050);
  border-color: color-mix(in srgb, var(--color-error, #d03050) 30%, transparent);
}

.testcase-manager .case-list .type-boundary,
.testcase-manager .case-list .type-empty_list {
  background: color-mix(in srgb, var(--color-info, #2080f0) 12%, transparent);
  color: var(--color-info, #2080f0);
  border-color: color-mix(in srgb, var(--color-info, #2080f0) 30%, transparent);
}

/* 1.0.10：隐藏标识 */
.testcase-manager .case-list .hidden-badge {
  display: inline-block;
  font-size: var(--font-size-sm);
  line-height: 1;
  padding: 0 var(--spacing-xs);
  border-radius: var(--radius-sm);
  margin-left: var(--spacing-xs);
  vertical-align: 1px;
  white-space: nowrap;
  background: var(--bg-hover);
  color: var(--text-tertiary);
  border: 1px dashed var(--border-base);
}

/* 1.0.5：执行状态点 */
.testcase-manager .case-list .status-dot {
  display: inline-block;
  width: 16px;
  text-align: center;
  font-size: 12px;
  line-height: 1;
}
.testcase-manager .case-list .st-passed { color: var(--color-success, #18a058); }
.testcase-manager .case-list .st-failed { color: var(--color-error, #d03050); }
.testcase-manager .case-list .st-error  { color: var(--color-warning, #f0a020); }
.testcase-manager .case-list .st-pending { color: var(--text-disabled); }

/* 1.0.5：运行/断言/重命名按钮 */
.testcase-manager .case-list .run-btn {
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: 11px;
  color: var(--text-secondary);
  padding: 2px 4px;
  border-radius: 3px;
  line-height: 1;
}
.testcase-manager .case-list .run-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--color-primary, #18a058);
}
.testcase-manager .case-list .run-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* 断言数量标记 */
.testcase-manager .case-list .assert-count {
  font-size: 10px;
  color: var(--color-info, #2080f0);
  font-weight: 600;
}

/* 操作按钮组 */
.testcase-manager .case-list .action-btns {
  display: inline-flex;
  gap: 2px;
}
</style>
