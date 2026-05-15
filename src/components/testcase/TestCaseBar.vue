<template>
  <div class="test-case-bar" v-if="testCases.length > 0 || alwaysShow">
    <!-- 参数变更提示条（非阻塞，响应返回后出现）-->
    <div v-if="paramsDirty" class="dirty-bar">
      <span class="dirty-label">📋 当前参数与「{{ activeTestCase?.name }}」不同</span>
      <n-button size="tiny" type="primary" ghost @click="emit('save-to-active')">保存到用例</n-button>
      <n-button size="tiny" @click="emit('save-as-new')">另存为新用例</n-button>
      <n-button size="tiny" quaternary @click="emit('dismiss-dirty')">忽略</n-button>
    </div>

    <!-- 用例 Tab 栏 -->
    <div class="case-tabs-row">
      <div
        v-for="tc in testCases"
        :key="tc.id"
        :class="['case-tab', tc.id === activeId && 'active']"
        @click="handleTabClick(tc.id)"
        @dblclick.stop="startInlineRename(tc)"
        @contextmenu.prevent="openMenu(tc, $event)"
      >
        <span v-if="tc.starred" class="star">⭐</span>
        <!-- 双击时显示内联编辑输入框 -->
        <template v-if="renamingId === tc.id">
          <n-input
            :ref="el => { if (el) renameInputRef = el }"
            v-model:value="renameValue"
            size="tiny"
            style="width: 90px"
            @keyup.enter="commitInlineRename"
            @keyup.escape="cancelInlineRename"
            @blur="commitInlineRename"
            @click.stop
          />
        </template>
        <span v-else class="case-name">{{ tc.name }}</span>
      </div>

      <!-- 新建用例 -->
      <div class="case-tab case-tab-add" v-if="!showNewInput" @click="showNewInput = true">
        + 新建
      </div>
      <div v-else class="new-case-input-row">
        <n-input
          ref="newInputRef"
          v-model:value="newName"
          size="tiny"
          style="width: 90px"
          placeholder="用例名称"
          @keyup.enter="confirmCreate"
          @keyup.escape="showNewInput = false"
        />
        <n-button size="tiny" type="primary" @click="confirmCreate">确定</n-button>
        <n-button size="tiny" quaternary @click="showNewInput = false">✕</n-button>
      </div>
    </div>

    <!-- 右键菜单 -->
    <n-dropdown
      :show="menuVisible"
      :x="menuX"
      :y="menuY"
      :options="menuOptions"
      placement="bottom-start"
      @clickoutside="menuVisible = false"
      @select="handleMenuSelect"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue'
import { NButton, NInput, NDropdown } from 'naive-ui'
import type { TestCase } from '../../types'

const props = defineProps<{
  testCases: TestCase[]
  activeId: number | null
  paramsDirty: boolean
  alwaysShow?: boolean
}>()

const emit = defineEmits<{
  activate: [id: number]
  create: [name: string]
  rename: [id: number, name: string]
  'toggle-star': [id: number]
  delete: [id: number]
  'save-to-active': []
  'save-as-new': []
  'dismiss-dirty': []
}>()

const activeTestCase = computed(() => props.testCases.find(t => t.id === props.activeId) ?? null)

// ── 新建输入 ───────────────────────────────────────────────
const showNewInput = ref(false)
const newName = ref('')
const newInputRef = ref()

watch(showNewInput, (v) => {
  if (v) nextTick(() => newInputRef.value?.focus())
})

function confirmCreate() {
  const name = newName.value.trim()
  if (!name) return
  emit('create', name)
  newName.value = ''
  showNewInput.value = false
}

// ── 单击激活（防与双击冲突：延迟100ms，如果双击则取消）──────
let clickTimer: ReturnType<typeof setTimeout> | null = null

function handleTabClick(id: number) {
  // 若正在重命名该 tab，不触发 activate（防止 blur 冲突）
  if (renamingId.value === id) return
  if (clickTimer) clearTimeout(clickTimer)
  clickTimer = setTimeout(() => {
    emit('activate', id)
    clickTimer = null
  }, 200)
}

// ── 内联双击重命名 ──────────────────────────────────────────
const renamingId = ref<number | null>(null)
const renameValue = ref('')
const renameInputRef = ref<any>(null)

function startInlineRename(tc: TestCase) {
  // 取消单击的 activate
  if (clickTimer) { clearTimeout(clickTimer); clickTimer = null }
  renamingId.value = tc.id
  renameValue.value = tc.name
  nextTick(() => {
    renameInputRef.value?.focus?.()
    renameInputRef.value?.select?.()
  })
}

function commitInlineRename() {
  const id = renamingId.value
  if (id === null) return
  const name = renameValue.value.trim()
  if (name) emit('rename', id, name)
  renamingId.value = null
}

function cancelInlineRename() {
  renamingId.value = null
}

// ── 右键菜单 ───────────────────────────────────────────────
const menuVisible = ref(false)
const menuX = ref(0)
const menuY = ref(0)
const menuTargetId = ref<number | null>(null)

const menuOptions = computed(() => {
  if (menuTargetId.value === null) return []
  const tc = props.testCases.find(t => t.id === menuTargetId.value)
  return [
    { label: '重命名', key: 'rename' },
    { label: tc?.starred ? '取消收藏 ⭐' : '收藏 ⭐', key: 'star' },
    { label: '删除', key: 'delete' },
  ]
})

function openMenu(tc: TestCase, e: MouseEvent) {
  menuTargetId.value = tc.id
  menuX.value = e.clientX
  menuY.value = e.clientY
  menuVisible.value = true
}

function handleMenuSelect(key: string) {
  menuVisible.value = false
  const id = menuTargetId.value
  if (id === null) return
  if (key === 'rename') {
    // 触发内联重命名（通过 startInlineRename）
    const tc = props.testCases.find(t => t.id === id)
    if (tc) startInlineRename(tc)
  } else if (key === 'star') {
    emit('toggle-star', id)
  } else if (key === 'delete') {
    emit('delete', id)
  }
}
</script>

<style scoped>
.test-case-bar {
  border-top: 1px solid var(--border-base);
  background: var(--bg-elevated);
  flex-shrink: 0;
}

.dirty-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  background: var(--color-warning);
  border-bottom: 1px solid var(--color-warning);
  font-size: 12px;
}
.dirty-label { flex: 1; color: var(--text-primary); }

.case-tabs-row {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px 8px;
  overflow-x: auto;
  scrollbar-width: thin;
}

.case-tab {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 3px 10px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
  color: var(--text-tertiary);
  border: 1px solid transparent;
  transition: background 0.1s;
}
.case-tab:hover { background: var(--bg-hover); color: var(--text-primary); }
.case-tab.active {
  background: var(--color-primary-soft);
  border-color: var(--color-primary);
  color: var(--color-primary);
}
.case-tab-add { color: var(--color-primary); font-weight: 500; }
.star { font-size: 10px; }
.case-name { max-width: 100px; overflow: hidden; text-overflow: ellipsis; }

.new-case-input-row { display: flex; align-items: center; gap: 4px; }
</style>
