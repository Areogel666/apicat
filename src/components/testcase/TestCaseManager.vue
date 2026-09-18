<template>
  <div class="testcase-manager">
    <!-- 左右分栏主体 -->
    <div class="manager-body">
      <!-- 左：用例列表（NDataTable） -->
      <div class="case-list" :style="{ flex: `0 0 ${caseListWidth}px`, minWidth: '200px' }">
        <div v-if="cases.length === 0" class="empty">
          当前接口暂无用例。<br>
          <span class="hint">发起一次请求会自动生成用例。</span>
        </div>
        <template v-else>
          <div class="list-toolbar">
            <n-select
              v-model:value="typeFilter"
              :options="typeFilterOptions"
              size="tiny"
              placeholder="全部类型"
              clearable
              class="type-filter"
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
        </template>
      </div>

      <ResizableSplitter
        direction="horizontal"
        :default-size="caseListWidth"
        :min-size="200"
        :max-size="600"
        storage-key="layout.testcaseLeftWidth"
        @resize="(w: number) => caseListWidth = w"
      />

      <!-- 右：历史详情 -->
      <div class="history-pane" style="flex: 1; min-width: 280px">
        <template v-if="!focusedCase">
          <div class="empty">从左侧选择一个用例查看历史</div>
        </template>
        <template v-else>
          <header class="history-header">
            <div class="history-title">
              <span v-if="focusedCase.starred === 1" class="star">⭐</span>
              <span class="case-name">{{ focusedCase.name }}</span>
            </div>
            <n-button size="tiny" quaternary :loading="loadingHistory" @click="reloadHistory">
              ↻ 刷新
            </n-button>
          </header>
          <div v-if="historyList.length === 0" class="empty">
            暂无历史调用。<br>
            <span class="hint">激活此用例后发起请求会记录到这里（最多保留最近 10 次）。</span>
          </div>
          <div v-else class="history-list">
            <HistoryItem v-for="h in historyList" :key="h.id" :record="h" />
          </div>
        </template>
      </div>
    </div>

    <!-- 底部操作栏 -->
    <footer class="manager-footer">
      <span class="footer-info">
        <template v-if="checkedIds.length > 0">
          已选 {{ checkedIds.length }} / {{ cases.length }}
        </template>
        <template v-else>
          共 {{ cases.length }} 个用例
        </template>
      </span>
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
    </footer>

    <!-- 重命名弹窗（右键菜单触发） -->
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
import HistoryItem from './HistoryItem.vue'
import ResizableSplitter from '../common/ResizableSplitter.vue'
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

// ── 类型筛选（1.0.5）────────────────────────────────────────
const typeFilter = ref<CaseType | null>(null)

const typeFilterOptions = Object.entries(CASE_TYPE_LABELS).map(([value, label]) => ({
  value, label,
}))

const filteredCases = computed<TestCase[]>(() =>
  typeFilter.value == null
    ? cases.value
    : cases.value.filter(c => c.case_type === typeFilter.value),
)

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
    const result = await testCaseStore.runTestCase(row.id)
    runResult.value = result
    showRunResult.value = true
  } catch (e) {
    message.error(`执行失败：${e}`)
  } finally {
    runningCaseId.value = null
  }
}

// 多选（批量删除目标）
const checkedIds = ref<DataTableRowKey[]>([])

// 高亮（右栏要展示哪个用例的历史）—— 与 checkedIds 完全正交
const focusedId = ref<number | null>(null)
const focusedCase = computed<TestCase | null>(() =>
  cases.value.find(c => c.id === focusedId.value) ?? null
)

const historyList = computed(() => {
  if (focusedId.value === null) return []
  return testCaseStore.historyMap[focusedId.value] ?? []
})

const loadingHistory = ref(false)

// 分栏拖拽：用例列表宽度
const caseListWidth = ref(
  Number(localStorage.getItem('layout.testcaseLeftWidth') ?? 320)
)

// ── 行配置 ────────────────────────────────────────────────────

function rowKey(row: TestCase): number {
  return row.id
}

function rowClassName(row: TestCase): string {
  return row.id === focusedId.value ? 'focused-row' : ''
}

function rowProps(row: TestCase) {
  return {
    style: 'cursor: pointer',
    onClick: () => focusCase(row.id),
    onContextmenu: (e: MouseEvent) => openContextMenu(e, row),
    onDblclick: (e: MouseEvent) => {
      // 双击行 → 进入重命名（避免触发 focusCase 切换）
      e.stopPropagation()
      startRename(row)
    },
  }
}

