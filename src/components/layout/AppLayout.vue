<template>
  <!-- 整体垂直布局：TopBar(固定) + 内容区(充满剩余) -->
  <div class="app-layout">
    <TopBar />
    <div class="app-body">
      <!-- 1.0.4：最左侧窄竖栏 —— 上层 Tab 切换「接口树 / 字典树」 -->
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
      </div>
      <!-- 1.0.4 fix：接口模式 → 左侧接口树 + 右侧主编辑区；字典模式 → 左侧字典树 + 右侧 JSON 编辑面板 -->
      <template v-if="leftPanel === 'interface'">
        <Sidebar :style="{ width: sidebarWidth + 'px', flexShrink: 0 }" />
        <ResizableSplitter
          direction="horizontal"
          :default-size="sidebarWidth"
          :min-size="160"
          :max-size="500"
          storage-key="layout.sidebarWidth"
          @resize="onSidebarResize"
        />
        <MainPanel style="flex: 1; min-width: 0" />
      </template>
      <template v-else>
        <DictionarySidebar :style="{ width: sidebarWidth + 'px', flexShrink: 0 }" />
        <ResizableSplitter
          direction="horizontal"
          :default-size="sidebarWidth"
          :min-size="160"
          :max-size="500"
          storage-key="layout.sidebarWidth"
          @resize="onSidebarResize"
        />
        <DictionaryJsonPanel style="flex: 1; min-width: 0" />
      </template>
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
import ResizableSplitter from '../common/ResizableSplitter.vue'

const sidebarWidth = ref(Number(localStorage.getItem('layout.sidebarWidth') ?? 240))

// 1.0.4：左侧内容面板切换（接口树 / 字典树）
const leftPanel = ref<'interface' | 'dictionary'>('interface')

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
