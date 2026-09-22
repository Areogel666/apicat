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
        <!-- 1.0.5：折叠/展开左侧栏（三栏统一） -->
        <button
          class="activity-item activity-toggle"
          :title="collapsed ? '展开左侧栏' : '收起左侧栏'"
          @click="collapsed = !collapsed"
        >{{ collapsed ? '▶' : '◀' }}</button>
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
      <!-- 1.0.6：折叠态折叠把手 —— hover 展开预览（移出收回），点击真正展开 -->
      <div
        class="fold-handle"
        v-show="collapsed && !hoverOpen"
        title="悬停预览左侧栏，点击展开"
        @mouseenter="openHoverPreview"
        @click="expandSidebar"
      ></div>
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
import { ref, watch } from 'vue'
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

// 1.0.6：折叠把手 —— hover 时临时展开预览（不改 collapsed、不持久化），移出收回；点击真正展开
const hoverOpen = ref(false)
function openHoverPreview() {
  if (collapsed.value) hoverOpen.value = true
}
// 移出 side-pane：仅收回预览，不改变 collapsed 持久化状态
function closeHoverPreview() {
  hoverOpen.value = false
}
// 点击把手：真正展开并持久化（watch 已处理 localStorage）
function expandSidebar() {
  hoverOpen.value = false
  collapsed.value = false
}

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
.activity-toggle {
  margin-top: auto; /* 折叠按钮沉底，与上方 Tab 隔开 */
  font-size: 13px;
}

/* 1.0.6：折叠态把手 —— 窄竖条，hover 高亮；命中区加宽到 10px 便于点击 */
.fold-handle {
  position: relative;
  width: 10px;
  flex-shrink: 0;
  cursor: pointer;
  z-index: 5;
}
.fold-handle::before {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  left: 3px;
  width: 3px;
  background: var(--border-base);
  transition: background-color 0.15s;
}
.fold-handle:hover::before,
.fold-handle:active::before {
  background: var(--color-primary);
}
</style>
