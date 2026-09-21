<template>
  <header class="top-bar">
    <!-- 左：产品名 + 项目切换 -->
    <div class="top-bar__left">
      <span class="top-bar__logo">🐱 ApiCat</span>
      <div class="ctx-select ctx-select--project">
        <span class="ctx-select__prefix">📁</span>
        <n-select
          :value="currentProjectId"
          :options="projectOptions"
          placeholder="选择项目"
          size="small"
          class="ctx-select__control"
          style="width: 160px"
          @update:value="handleProjectChange"
        />
      </div>
    </div>

    <!-- 右：环境切换 + 全局 Cookie + 设置 -->
    <div class="top-bar__right">
      <div class="ctx-select ctx-select--env">
        <span class="ctx-select__prefix">🌐</span>
        <n-select
          :value="currentEnvValue"
          :options="envOptions"
          placeholder="无环境"
          size="small"
          class="ctx-select__control"
          style="width: 150px"
          @update:value="handleEnvChange"
        />
      </div>

      <!-- 操作图标组：安静的小圆底 hover，与切换器用发丝线分隔 -->
      <div class="top-bar__actions">
        <n-button
          size="small"
          quaternary
          circle
          :title="themeStore.effectiveMode === 'dark' ? '切换到浅色模式' : '切换到深色模式'"
          @click="toggleThemeMode"
        >{{ themeStore.effectiveMode === 'dark' ? '🌙' : '☀️' }}</n-button>
        <n-button size="small" quaternary circle title="Cookie 管理" @click="showCookieManager = true">🍪</n-button>
        <n-dropdown
          :options="settingsMenuOptions"
          placement="bottom-end"
          @select="handleSettingsMenu"
        >
          <n-button size="small" quaternary circle title="更多操作">⚙️</n-button>
        </n-dropdown>
      </div>
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

  <!-- AI 技能管理弹窗 -->
  <SkillManager v-model:show="showSkillManager" />

  <!-- 主题工作室弹窗 -->
  <ThemeStudioModal ref="themeStudioModalRef" />

  <!-- 重命名项目弹窗 -->
  <n-modal v-model:show="showRenameModal" preset="dialog" title="项目设置">
    <div style="display: flex; flex-direction: column; gap: 12px">
      <div>
        <div style="font-size: 12px; color: var(--text-secondary); margin-bottom: 4px">项目名称</div>
        <n-input v-model:value="renameInput" placeholder="输入项目名称" @keyup.enter="confirmRenameProject" />
      </div>
      <div>
        <div style="font-size: 12px; color: var(--text-secondary); margin-bottom: 4px">
          文档输出目录（apicat-doc-gen 技能使用，留空则默认 ~/.apicat/apidoc/{项目名}）
        </div>
        <n-input v-model:value="docsDirInput" placeholder="如 D:\Projects\ias-api-doc\ias-home-api" />
      </div>
    </div>
    <template #action>
      <n-button @click="showRenameModal = false">取消</n-button>
      <n-button type="primary" @click="confirmRenameProject">确定</n-button>
    </template>
  </n-modal>

  <!-- 清理历史记录弹窗 -->
  <n-modal v-model:show="showCleanupHistoryModal" preset="dialog" title="清理历史记录">
    <div style="display: flex; flex-direction: column; gap: 16px">
      <div style="font-size: 13px; color: var(--text-secondary)">
        选择清理策略，可组合使用。清理后不可恢复，请谨慎操作。
      </div>

      <!-- 清理范围 -->
      <div>
        <div style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px">清理范围</div>
        <n-radio-group v-model:value="cleanupScope">
          <n-radio value="all">所有项目</n-radio>
          <n-radio value="current">仅当前项目（{{ projectStore.currentProject?.name || '未知' }}）</n-radio>
        </n-radio-group>
      </div>

      <!-- 按天数清理 -->
      <div>
        <n-checkbox v-model:checked="cleanupByDays">
          按天数清理
        </n-checkbox>
        <div v-if="cleanupByDays" style="margin-top: 8px; margin-left: 24px">
          <n-input-number
            v-model:value="cleanupDays"
            :min="1"
            :max="365"
            placeholder="天数"
            style="width: 120px"
          />
          <span style="margin-left: 8px; font-size: 12px; color: var(--text-tertiary)">
            天前的历史记录
          </span>
        </div>
      </div>

      <!-- 按数量清理 -->
      <div>
        <n-checkbox v-model:checked="cleanupByCount">
          按数量清理
        </n-checkbox>
        <div v-if="cleanupByCount" style="margin-top: 8px; margin-left: 24px">
          <span style="font-size: 12px; color: var(--text-tertiary)">每个接口保留最近</span>
          <n-input-number
            v-model:value="cleanupKeepCount"
            :min="1"
            :max="1000"
            placeholder="数量"
            style="width: 100px; margin: 0 8px"
          />
          <span style="font-size: 12px; color: var(--text-tertiary)">条</span>
        </div>
      </div>

      <!-- 同时清理响应文件 -->
      <div>
        <n-checkbox v-model:checked="cleanupFiles">
          同时清理响应文件（大响应保存在 ~/.apicat/responses/）
        </n-checkbox>
      </div>

      <!-- 警告提示 -->
      <n-alert v-if="cleanupByDays || cleanupByCount" type="warning" :bordered="false">
        即将清理{{ cleanupScope === 'current' ? '当前项目' : '所有项目' }}的历史记录，此操作不可恢复
      </n-alert>
    </div>
    <template #action>
      <n-button @click="showCleanupHistoryModal = false">取消</n-button>
      <n-button
        type="error"
        :disabled="!cleanupByDays && !cleanupByCount"
        :loading="cleanupLoading"
        @click="confirmCleanupHistory"
      >
        确认清理
      </n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { NSelect, NButton, NDropdown, NModal, NInput, NCheckbox, NInputNumber, NAlert, NRadioGroup, NRadio, useDialog, useMessage } from 'naive-ui'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '../../stores/project'
