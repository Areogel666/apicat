<template>
  <div class="tab-bar" @wheel.prevent="onWheel">
    <div class="tab-list" ref="tabListRef">
      <div
        v-for="tab in tabStore.openTabs"
        :key="tab.requestId"
        class="tab-item"
        :class="{ 'tab-item--active': tab.requestId === tabStore.activeRequestId }"
        @click="tabStore.activateTab(tab.requestId)"
        @contextmenu.prevent="openContextMenu($event, tab.requestId)"
      >
        <!-- dirty 圆点 -->
        <span
          v-if="requestStore.dirtyRequestIds.has(tab.requestId)"
          class="tab-dirty-dot"
        />
        <!-- 标题 -->
        <span class="tab-title" :title="tab.title">{{ tab.title }}</span>
        <!-- 关闭按钮（hover 显示） -->
        <button
          class="tab-close"
          @click.stop="handleCloseSingle(tab.requestId)"
          title="关闭"
        >✕</button>
      </div>
    </div>
  </div>

  <!-- 右键菜单 -->
  <Teleport to="body">
    <div
      v-if="ctxVisible"
      class="tab-ctx-menu"
      :style="{ top: ctxY + 'px', left: ctxX + 'px' }"
      @click.stop
    >
      <div class="tab-ctx-item" @click="handleCtxClose('current')">关闭当前</div>
      <div class="tab-ctx-item" @click="handleCtxClose('others')">关闭其他所有</div>
      <div class="tab-ctx-item" @click="handleCtxClose('left')">关闭左侧所有</div>
      <div class="tab-ctx-item" @click="handleCtxClose('right')">关闭右侧所有</div>
    </div>
  </Teleport>

  <!-- 单个 dirty Tab 关闭确认弹窗 -->
  <n-modal
    v-model:show="showSingleConfirm"
    preset="dialog"
    title="有未保存的修改"
    :show-icon="false"
  >
    <div style="font-size:14px; line-height:1.7">
      「{{ singleConfirmTitle }}」有未保存的修改，关闭后将丢失。
    </div>
    <template #action>
      <n-button @click="showSingleConfirm = false">取消</n-button>
      <n-button @click="confirmSingleClose(false)">直接关闭</n-button>
      <n-button type="primary" :loading="saving" @click="confirmSingleClose(true)">保存后关闭</n-button>
    </template>
  </n-modal>

  <!-- 批量关闭汇总确认弹窗 -->
  <n-modal
    v-model:show="showBatchConfirm"
    preset="dialog"
    title="以下接口有未保存的修改"
    :show-icon="false"
  >
    <div style="font-size:14px; line-height:1.8">
      <div v-for="title in batchDirtyTitles" :key="title" style="padding-left:8px">
        · {{ title }}
      </div>
    </div>
    <template #action>
      <n-button @click="showBatchConfirm = false">取消</n-button>
      <n-button @click="confirmBatchClose(false)">全部直接关闭</n-button>
      <n-button type="primary" :loading="saving" @click="confirmBatchClose(true)">全部保存后关闭</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { NModal, NButton, useMessage } from 'naive-ui'
import { useTabStore } from '../../stores/tab'
import { useRequestStore } from '../../stores/request'

const tabStore = useTabStore()
const requestStore = useRequestStore()
const message = useMessage()

// ── 横向滚动（鼠标滚轮 + Mac 触控板） ────────────────────
const tabListRef = ref<HTMLElement | null>(null)
function onWheel(e: WheelEvent) {
  if (tabListRef.value) {
    // 优先使用 deltaX（Mac 触控板横向手势），deltaX 为 0 时才用 deltaY（鼠标竖向滚轮）
    tabListRef.value.scrollLeft += e.deltaX || e.deltaY
  }
}

// ── 右键菜单 ──────────────────────────────────────────────
const ctxVisible = ref(false)
const ctxX = ref(0)
const ctxY = ref(0)
const ctxTargetId = ref<number | null>(null)

function openContextMenu(e: MouseEvent, requestId: number) {
  ctxTargetId.value = requestId
  ctxX.value = e.clientX
  ctxY.value = e.clientY
  ctxVisible.value = true
}

function closeCtxMenu() { ctxVisible.value = false }
onMounted(() => document.addEventListener('click', closeCtxMenu))
onUnmounted(() => document.removeEventListener('click', closeCtxMenu))

// ── 单个关闭确认 ──────────────────────────────────────────
const showSingleConfirm = ref(false)
const singleConfirmTitle = ref('')
const singleTargetId = ref<number | null>(null)
const saving = ref(false)

function handleCloseSingle(requestId: number) {
  if (requestStore.dirtyRequestIds.has(requestId)) {
    const tab = tabStore.openTabs.find(t => t.requestId === requestId)
    singleConfirmTitle.value = tab?.title ?? ''
    singleTargetId.value = requestId
    showSingleConfirm.value = true
  } else {
    tabStore.closeTab(requestId)
  }
}

async function confirmSingleClose(saveFirst: boolean) {
  const id = singleTargetId.value
  if (id === null) return
  saving.value = true
  try {
    if (saveFirst) {
      await requestStore.saveRequest(id)
    }
    // 若是批量操作（batchTargetIds 有值），关闭所有目标；否则只关闭单个
    if (batchTargetIds.value.length > 0) {
      ;[...batchTargetIds.value].reverse().forEach(bid => tabStore.closeTab(bid))
      batchTargetIds.value = []
    } else {
      tabStore.closeTab(id)
    }
    showSingleConfirm.value = false
  } catch (e) {
    message.error('保存失败：' + String(e))
  } finally {
    saving.value = false
  }
}

