<template>
  <n-config-provider :theme="themeStore.naiveTheme" :theme-overrides="themeStore.naiveOverrides" :locale="zhCN" :date-locale="dateZhCN">
    <n-dialog-provider>
      <n-message-provider>
        <AppLayout />
      </n-message-provider>
    </n-dialog-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { NConfigProvider, NMessageProvider, NDialogProvider, zhCN, dateZhCN } from 'naive-ui'
import AppLayout from './components/layout/AppLayout.vue'
import { useProjectStore } from './stores/project'
import { useThemeStore } from './stores/theme'

const projectStore = useProjectStore()
const themeStore = useThemeStore()

// 应用启动：
// 1. 主题先初始化（避免首屏闪白；读偏好 → 写 <html data-theme>）
// 2. 再加载项目列表
// 3. 最后恢复上次打开的项目（M3-A，必须在 loadProjects 后才能校验目标 id）
//
// Sidebar.vue 已有 loadSeq 防竞态机制，currentProjectId 变更触发的侧边栏加载会被自动管理。
onMounted(async () => {
  await themeStore.init()
  await projectStore.loadProjects()
  await projectStore.restoreLastProject()
})
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
