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
        v-model:value="currentEnvId"
        :options="envOptions"
        placeholder="选择环境"
        size="small"
        style="width: 140px"
      />
      <n-button size="small" quaternary title="全局 Cookie">🍪</n-button>
      <n-button size="small" quaternary title="设置">⚙️</n-button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NSelect, NButton } from 'naive-ui'
import { useProjectStore } from '../../stores/project'

const projectStore = useProjectStore()

const currentProjectId = computed({
  get: () => projectStore.currentProjectId,
  set: (v) => { projectStore.currentProjectId = v }
})

const projectOptions = computed(() =>
  projectStore.projects.map(p => ({ label: p.name, value: p.id }))
)

// 环境下拉（M4 实现，M2 保留占位）
const currentEnvId = ref<number | null>(null)
const envOptions = ref([{ label: '开发环境', value: 1 }])
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
  /* Tauri 窗口拖拽区域 */
  -webkit-app-region: drag;
}

/* 按钮和 select 不触发拖拽 */
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
