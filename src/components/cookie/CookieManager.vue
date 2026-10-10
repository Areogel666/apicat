<template>
  <n-modal v-model:show="show" preset="card" title="Cookie 管理" style="width: 860px; max-width: 95vw">
    <!-- 工具条：作用域筛选 + 清理（Q14-B：一张表 + 筛选列，不再分 tab） -->
    <GuideBar guide-id="cookie" />
    <div class="toolbar">
      <n-select
        v-model:value="scopeFilter"
        size="small"
        :options="scopeFilterOptions"
        style="width:110px"
      />
      <n-select
        v-if="scopeFilter === 'env'"
        v-model:value="filterEnvId"
        size="small"
        :options="envOptions"
        placeholder="选环境"
        style="width:150px"
      />
      <span class="hint">{{ projectHint }}</span>
      <span style="flex:1" />
      <n-button size="small" type="primary" @click="startCreate">+ 添加 Cookie</n-button>
      <n-popover trigger="click" placement="bottom-end">
        <template #trigger>
          <n-button size="small" quaternary>清理…</n-button>
        </template>
        <div class="cleanup">
          <n-checkbox v-model:checked="cleanupOpts.expired">已过期（{{ expiredCount }}）</n-checkbox>
          <n-checkbox v-model:checked="cleanupOpts.session">无过期时间（{{ sessionCount }}）</n-checkbox>
          <n-button
            size="small"
            type="error"
            :disabled="!cleanupOpts.expired && !cleanupOpts.session"
            style="width:100%; margin-top:8px"
            @click="runCleanup"
          >删除所选</n-button>
        </div>
      </n-popover>
    </div>

    <CookieTable
      :cookies="visibleCookies"
      :editing-id="editingCookie?.id ?? null"
      @delete="onDelete"
      @edit="startEdit"
      @toggle="toggleCookie"
    />
    <AddCookieForm
      :default-scope="defaultScope"
      :project-id="projectStore.currentProjectId"
      :creating="creating"
      :editing="editingCookie"
      @saved="reload"
      @cancel="cancelEdit"
    />

    <template #footer>
      <n-button @click="show = false">关闭</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import GuideBar from '../guide/GuideBar.vue'
import { computed, reactive, ref, watch } from 'vue'
import { NModal, NButton, NSelect, NPopover, NCheckbox, useMessage } from 'naive-ui'
import { useCookieStore, type CookieScopeFilter } from '../../stores/cookie'
import type { CookieItem } from '../../types'
import { useProjectStore } from '../../stores/project'
import { useEnvironmentStore } from '../../stores/environment'
import CookieTable from './CookieTable.vue'
import AddCookieForm from './AddCookieForm.vue'
import { nowUtcString } from '../../utils/time'

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ 'update:show': [value: boolean] }>()

const show = computed({
  get: () => props.show,
  set: (v) => emit('update:show', v),
})

const cookieStore = useCookieStore()
const projectStore = useProjectStore()
const envStore = useEnvironmentStore()
const message = useMessage()

const scopeFilter = ref<CookieScopeFilter>('all')
const filterEnvId = ref<number | null>(null)
const cleanupOpts = reactive({ expired: true, session: false })

const scopeFilterOptions = [
  { label: '全部', value: 'all' },
  { label: '全局', value: 'global' },
  { label: '项目', value: 'project' },
  { label: '环境', value: 'env' },
]

const envOptions = computed(() =>
  envStore.environments.map(e => ({ label: e.name, value: e.id })),
)

const defaultScope = computed(() =>
  scopeFilter.value === 'all' ? 'global' : scopeFilter.value,
)

const projectHint = computed(() =>
  projectStore.currentProjectId
    ? `项目级 / 环境级 cookie 归属当前项目`
    : `未选中项目，仅能管理全局 Cookie`,
)

/**
 * 筛选后的可见列表。
 *
 * 取数固定拉全量（global + 项目 + 全部环境），筛选纯在前端做——
 * 两个状态源（取数用 activeEnvId、显示用 filterEnvId）曾各走各的，
 * 选到非激活环境时列表恒为空（评审发现）。
 */
const visibleCookies = computed(() => {
  const list = cookieStore.cookies
  if (scopeFilter.value === 'global') return list.filter(c => c.scope_type === 'global')
  if (scopeFilter.value === 'project') return list.filter(c => c.scope_type === 'project')
  if (scopeFilter.value === 'env') {
    return list.filter(c => c.scope_type === 'env'
      && (filterEnvId.value == null || c.env_id === filterEnvId.value))
  }
  return list
})

const expiredCount = computed(() => {
  const now = nowUtcString()
  return cookieStore.cookies.filter(c => c.expires_at && c.expires_at <= now).length
})
const sessionCount = computed(() => cookieStore.cookies.filter(c => !c.expires_at).length)

/** 正在编辑的 cookie；非空时编辑区进入编辑模式 */
const editingCookie = ref<CookieItem | null>(null)
/** 新建模式；为真时编辑区显示空白表单 */
const creating = ref(false)

function startEdit(c: CookieItem) {
  creating.value = false
  editingCookie.value = c
}

function startCreate() {
  editingCookie.value = null
  creating.value = true
}

/** 收起编辑区（编辑与新建两种模式一起收） */
function cancelEdit() {
  editingCookie.value = null
  creating.value = false
}

/** 删除时若正编辑该条，先退出编辑态——否则表单会提交到一个已不存在的 id */
async function onDelete(id: number) {
  if (editingCookie.value?.id === id) editingCookie.value = null
  await cookieStore.deleteCookie(id)
}

async function reload() {
  cancelEdit() // 写入完成后收起编辑区
  // 固定拉全量，筛选交给 visibleCookies（见其注释）
  await cookieStore.loadCookies({
    scope: 'all',
    projectId: projectStore.currentProjectId,
    envId: null,
  })
}

// 打开弹窗时加载（Q14-B 修：旧版把项目加载挂在 n-tab-pane 的 @click 上，点 tab 标题不冒泡 → 永不触发）
watch(() => props.show, async (v) => {
  if (!v) return
  if (projectStore.currentProjectId && envStore.environments.length === 0) {
    try {
      await envStore.loadEnvironments(projectStore.currentProjectId)
    } catch { /* 环境列表拿不到不影响 cookie 管理 */ }
  }
  if (!filterEnvId.value && envStore.activeEnvId) filterEnvId.value = envStore.activeEnvId
  await reload()
})

// 切项目要重新取（项目层 cookie 换了一批）
watch(() => projectStore.currentProjectId, reload)
// 筛选是纯前端的，无需重取；但切环境时若正显示「环境」筛选，列表随 computed 自动变

async function toggleCookie(id: number, enabled: boolean) {
  await cookieStore.updateCookie(id, { enabled: enabled ? 1 : 0 })
}

async function runCleanup() {
  const n = await cookieStore.cleanupCookies(cleanupOpts)
  message.success(`已删除 ${n} 条 Cookie`)
}
</script>

<style scoped>
/* 全部走项目主题 token（见 CLAUDE.md「主题系统」），硬编码 px 不随风格档位变 */
.toolbar {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-md);
}
.hint { font-size: var(--font-size-sm); color: var(--text-tertiary); }
.cleanup {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  min-width: 180px;
}
</style>
