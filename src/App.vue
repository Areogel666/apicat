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
import { onMounted, onUnmounted } from 'vue'
import { NConfigProvider, NMessageProvider, NDialogProvider, zhCN, dateZhCN } from 'naive-ui'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import AppLayout from './components/layout/AppLayout.vue'
import { useProjectStore } from './stores/project'
import { useThemeStore } from './stores/theme'
import { useCollectionStore } from './stores/collection'
import { useRequestStore } from './stores/request'
import { useDictionaryStore } from './stores/dictionary'

const projectStore = useProjectStore()
const themeStore = useThemeStore()
const collectionStore = useCollectionStore()
const requestStore = useRequestStore()
const dictionaryStore = useDictionaryStore()

let bridgeUnlisten: UnlistenFn | null = null

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

  // 1.0.5：监听 HTTP Bridge 写操作广播，按需 reload 对应 store
  bridgeUnlisten = await listen<{ kind: string }>('bridge-data-changed', async (event) => {
    const pid = projectStore.currentProjectId
    if (pid == null) return
    const kind = event.payload?.kind
    // 粗粒度：projects/collections/requests 任一变更都刷侧边栏树
    if (kind === 'projects') {
      await projectStore.loadProjects()
    } else if (kind === 'collections' || kind === 'requests') {
      await collectionStore.loadCollections(pid)
      for (const c of collectionStore.getCollections(pid)) {
        await requestStore.loadRequests(c.id)
      }
    } else if (kind === 'dictionaries' || kind === 'field_bindings') {
      await dictionaryStore.loadDictionaries(pid)
      await dictionaryStore.loadFieldBindings(pid)
    }
    // test_cases / stress 由各面板自行监听或用户手动刷新
  })
})

onUnmounted(() => {
  bridgeUnlisten?.()
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
