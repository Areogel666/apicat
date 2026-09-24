<template>
  <!-- 整体垂直布局：TopBar(固定) + 内容区(充满剩余) -->
  <div class="app-layout">
    <TopBar />
    <div class="app-body">
      <!-- 最左侧窄竖栏 —— 上层 Tab 切换「接口树 / 字典树 / 文档」 -->
      <div class="activity-bar">
        <button
          class="activity-item"
          :class="{ active: leftPanel === 'interface' }"
          title="接口树"
          @click="leftPanel = 'interface'"
        >🔗</button>
        <button
          class="activity-item"
          :class="{ active: leftPanel === 'dictionary' }"
          title="数据字典"
          @click="leftPanel = 'dictionary'"
        >📖</button>
        <button
          class="activity-item"
          :class="{ active: leftPanel === 'docs' }"
          title="接口文档"
          @click="leftPanel = 'docs'"
        >📄</button>
      </div>
      <!-- 左右面板均常驻(v-show)，切换时保留全部浏览状态；折叠时整栏收起 -->
      <!-- hoverOpen 为临时预览：仅显示当前 leftPanel 对应栏，不改变 collapsed/不持久化 -->
      <div class="side-pane" v-show="leftPanel === 'interface' && (!collapsed || hoverOpen)">
        <Sidebar :style="{ width: sidebarWidth + 'px' }" @mouseleave="closeHoverPreview" />
      </div>
      <div class="side-pane" v-show="leftPanel === 'dictionary' && (!collapsed || hoverOpen)">
        <DictionarySidebar :style="{ width: sidebarWidth + 'px' }" @mouseleave="closeHoverPreview" />
      </div>
      <div class="side-pane" v-show="leftPanel === 'docs' && (!collapsed || hoverOpen)">
        <DocsSidebar
          ref="docsSidebarRef"
          :style="{ width: sidebarWidth + 'px' }"
          @select="onDocSelect"
          @mouseleave="closeHoverPreview"
        />
      </div>
      <!-- 1.0.6：折叠把手（门把手式）—— 垂直居中骑在左侧栏外缘，常驻可见。
           hover 预览展开（不移除把手）；点击切换收起/展开；
           hover 时把手朝动作方向轻推（收起态向右「拉」、展开态向左「推」）作为方向暗示。 -->
      <button
        type="button"
        class="fold-handle"
        :class="{ 'is-expanded': !collapsed }"
        :style="{ left: handleLeft + 'px' }"
        :title="collapsed ? '展开左侧栏' : '收起左侧栏'"
        :aria-label="collapsed ? '展开左侧栏' : '收起左侧栏'"
        :aria-expanded="!collapsed"
        @mouseenter="openHoverPreview"
        @click="toggleSidebar"
      >
        <span class="fold-handle__grip"></span>
        <span class="fold-handle__grip"></span>
      </button>
      <ResizableSplitter
        v-show="!collapsed"
        direction="horizontal"
        :default-size="sidebarWidth"
        :min-size="160"
        :max-size="500"
        storage-key="layout.sidebarWidth"
        @resize="onSidebarResize"
      />
      <MainPanel v-show="leftPanel === 'interface'" style="flex: 1; min-width: 0" />
      <DictionaryJsonPanel v-show="leftPanel === 'dictionary'" style="flex: 1; min-width: 0" />
      <DocsPanel v-show="leftPanel === 'docs'" :file="selectedDocFile" style="flex: 1; min-width: 0" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import TopBar from './TopBar.vue'
import Sidebar from './Sidebar.vue'
import DictionarySidebar from './DictionarySidebar.vue'
import DictionaryJsonPanel from './DictionaryJsonPanel.vue'
import MainPanel from './MainPanel.vue'
import DocsSidebar from '../docs/DocsSidebar.vue'
import DocsPanel from '../docs/DocsPanel.vue'
import ResizableSplitter from '../common/ResizableSplitter.vue'

interface DocFile {
  relative_path: string
  name: string
  dir: string
  size: number
  modified_at: string
  absolute_path: string
}

const sidebarWidth = ref(Number(localStorage.getItem('layout.sidebarWidth') ?? 240))