// ── 批量关闭确认 ──────────────────────────────────────────
const showBatchConfirm = ref(false)
const batchDirtyTitles = ref<string[]>([])
// 批量关闭时待关闭的所有 ID（含 dirty 和 clean）
const batchTargetIds = ref<number[]>([])

/**
 * 通用批量关闭入口：
 * - 无 dirty Tab → 直接关闭
 * - 1 个 dirty Tab → 单个确认弹窗
 * - 多个 dirty Tab → 汇总确认弹窗
 */
function handleBatchClose(ids: number[]) {
  if (ids.length === 0) return
  const dirtyIds = ids.filter(id => requestStore.dirtyRequestIds.has(id))

  if (dirtyIds.length === 0) {
    // 从后往前关闭避免索引错位
    ;[...ids].reverse().forEach(id => tabStore.closeTab(id))
  } else if (dirtyIds.length === 1) {
    const tab = tabStore.openTabs.find(t => t.requestId === dirtyIds[0])
    singleConfirmTitle.value = tab?.title ?? ''
    singleTargetId.value = dirtyIds[0]
    batchTargetIds.value = ids
    showSingleConfirm.value = true
  } else {
    batchTargetIds.value = ids
    batchDirtyTitles.value = dirtyIds.map(id => {
      return tabStore.openTabs.find(t => t.requestId === id)?.title ?? String(id)
    })
    showBatchConfirm.value = true
  }
}

async function confirmBatchClose(saveFirst: boolean) {
  saving.value = true
  try {
    if (saveFirst) {
      const dirtyIds = batchTargetIds.value.filter(id =>
        requestStore.dirtyRequestIds.has(id)
      )
      await Promise.all(dirtyIds.map(id => requestStore.saveRequest(id)))
    }
    // 批量关闭：从后往前关闭避免索引错位
    ;[...batchTargetIds.value].reverse().forEach(id => tabStore.closeTab(id))
    batchTargetIds.value = []
    showBatchConfirm.value = false
  } catch (e) {
    message.error('保存失败：' + String(e))
  } finally {
    saving.value = false
  }
}

// ── 右键菜单操作路由 ──────────────────────────────────────
function handleCtxClose(action: 'current' | 'others' | 'left' | 'right') {
  closeCtxMenu()
  const id = ctxTargetId.value
  if (id === null) return

  const all = tabStore.openTabs.map(t => t.requestId)
  const idx = all.indexOf(id)

  let targets: number[]
  switch (action) {
    case 'current':
      targets = [id]
      break
    case 'others':
      targets = all.filter(rid => rid !== id)
      break
    case 'left':
      targets = all.slice(0, idx)
      break
    case 'right':
      targets = all.slice(idx + 1)
      break
    default:
      targets = []
  }
  handleBatchClose(targets)
}
</script>

<style scoped>
.tab-bar {
  height: 36px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-base);
  background: var(--bg-surface);
  overflow: hidden;
}

.tab-list {
  display: flex;
  height: 100%;
  overflow-x: auto;
  scrollbar-width: none; /* 隐藏滚动条，用鼠标滚轮控制 */
}
.tab-list::-webkit-scrollbar { display: none; }

.tab-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 0 12px;
  height: 100%;
  min-width: 80px;
  max-width: 180px;
  flex-shrink: 0;
  cursor: pointer;
  border-right: 1px solid var(--border-base);
  font-size: 12px;
  color: var(--text-tertiary);
  user-select: none;
  position: relative;
  transition: background 0.1s, color 0.1s;
}
.tab-item:hover { background: var(--bg-hover); color: var(--text-primary); }
.tab-item--active {
  background: var(--bg-elevated);
  color: var(--text-primary);
  border-bottom: 2px solid var(--color-primary);
}

.tab-dirty-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--color-warning);
  flex-shrink: 0;
}

.tab-title {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab-close {
  all: unset;
  width: 16px;
  height: 16px;
  border-radius: 3px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  color: var(--text-disabled);
  cursor: pointer;
  flex-shrink: 0;
  opacity: 0;       /* 默认隐藏 */
  transition: opacity 0.1s, background 0.1s;
}
.tab-item:hover .tab-close { opacity: 1; }
.tab-close:hover { background: rgba(208, 48, 80, 0.12); color: var(--color-error); }

/* 右键菜单（Teleport 到 body，不受 scoped 影响，使用 :global 等价写法） */
</style>

<!-- 右键菜单样式必须全局：通过 <Teleport to="body"> 渲染，DOM 挂载在 body 下，
     scoped CSS 哈希不匹配，因此需要全局样式。
     类名已加 "tab-ctx-" 前缀，降低与其他全局样式冲突的风险。 -->
<style>
/* TabBar 右键菜单（Teleport 到 body，需全局样式） */
.tab-ctx-menu {
  position: fixed;
  z-index: 9999;
  background: var(--bg-elevated);
  border: 1px solid var(--border-base);
  border-radius: 6px;
  box-shadow: var(--shadow-md);
  padding: 4px 0;
  min-width: 140px;
  font-size: 13px;
}
.tab-ctx-item {
  padding: 7px 14px;
  cursor: pointer;
  color: var(--text-primary);
  user-select: none;
  transition: background 0.1s;
}
.tab-ctx-item:hover { background: var(--bg-hover); }
</style>
