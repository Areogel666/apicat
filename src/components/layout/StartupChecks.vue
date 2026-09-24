<template>
  <!-- 无渲染逻辑组件：承载启动期一次性检查 + 首启引导弹窗。
       必须挂在 <n-message-provider>/<n-dialog-provider> 子树内部，
       因为 useDialog/useMessage 依赖 provide/inject（在 App.vue 本身调用会抛
       "No outer <n-dialog-provider /> founded"）。 -->
  <SkillManager v-model:show="showSkillGuide" />
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useDialog, useMessage } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import SkillManager from '../settings/SkillManager.vue'
import { checkForUpdateAtStartup, skillGuideShown, markSkillGuideShown } from '../../stores/startupChecks'

interface SkillTarget {
  id: string
  name: string
  path: string
  agent_installed: boolean
  skills_installed: boolean
  installed_skills: string[]
}

const dialog = useDialog()
const message = useMessage()

// 1.0.6：首启技能引导弹窗
const showSkillGuide = ref(false)

onMounted(() => {
  // 启动期一次性检查（并行跑，互不阻塞）
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
</script>
