<template>
  <n-config-provider :theme="theme" :locale="zhCN" :date-locale="dateZhCN">
    <n-dialog-provider>
      <n-message-provider>
        <AppLayout />
      </n-message-provider>
    </n-dialog-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { NConfigProvider, NMessageProvider, NDialogProvider, zhCN, dateZhCN, darkTheme } from 'naive-ui'
import AppLayout from './components/layout/AppLayout.vue'
import { useUiStore } from './stores/ui'
import { useProjectStore } from './stores/project'

const uiStore = useUiStore()
const projectStore = useProjectStore()

// 跟随系统主题（后续可在设置中手动切换）
const theme = computed(() => uiStore.darkMode ? darkTheme : null)

// 应用启动时加载项目列表
onMounted(() => projectStore.loadProjects())
</script>

<style>
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html, body, #app {
  width: 100%;
  height: 100%;
  overflow: hidden;
}
</style>