import { useEnvironmentStore } from '../../stores/environment'
import { useThemeStore, type ThemeMode } from '../../stores/theme'
import { readSetting, writeSetting } from '../../stores/_persistedSettings'
import EnvManager from '../env/EnvManager.vue'
import CookieManager from '../cookie/CookieManager.vue'
import ImportDialog from '../io/ImportDialog.vue'
import ExportDialog from '../io/ExportDialog.vue'
import HeaderTemplateModal from './HeaderTemplateModal.vue'
import ThemeStudioModal from '../theme/ThemeStudioModal.vue'
import SkillManager from '../settings/SkillManager.vue'

const projectStore = useProjectStore()
const envStore = useEnvironmentStore()
const themeStore = useThemeStore()
const dialog = useDialog()
const message = useMessage()

const showEnvManager = ref(false)
const showCookieManager = ref(false)
const showImportDialog = ref(false)
const showExportDialog = ref(false)
const showHeaderTemplateModal = ref(false)
const showSkillManager = ref(false)

// 1.0.5：Bridge 开关（默认开；Rust 侧启动时读同一 key）
const bridgeEnabled = ref(true)
onMounted(async () => {
  const v = await readSetting<boolean>('bridgeEnabled')
  bridgeEnabled.value = v ?? true
})
const themeStudioModalRef = ref<InstanceType<typeof ThemeStudioModal> | null>(null)

const showRenameModal = ref(false)
const renameInput = ref('')
const showCleanupHistoryModal = ref(false)

// 清理历史记录状态
const cleanupScope = ref<'all' | 'current'>('current')
const cleanupByDays = ref(false)
const cleanupDays = ref(30)
const cleanupByCount = ref(false)
const cleanupKeepCount = ref(50)
const cleanupFiles = ref(true)
const cleanupLoading = ref(false)

// 主题三选一菜单项（M3-B）
// 选中项前缀 ●，未选中前缀 ○，构成单选视觉
const themeChildren = computed(() => {
  const m = themeStore.mode
  const mark = (target: ThemeMode) => m === target ? '● ' : '○ '
  return [
    { label: `${mark('system')}跟随系统`, key: 'theme:system' },
    { label: `${mark('light')}浅色`, key: 'theme:light' },
    { label: `${mark('dark')}深色`, key: 'theme:dark' },
  ]
})

