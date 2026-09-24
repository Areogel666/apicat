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
import { checkForUpdateAtStartup, skillGuideShownThisVersion, markSkillGuideShown } from '../../stores/startupChecks'

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

onMounted(async () => {
  // 串行，不并行：两个检查都会弹模态，并行会在「首启 + 恰有新版本」时叠成两层弹窗。
  // 技能引导先跑（纯本地检测，瞬时返回），更新检查随后跑；更新检查弹框前会看引导弹窗是否开着，
  // 开着就让位（不写「已提示」标记，下次启动再提示）。
  await checkSkillGuideOnStartup()
  checkForUpdateAtStartup({ dialog, message, isBlocked: () => showSkillGuide.value })
})

/**
 * 1.0.6：首启技能引导 —— 两个条件同时满足才弹窗：
 *   1) 有 agent 目录可用（否则无处可装，弹窗无意义）
 *   2) 技能**完全没装过**（所有目标都没有任何 apicat 技能）——
 *      而非「任一目标没装就提示」（只装了 Claude Code 的人不该被反复提示 Codex）
 * 标记按 App 版本记：同版本不重复提示，App 升级后若仍完全没装会再提示一次。
 * 静默失败：get_skill_targets 异常不打断启动。
 */
async function checkSkillGuideOnStartup() {
  try {
    if (await skillGuideShownThisVersion()) return
    const targets = await invoke<SkillTarget[]>('get_skill_targets')
    const anyAgentAvailable = targets.some(t => t.agent_installed)
    const anySkillInstalled = targets.some(t => t.skills_installed)
    if (!anyAgentAvailable || anySkillInstalled) return
    await markSkillGuideShown() // 先标记再弹，关掉后本版本内不重复打扰
    showSkillGuide.value = true
  } catch (e) {
    console.debug('[startup] 技能引导检测跳过:', e)
  }
}
</script>
