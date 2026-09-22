<template>
  <n-config-provider :theme="themeStore.naiveTheme" :theme-overrides="themeStore.naiveOverrides" :locale="zhCN" :date-locale="dateZhCN">
    <n-dialog-provider>
      <n-message-provider>
        <AppLayout />
        <!-- 1.0.6：首启技能引导 —— 检测到 agent 目录但未装技能时自动弹出 -->
        <SkillManager v-model:show="showSkillGuide" />
      </n-message-provider>
    </n-dialog-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { NConfigProvider, NMessageProvider, NDialogProvider, zhCN, dateZhCN, useDialog, useMessage } from 'naive-ui'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import AppLayout from './components/layout/AppLayout.vue'
import SkillManager from './components/settings/SkillManager.vue'
import { useProjectStore } from './stores/project'
import { useThemeStore } from './stores/theme'
import { useCollectionStore } from './stores/collection'
import { useRequestStore } from './stores/request'
import { useDictionaryStore } from './stores/dictionary'
import { useTestCaseStore } from './stores/testCase'
import { checkForUpdateAtStartup, skillGuideShown, markSkillGuideShown } from './stores/startupChecks'

interface SkillTarget {
  id: string
  name: string
  path: string
  agent_installed: boolean
  skills_installed: boolean
  installed_skills: string[]
}

const projectStore = useProjectStore()
const themeStore = useThemeStore()
const collectionStore = useCollectionStore()
const requestStore = useRequestStore()
const dictionaryStore = useDictionaryStore()
const testCaseStore = useTestCaseStore()

// 1.0.6：启动检查更新 / 首启引导需要 dialog + message 实例
const dialog = useDialog()
const message = useMessage()

// 1.0.6：首启技能引导弹窗
const showSkillGuide = ref(false)

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
      await Promise.all(
        collectionStore.getCollections(pid).map(c => requestStore.loadRequests(c.id))
      )
    } else if (kind === 'dictionaries' || kind === 'field_bindings') {
      await Promise.all([
        dictionaryStore.loadDictionaries(pid),
        dictionaryStore.loadFieldBindings(pid),
      ])
    } else if (kind === 'test_cases') {
      // 技能通过 bridge 写用例后，刷新当前激活接口的用例列表
      const rid = requestStore.activeRequestId
      if (rid != null) {
        await testCaseStore.loadTestCases(rid)
      }
    }
    // stress 由压测面板自行处理
  })

  // 1.0.6：启动期一次性检查（并行跑，互不阻塞）
  checkForUpdateAtStartup({ dialog, message })
  checkSkillGuideOnStartup()
})

/**
 * 1.0.6：首启技能引导 —— 检测到存在 agent 目录但技能未装（且未提示过）→ 弹窗引导安装。
 * 静默失败：get_skill_targets 异常不打断启动。
 */
async function checkSkillGuideOnStartup() {
  try {
    if (await skillGuideShown()) return
    const targets = await invoke<SkillTarget[]>('get_skill_targets')
    const needsGuide = targets.some(t => t.agent_installed && !t.skills_installed)
    if (!needsGuide) return
    await markSkillGuideShown() // 先标记再弹，关掉后不重复打扰
    showSkillGuide.value = true
  } catch (e) {
    console.debug('[startup] 技能引导检测跳过:', e)
  }
}

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
