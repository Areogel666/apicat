<template>
  <header class="top-bar">
    <!-- 左：产品名 + 项目切换 -->
    <div class="top-bar__left">
      <span class="top-bar__logo">🐱 ApiCat</span>
      <n-select
        v-model:value="currentProjectId"
        :options="projectOptions"
        placeholder="选择项目"
        size="small"
        style="width: 160px"
      />
    </div>

    <!-- 右：环境切换 + 全局 Cookie + 设置 -->
    <div class="top-bar__right">
      <n-select
        :value="currentEnvValue"
        :options="envOptions"
        placeholder="无环境"
        size="small"
        style="width: 150px"
        @update:value="handleEnvChange"
      />
      <n-button size="small" quaternary title="Cookie 管理" @click="showCookieManager = true">🍪</n-button>
      <n-button size="small" quaternary title="设置">⚙️</n-button>
    </div>
  </header>

  <!-- 环境管理弹窗 -->
  <EnvManager v-model:show="showEnvManager" />

  <!-- Cookie 管理弹窗 -->
  <CookieManager v-model:show="showCookieManager" />
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NSelect, NButton } from 'naive-ui'
import { useProjectStore } from '../../stores/project'
import { useEnvironmentStore } from '../../stores/environment'
import EnvManager from '../env/EnvManager.vue'
import CookieManager from '../cookie/CookieManager.vue'

const projectStore = useProjectStore()
const envStore = useEnvironmentStore()

const showEnvManager = ref(false)
const showCookieManager = ref(false)

// ── 项目下拉 ──────────────────────────────────────────────
const currentProjectId = computed({
  get: () => projectStore.currentProjectId,
  set: (v) => { projectStore.currentProjectId = v },
})

const projectOptions = computed(() =>
  projectStore.projects.map(p => ({ label: p.name, value: p.id }))
)

// ── 环境下拉 ──────────────────────────────────────────────
// 当前项目切换时，重新加载环境列表
watch(() => projectStore.currentProjectId, async (pid) => {
  if (pid) {
    await envStore.loadEnvironments(pid)
  }
}, { immediate: true })

// 环境选项：「无环境」+ 真实环境列表 + 分隔线 + 「管理环境...」
const ENV_MANAGE_SENTINEL = -1
const ENV_NULL_SENTINEL = 0  // 代表"无环境"

const envOptions = computed(() => [
  { label: '无环境', value: ENV_NULL_SENTINEL },
  ...envStore.environments.map(e => ({
    label: e.name + (e.is_active ? ' ✓' : ''),
    value: e.id,
  })),
  { label: '—— 管理环境...', value: ENV_MANAGE_SENTINEL },
])

// 当前激活的 env value（用于 v-model）
const currentEnvValue = computed(() => {
  const active = envStore.activeEnvId
  return active ?? ENV_NULL_SENTINEL
})

async function handleEnvChange(val: number) {
  if (val === ENV_MANAGE_SENTINEL) {
    showEnvManager.value = true
    return
  }
  const pid = projectStore.currentProjectId
  if (!pid) return
  if (val === ENV_NULL_SENTINEL) {
    await envStore.deactivateEnvironment(pid)
  } else {
    await envStore.activateEnvironment(pid, val)
  }
}
</script>

<style scoped>
.top-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 48px;
  padding: 0 16px;
  border-bottom: 1px solid var(--n-border-color, #e0e0e6);
  background: var(--n-color, #fff);
  flex-shrink: 0;
  gap: 12px;
  -webkit-app-region: drag;
}

.top-bar__left,
.top-bar__right {
  -webkit-app-region: no-drag;
  display: flex;
  align-items: center;
  gap: 8px;
}

.top-bar__logo {
  font-weight: 700;
  font-size: 15px;
  letter-spacing: -0.3px;
  white-space: nowrap;
  user-select: none;
}
</style>
