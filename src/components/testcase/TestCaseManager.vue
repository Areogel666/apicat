<template>
  <div class="testcase-manager">
    <!-- 左右分栏主体 -->
    <div class="manager-body">
      <!-- 左：用例列表（NDataTable） -->
      <div class="case-list">
        <div v-if="cases.length === 0" class="empty">
          当前接口暂无用例。<br>
          <span class="hint">发起一次请求会自动生成用例。</span>
        </div>
        <n-data-table
          v-else
          :columns="columns"
          :data="cases"
          :row-key="rowKey"
          v-model:checked-row-keys="checkedIds"
          :pagination="false"
          :flex-height="true"
          :row-class-name="rowClassName"
          :row-props="rowProps"
          size="small"
        />
      </div>

      <!-- 右：历史详情 -->
      <div class="history-pane">
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
  </div>
</template>

<script setup lang="ts">
import { computed, h, ref, watch, nextTick } from 'vue'
import {
  NDataTable, NButton, NPopconfirm, NModal, NInput, NDropdown,
  useMessage,
  type DataTableColumns,
  type DataTableRowKey,
} from 'naive-ui'
import { useTestCaseStore } from '../../stores/testCase'
import type { TestCase } from '../../types'
import HistoryItem from './HistoryItem.vue'

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
      row.name,
    ]),
    sorter: (a, b) => a.name.localeCompare(b.name),
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

  if (key === 'rename') {
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

// ── 跨接口切换：重置选中和高亮 ─────────────────────────────────

watch(() => props.requestId, () => {
  focusedId.value = null
  checkedIds.value = []
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

/* 左列：用例列表 60% */
.case-list {
  flex: 1 1 60%;
  min-width: 320px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
}

/* 右列：历史详情 40% */
.history-pane {
  flex: 1 1 40%;
  min-width: 280px;
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

<!-- 全局样式：用例列表选中行高亮（NDataTable 的 row-class-name 会渲染到 tr） -->
<style>
.testcase-manager .case-list .n-data-table-tr.focused-row > td {
  background-color: var(--bg-selected) !important;
}

.testcase-manager .case-list .cell-name .star {
  font-size: 12px;
  margin-right: 2px;
}
</style>
