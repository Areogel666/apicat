<template>
  <n-modal v-model:show="show" preset="card" title="Cookie 管理" style="width: 860px; max-width: 95vw">
    <!-- 工具条：作用域筛选 + 清理（Q14-B：一张表 + 筛选列，不再分 tab） -->
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
      @delete="cookieStore.deleteCookie"
      @toggle="toggleCookie"
    />
    <AddCookieForm
      :default-scope="defaultScope"
      :project-id="projectStore.currentProjectId"
      @created="reload"
    />

    <template #footer>
      <n-button @click="show = false">关闭</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { NModal, NButton, NSelect, NPopover, NCheckbox, useMessage } from 'naive-ui'
import { useCookieStore, type CookieScopeFilter } from '../../stores/cookie'
import { useProjectStore } from '../../stores/project'
import { useEnvironmentStore } from '../../stores/environment'
import CookieTable from './CookieTable.vue'
import AddCookieForm from './AddCookieForm.vue'

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

/** 筛选后的可见列表（store 已按 scope 拉取） */
const visibleCookies = computed(() => {
  if (scopeFilter.value !== 'env') return cookieStore.cookies
  return cookieStore.cookies.filter(c => c.env_id === filterEnvId.value)
})

const expiredCount = computed(() => {
  const now = nowStr()
  return cookieStore.cookies.filter(c => c.expires_at && c.expires_at <= now).length
})
const sessionCount = computed(() => cookieStore.cookies.filter(c => !c.expires_at).length)

function nowStr() {
  const n = new Date()
  const p = (x: number) => String(x).padStart(2, '0')
  return `${n.getUTCFullYear()}-${p(n.getUTCMonth() + 1)}-${p(n.getUTCDate())} ${p(n.getUTCHours())}:${p(n.getUTCMinutes())}:${p(n.getUTCSeconds())}`
}

async function reload() {
  await cookieStore.loadCookies({
    scope: scopeFilter.value === 'env' ? 'all' : scopeFilter.value,
    projectId: projectStore.currentProjectId,
    envId: envStore.activeEnvId,
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

watch(scopeFilter, reload)

async function toggleCookie(id: number, enabled: boolean) {
  await cookieStore.updateCookie(id, { enabled: enabled ? 1 : 0 })
}

async function runCleanup() {
  const n = await cookieStore.cleanupCookies(cleanupOpts)
  message.success(`已删除 ${n} 条 Cookie`)
}
</script>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.hint { font-size: 11px; color: var(--text-tertiary); }
.cleanup { display: flex; flex-direction: column; gap: 6px; min-width: 180px; }
</style>
