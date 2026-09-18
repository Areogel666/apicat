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
      <!-- 左右面板均常驻(v-show)，切换时保留全部浏览状态 -->
      <div class="side-pane" v-show="leftPanel === 'interface'">
        <Sidebar :style="{ width: sidebarWidth + 'px' }" />
      </div>
      <div class="side-pane" v-show="leftPanel === 'dictionary'">
        <DictionarySidebar :style="{ width: sidebarWidth + 'px' }" />
      </div>
      <div class="side-pane" v-show="leftPanel === 'docs'">
        <DocsSidebar
          ref="docsSidebarRef"
          :style="{ width: sidebarWidth + 'px' }"
          @select="onDocSelect"
        />
      </div>
      <ResizableSplitter
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
import { ref } from 'vue'
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
</style>