const settingsMenuOptions = computed(() => [
  { label: '📥 导入接口...', key: 'import' },
  { label: '📤 导出接口...', key: 'export' },
  { type: 'divider', key: 'd1' },
  { label: '📋 公共 Headers 模板...', key: 'headerTemplate' },
  { label: '🤖 AI 技能管理...', key: 'skillManager' },
  { type: 'divider', key: 'd2' },
  { label: '🎨 主题', key: 'theme', children: themeChildren.value },
  { label: '🎨 主题工作室…', key: 'themeStudio' },
  { type: 'divider', key: 'd3' },
  { label: `${bridgeEnabled.value ? '🟢' : '⚪'} HTTP Bridge（${bridgeEnabled.value ? '开' : '关'}）`, key: 'bridgeToggle' },
  { label: '🗑️ 清理历史记录...', key: 'cleanupHistory' },
  { label: '🔄 检查更新...', key: 'checkUpdate' },
])

// 顶部栏二态切换：浅色 ↔ 深色（不经过跟随系统）
async function toggleThemeMode() {
  const target: ThemeMode = themeStore.effectiveMode === 'dark' ? 'light' : 'dark'
  await themeStore.setMode(target)
}

async function handleSettingsMenu(key: string) {
  if (key === 'import') showImportDialog.value = true
  else if (key === 'export') showExportDialog.value = true
  else if (key === 'headerTemplate') showHeaderTemplateModal.value = true
  else if (key === 'skillManager') showSkillManager.value = true
  else if (key === 'themeStudio') themeStudioModalRef.value?.open()
  else if (key === 'checkUpdate') await checkForUpdate()
  else if (key === 'cleanupHistory') showCleanupHistoryModal.value = true
  else if (key === 'bridgeToggle') {
    bridgeEnabled.value = !bridgeEnabled.value
    await writeSetting('bridgeEnabled', bridgeEnabled.value)
    message.info(
      bridgeEnabled.value
        ? 'Bridge 已开启（重启应用后生效）'
        : 'Bridge 已关闭（重启应用后生效）',
    )
  }
  else if (key.startsWith('theme:')) {
    const mode = key.slice('theme:'.length) as ThemeMode
    await themeStore.setMode(mode)
  }
}

// 清理历史记录
async function confirmCleanupHistory() {
  if (!cleanupByDays.value && !cleanupByCount.value) {
    message.warning('请至少选择一种清理策略')
    return
  }

  const scopeText = cleanupScope.value === 'current' ? '当前项目' : '所有项目'
  dialog.warning({
    title: '确认清理',
    content: `将清理${scopeText}的历史记录${cleanupFiles.value ? '及响应文件' : ''}，此操作不可恢复。确定继续？`,
    positiveText: '确认清理',
    negativeText: '取消',
    onPositiveClick: async () => {
      cleanupLoading.value = true
      try {
        const result = await invoke<{ deleted_records: number; deleted_files: number }>('cleanup_history', {
          params: {
            days: cleanupByDays.value ? cleanupDays.value : null,
            keep_per_request: cleanupByCount.value ? cleanupKeepCount.value : null,
            project_id: cleanupScope.value === 'current' ? projectStore.currentProjectId : null,
            cleanup_files: cleanupFiles.value,
          },
        })
        message.success(`清理完成：删除 ${result.deleted_records} 条记录，${result.deleted_files} 个文件`)
        showCleanupHistoryModal.value = false
      } catch (e) {
        message.error(`清理失败：${e}`)
      } finally {
        cleanupLoading.value = false
      }
    },
  })
}

// ── 自动更新 ──────────────────────────────────────────────
async function checkForUpdate() {
  try {
    message.loading('正在检查更新...', { duration: 0 })
    const update = await check()
    message.destroyAll()

    if (!update) {
      message.success('当前已是最新版本 🎉')
      return
    }

    dialog.info({
      title: `发现新版本 ${update.version}`,
      content: update.body
        ? `更新内容：\n${update.body}`
        : '有新版本可用，是否立即更新并重启？',
      positiveText: '立即更新',
      negativeText: '稍后再说',
      onPositiveClick: async () => {
        const downloadMsg = message.loading('正在下载更新，请稍候...', { duration: 0 })
        try {
          await update.downloadAndInstall()
          downloadMsg.destroy()
          message.success('更新完成，正在重启...')
          setTimeout(() => relaunch(), 1500)
        } catch (e) {
          downloadMsg.destroy()
          message.error(`更新失败：${e}`)
        }
      }
    })
  } catch (e) {
    message.destroyAll()
    const msg = String(e)
    // 注意：check() 返回 null 才是「已是最新」（见上方 if (!update) 分支）。
    // 走到 catch 说明更新服务器不可达，常见原因是 Release 仍为草稿
    // —— /releases/latest 不返回草稿，latest.json 取不到。
    if (msg.includes('Could not fetch a valid release JSON')) {
      message.warning('无法连接更新服务器，请稍后重试')
    } else {
      message.error(`检查更新失败：${msg}`)
    }
  }
}

