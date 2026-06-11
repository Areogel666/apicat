<template>
  <!-- 整体垂直布局：TopBar(固定) + 内容区(充满剩余) -->
  <div class="app-layout">
    <TopBar />
    <div class="app-body">
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
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import TopBar from './TopBar.vue'
import Sidebar from './Sidebar.vue'
import MainPanel from './MainPanel.vue'
import ResizableSplitter from '../common/ResizableSplitter.vue'

const sidebarWidth = ref(Number(localStorage.getItem('layout.sidebarWidth') ?? 240))

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
</style>