// 1.0.5：左侧栏折叠收起（接口树/字典树/文档三栏统一），状态持久化
const collapsed = ref(localStorage.getItem('layout.sidebarCollapsed') === '1')
watch(collapsed, v => localStorage.setItem('layout.sidebarCollapsed', v ? '1' : '0'))

// 1.0.6：折叠把手（门把手式）—— hover 预览展开（移出收回），点击切换；把手常驻可见
const hoverOpen = ref(false)
function openHoverPreview() {
  if (collapsed.value) hoverOpen.value = true
}
// 移出 side-pane：仅收回预览，不改变 collapsed 持久化状态
function closeHoverPreview() {
  hoverOpen.value = false
}
// 点击把手：切换收起/展开并持久化
function toggleSidebar() {
  hoverOpen.value = false
  collapsed.value = !collapsed.value
}

// 活动栏宽度（与 .activity-bar 的 42px 保持一致）
const ACTIVITY_BAR_W = 42// 把手横向位置：骑在当前侧栏外缘（折叠时贴在活动栏右缘）
const handleLeft = computed(() => {
  const showing = !collapsed.value || hoverOpen.value
  return ACTIVITY_BAR_W + (showing ? sidebarWidth.value : 0)
})

// 左侧内容面板切换（接口树 / 字典树 / 文档）
const leftPanel = ref<'interface' | 'dictionary' | 'docs'>('interface')

const selectedDocFile = ref<DocFile | null>(null)

function onDocSelect(file: DocFile | null) {
  selectedDocFile.value = file
}

function onSidebarResize(size: number) {
  sidebarWidth.value = size
}
</script>

<style scoped>
.app-layout {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100vh;
  overflow: hidden;
}

.app-body {
  position: relative; /* 折叠把手绝对定位的锚点 */
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* 常驻侧栏容器（v-show 显隐；隐藏时 display:none 不占位） */
.side-pane {
  display: flex;
  flex-shrink: 0;
}

/* 1.0.4：窄竖栏（40px 上层 Tab） */
.activity-bar {
  width: 42px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-xs);
  padding: var(--spacing-sm) 0;
  border-right: 1px solid var(--border-base);
  background: var(--bg-surface);
}
.activity-item {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 16px;
}
.activity-item:hover {
  background: var(--bg-hover);
}
.activity-item.active {
  background: var(--bg-selected);
  color: var(--color-primary);
}

/* 1.0.6：折叠把手（门把手式）—— 竖向拉手，垂直居中骑在左侧栏外缘 */
.fold-handle {
  position: absolute;
  top: 50%;
  /* 骑在边界上（-50% 抵消自身宽度），nudge 由 hover 状态给方向 */
  transform: translate(-50%, -50%) translateX(var(--handle-nudge, 0px));
  width: var(--handle-w);
  height: var(--handle-h);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 3px;
  padding: 0;
  border: 1px solid var(--handle-border);
  border-radius: var(--handle-radius);
  background: var(--handle-bg);
  cursor: pointer;
  z-index: 12; /* 高于 ResizableSplitter(10)，可覆在其上 */
  transition: background-color 0.15s ease, border-color 0.15s ease, transform 0.18s ease;
}
.fold-handle__grip {
  width: var(--handle-grip-w);
  height: var(--handle-grip-h);
  border-radius: 1px;
  background: var(--handle-grip);
  transition: background-color 0.15s ease;
}
.fold-handle:hover {
  background: var(--handle-bg-hover);
  border-color: var(--handle-accent);
  --handle-nudge: 3px; /* 收起态：向右「拉」开 */
}
.fold-handle.is-expanded:hover {
  --handle-nudge: -3px; /* 展开态：向左「推」合 */
}
.fold-handle:hover .fold-handle__grip {
  background: var(--handle-accent);
}
/* 收起态：握纹用强调色，暗示「这里是打开入口」（展开态无需提示） */
.fold-handle:not(.is-expanded) .fold-handle__grip {
  background: var(--handle-accent);
}
.fold-handle:active {
  transform: translate(-50%, -50%) scale(0.96);
}
.fold-handle:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--handle-accent);
}
@media (prefers-reduced-motion: reduce) {
  .fold-handle,
  .fold-handle__grip {
    transition: none;
  }
}
</style>
