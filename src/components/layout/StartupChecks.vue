<template>
  <!-- 无渲染逻辑组件：承载启动期一次性检查 + 首启引导弹窗。
       必须挂在 <n-message-provider>/<n-dialog-provider> 子树内部，
       因为 useDialog/useMessage 依赖 provide/inject（在 App.vue 本身调用会抛
       "No outer <n-dialog-provider /> founded"）。 -->
  <SkillManager v-model:show="showSkillGuide" />
</template>

<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useDialog, useMessage } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import SkillManager from '../settings/SkillManager.vue'
import { useGuideStore } from '../../stores/guide'
import { useTourStore } from '../../stores/tour'
import { useProjectStore } from '../../stores/project'
import { checkForUpdateAtStartup, skillGuideShownThisVersion, markSkillGuideShown } from '../../stores/startupChecks'

/** 只用到两个标志位（首启引导的判据）；完整形状见 SkillManager.vue */
interface SkillTarget {
  id: string
  name: string
  path: string
  agent_installed: boolean
  skills_installed: boolean
}

const dialog = useDialog()
const message = useMessage()

// 1.0.6：首启技能引导弹窗
const showSkillGuide = ref(false)

onMounted(async () => {
  // 串行，不并行：三个启动期弹层（tour / 技能引导 / 更新检查）依次进行，避免叠层。
  // 1.0.9：首启 tour 最先跑（driver.js 聚光灯），无论完成还是中途关闭都记 seen，
  // 且 await 其结束——技能引导与更新检查保证在 tour 之后照常弹出，不被吞掉。
  await runTourOnStartup()
  await checkSkillGuideOnStartup()
  checkForUpdateAtStartup({ dialog, message, isBlocked: () => showSkillGuide.value })
})

/**
 * 1.0.9：首启功能导览 —— guide.tour 未 seen 才跑（应用生涯一次性，不随版本重弹；
 * ⚙️「重看新手引导」的入口在 TopBar，不经这里）。tour 实例在 AppLayout（每步要切面板），
 * 这里经 tour store 的信号触发并等待结束。
 */
async function runTourOnStartup() {
  try {
    const guideStore = useGuideStore()
    // App.vue 对 guide init 是 void 不等待，这里判定前必须确保 seen 标记已载入
    await guideStore.init()
    if (guideStore.isSeen('tour')) return
    await waitSeedSettled()
    await new Promise<void>(resolve => useTourStore().requestStart(() => resolve()))
  } catch (e) {
    console.debug('[startup] 功能导览跳过:', e)
  }
}

/** 等示例数据播种完成（空库首启有 sidebarReloadTick 信号）或 800ms 兜底，让 tour 第一步的接口树有内容可看 */
function waitSeedSettled() {
  return new Promise<void>(resolve => {
    const projectStore = useProjectStore()
    let done = false
    const finish = () => {
      if (done) return
      done = true
      clearTimeout(timer)
      stop()
      resolve()
    }
    const timer = setTimeout(finish, 800)
    const stop = watch(() => projectStore.sidebarReloadTick, (tick) => {
      if (tick > 0) finish()
    }, { immediate: true })
  })
}

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
