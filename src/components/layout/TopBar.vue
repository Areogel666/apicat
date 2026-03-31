<template>
  <header class="top-bar">
    <!-- 左：产品名 + 项目切换 -->
    <div class="top-bar__left">
      <span class="top-bar__logo">🐱 ApiCat</span>
      <n-select
        :value="currentProjectId"
        :options="projectOptions"
        placeholder="选择项目"
        size="small"
        style="width: 160px"
        @update:value="handleProjectChange"
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
      <n-dropdown
        :options="settingsMenuOptions"
        placement="bottom-end"
        @select="handleSettingsMenu"
      >
        <n-button size="small" quaternary title="更多操作">⚙️</n-button>
      </n-dropdown>
    </div>
  </header>

  <!-- 环境管理弹窗 -->
  <EnvManager v-model:show="showEnvManager" />

  <!-- Cookie 管理弹窗 -->
  <CookieManager v-model:show="showCookieManager" />

  <!-- 导入弹窗 -->
  <ImportDialog v-model:show="showImportDialog" />

  <!-- 导出弹窗 -->
  <ExportDialog v-model:show="showExportDialog" />

  <!-- 公共 Headers 模板弹窗 -->
  <HeaderTemplateModal v-model:show="showHeaderTemplateModal" />

  <!-- 重命名项目弹窗 -->
  <n-modal v-model:show="showRenameModal" preset="dialog" title="重命名项目">
    <n-input v-model:value="renameInput" placeholder="输入新的项目名称" @keyup.enter="confirmRenameProject" />
    <template #action>
      <n-button @click="showRenameModal = false">取消</n-button>
      <n-button type="primary" @click="confirmRenameProject">确定</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NSelect, NButton, NDropdown, NModal, NInput, useDialog } from 'naive-ui'
import { useProjectStore } from '../../stores/project'
import { useEnvironmentStore } from '../../stores/environment'
import EnvManager from '../env/EnvManager.vue'
import CookieManager from '../cookie/CookieManager.vue'
import ImportDialog from '../io/ImportDialog.vue'
import ExportDialog from '../io/ExportDialog.vue'
import HeaderTemplateModal from './HeaderTemplateModal.vue'

const projectStore = useProjectStore()
const envStore = useEnvironmentStore()
const dialog = useDialog()

const showEnvManager = ref(false)
const showCookieManager = ref(false)
const showImportDialog = ref(false)
const showExportDialog = ref(false)
const showHeaderTemplateModal = ref(false)

const showRenameModal = ref(false)
const renameInput = ref('')

const settingsMenuOptions = computed(() => [
  { label: '📥 导入接口...', key: 'import' },
  { label: '📤 导出接口...', key: 'export' },
  { type: 'divider', key: 'd1' },
  { label: '📋 公共 Headers 模板...', key: 'headerTemplate' },
])

async function handleSettingsMenu(key: string) {
  if (key === 'import') showImportDialog.value = true
  else if (key === 'export') showExportDialog.value = true
  else if (key === 'headerTemplate') showHeaderTemplateModal.value = true
}

async function handleRenameProject() {
  const pid = projectStore.currentProjectId
  if (!pid) return
  const current = projectStore.projects.find(p => p.id === pid)
  renameInput.value = current?.name ?? ''
  showRenameModal.value = true
}

async function confirmRenameProject() {
  const pid = projectStore.currentProjectId
  if (!pid) return
  const current = projectStore.projects.find(p => p.id === pid)
  const name = renameInput.value.trim()
  if (!name) return
  await projectStore.updateProject(pid, name, current?.description ?? undefined)
  showRenameModal.value = false
}

async function handleDeleteProject() {
  const pid = projectStore.currentProjectId
  if (!pid) return
  const current = projectStore.projects.find(p => p.id === pid)
  dialog.warning({
    title: '确认删除项目',
    content: `确定删除项目「${current?.name}」及其所有数据吗？此操作不可撤销！`,
    positiveText: '确认删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await projectStore.deleteProject(pid)
    }
  })
}

// ── 项目下拉 ──────────────────────────────────────────────
const currentProjectId = computed({
  get: () => projectStore.currentProjectId,
  set: (v) => { projectStore.currentProjectId = v },
})

const CREATE_PROJ_SENTINEL = -1
const RENAME_PROJ_SENTINEL = -2
const DELETE_PROJ_SENTINEL = -3

const projectOptions = computed(() => {
  const opts: Array<{label: string; value: number; disabled?: boolean}> = projectStore.projects.map(p => ({ label: p.name, value: p.id }))
  opts.push({ label: '✏️ 重命名当前项目...', value: RENAME_PROJ_SENTINEL, disabled: !projectStore.currentProjectId })
  opts.push({ label: '🗑️ 删除当前项目...', value: DELETE_PROJ_SENTINEL, disabled: !projectStore.currentProjectId })
  opts.push({ label: '➕ 新建项目...', value: CREATE_PROJ_SENTINEL })
  return opts
})

async function handleProjectChange(val: number) {
  if (val === CREATE_PROJ_SENTINEL) {
    const name = prompt('请输入新项目名称：', 'New Project')
    if (name && name.trim()) {
      const proj = await projectStore.createProject(name.trim())
      projectStore.currentProjectId = proj.id
    }
  } else if (val === RENAME_PROJ_SENTINEL) {
    await handleRenameProject()
  } else if (val === DELETE_PROJ_SENTINEL) {
    await handleDeleteProject()
  } else {
    projectStore.currentProjectId = val
  }
}

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
  { label: '管理环境...', value: ENV_MANAGE_SENTINEL },
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