const docsDirInput = ref('')

async function handleRenameProject() {
  const pid = projectStore.currentProjectId
  if (!pid) return
  const current = projectStore.projects.find(p => p.id === pid)
  renameInput.value = current?.name ?? ''
  docsDirInput.value = current?.docs_output_dir ?? ''
  showRenameModal.value = true
}

async function confirmRenameProject() {
  const pid = projectStore.currentProjectId
  if (!pid) return
  const current = projectStore.projects.find(p => p.id === pid)
  const name = renameInput.value.trim()
  if (!name) return
  const dir = docsDirInput.value.trim()
  await projectStore.updateProject(pid, name, current?.description ?? undefined, dir || null)
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
  const opts: Array<Record<string, unknown>> = [
    {
      type: 'group',
      label: '项目',
      key: 'proj-switch',
      children: projectStore.projects.map(p => ({ label: p.name, value: p.id })),
    },
    {
      type: 'group',
      label: '项目操作',
      key: 'proj-actions',
      children: [
        { label: '⚙️ 项目设置...', value: RENAME_PROJ_SENTINEL, disabled: !projectStore.currentProjectId },
        { label: '🗑️ 删除当前项目...', value: DELETE_PROJ_SENTINEL, disabled: !projectStore.currentProjectId },
        { label: '➕ 新建项目...', value: CREATE_PROJ_SENTINEL },
      ],
    },
  ]
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
  {
    type: 'group',
    label: '环境',
    key: 'env-switch',
    children: [
      { label: '无环境', value: ENV_NULL_SENTINEL },
      ...envStore.environments.map(e => ({
        label: e.name + (e.is_active ? ' ✓' : ''),
        value: e.id,
      })),
    ],
  },
  {
    type: 'group',
    label: '环境操作',
    key: 'env-actions',
    children: [
      { label: '管理环境...', value: ENV_MANAGE_SENTINEL },
    ],
  },
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
  height: calc(var(--row-height) + 16px);
  padding: 0 var(--spacing-md);
  border-bottom: 1px solid var(--border-base);
  background: var(--bg-elevated);
  flex-shrink: 0;
  gap: var(--spacing-md);
  -webkit-app-region: drag;
}

.top-bar__left,
.top-bar__right {
  -webkit-app-region: no-drag;
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.top-bar__logo {
  font-weight: 700;
  font-size: var(--font-size-lg);
  letter-spacing: -0.3px;
  white-space: nowrap;
  user-select: none;
  margin-right: var(--spacing-sm);
}

/* ── 上下文切换器（项目 / 环境）：表单感：有 icon、浅底、发丝边框 ── */
.ctx-select {
  position: relative;
  display: inline-flex;
}

.ctx-select__prefix {
  position: absolute;
  left: var(--spacing-sm);
  top: 50%;
  transform: translateY(-50%);
  font-size: var(--font-size-base);
  line-height: 1;
  pointer-events: none;
  z-index: 2;
}

/* 触发表单感：浅底 + 整圈发丝边框，与右侧纯图标按钮区分 */
.ctx-select :deep(.n-base-selection) {
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  padding-left: calc(var(--font-size-base) + var(--spacing-md) + 2px); /* 给 prefix icon 让位 */
  transition: border-color 0.15s ease, background 0.15s ease;
}

.ctx-select :deep(.n-base-selection:hover) {
  background: var(--bg-hover);
  border-color: var(--border-strong);
}

/* 聚焦时用品牌主色描边，强调"当前所在上下文" */
.ctx-select :deep(.n-base-selection--focus) {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-soft);
}

/* ── 操作图标组：安静 + 发丝线分隔 ── */
.top-bar__actions {
  -webkit-app-region: no-drag;
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  margin-left: var(--spacing-sm);
  padding-left: var(--spacing-sm);
  border-left: 1px solid var(--border-base);
}
</style>