const columns: DataTableColumns<TestCase> = [
  { type: 'selection', width: 36 },
  {
    title: '名称',
    key: 'name',
    minWidth: 120,
    render: (row) => h('span', { class: 'cell-name' }, [
      row.starred === 1 ? h('span', { class: 'star' }, '⭐ ') : null,
      h('span', { class: `type-badge type-${row.case_type}` }, CASE_TYPE_LABELS[row.case_type] ?? row.case_type),
      row.name,
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
    width: 40,
    render: (row) => h(
      'button',
      {
        class: 'run-btn',
        title: '运行用例（含断言）',
        disabled: runningCaseId.value != null,
        onClick: (e: MouseEvent) => { e.stopPropagation(); runCase(row) },
      },
      runningCaseId.value === row.id ? '…' : '▶',
    ),
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

async function focusCase(id: number) {
  // 防止重复点击触发不必要的重新渲染
  if (focusedId.value === id) return
  focusedId.value = id
  // 懒加载历史（首次或切换时）
  if (!testCaseStore.historyMap[id]) {
    loadingHistory.value = true
    try {
      await testCaseStore.loadHistory(id)
    } catch (e) {
      message.error(`加载历史失败：${e}`)
    } finally {
      loadingHistory.value = false
    }
  }
}

async function reloadHistory() {
  if (focusedId.value === null) return
  loadingHistory.value = true
  try {
    await testCaseStore.loadHistory(focusedId.value)
  } catch (e) {
    message.error(`刷新失败：${e}`)
  } finally {
    loadingHistory.value = false
  }
}

async function onBatchDelete() {
  const ids = checkedIds.value.map(Number)
  if (ids.length === 0) return
  try {
    const affected = await testCaseStore.deleteTestCases(ids)
    message.success(`已删除 ${affected} 个用例`)
    checkedIds.value = []
    if (focusedId.value !== null && ids.includes(focusedId.value)) {
      focusedId.value = null
    }
  } catch (e) {
    message.error(`删除失败：${e}`)
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
    { label: tc.starred === 1 ? '取消收藏 ⭐' : '收藏 ⭐', key: 'star' },
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
  } else if (key === 'star') {
    try {
      await testCaseStore.updateTestCase(id, { starred: tc.starred === 1 ? 0 : 1 })
    } catch (e) {
      message.error(`操作失败：${e}`)
    }
  } else if (key === 'delete') {
    try {
      await testCaseStore.deleteTestCase(id)
      message.success('已删除')
      if (focusedId.value === id) focusedId.value = null
    } catch (e) {
      message.error(`删除失败：${e}`)
    }
  }
}

// ── 跨接口切换：重置选中、高亮、类型筛选 ──────────────────────

watch(() => props.requestId, () => {
  focusedId.value = null
  checkedIds.value = []
  typeFilter.value = null
})

// 筛选变化时，清掉已隐藏行的勾选（防止批量删除误删不可见行）
watch(typeFilter, () => {
  const visible = new Set(filteredCases.value.map(c => c.id))
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
  gap: 8px;
  overflow: hidden;
  padding: 8px 4px 4px;
  min-height: 0;
}

/* 左列：用例列表（宽度由分栏拖拽控制） */
.case-list {
  overflow: hidden;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
}

/* 类型筛选工具条 */
.list-toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border-base);
  flex-shrink: 0;
}

.list-toolbar .type-filter {
  flex: 1;
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

/* 右列：历史详情（宽度自适应剩余空间） */
.history-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
}

.history-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-base);
  flex-shrink: 0;
}

.history-title {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.history-title .star {
  font-size: 12px;
  flex-shrink: 0;
}

.history-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
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
  gap: 8px;
  padding: 6px 8px;
  border-top: 1px solid var(--border-base);
  background: var(--bg-surface);
  flex-shrink: 0;
}

.footer-info {
  font-size: 12px;
  color: var(--text-secondary);
}
</style>

<!-- 全局样式：用例列表选中行高亮 + 类型 badge（NDataTable render 函数产物拿不到 scoped 属性） -->
<style>
.testcase-manager .case-list .n-data-table-tr.focused-row > td {
  background-color: var(--bg-selected) !important;
}

.testcase-manager .case-list .cell-name .star {
  font-size: 12px;
  margin-right: 2px;
}

.testcase-manager .case-list .type-badge {
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

/* 1.0.5：运行按钮 */
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
  color: var(--color-primary, #2080f0);
}
.testcase-manager .case-list .run-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
