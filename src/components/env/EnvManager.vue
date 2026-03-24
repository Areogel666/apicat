<template>
  <n-modal v-model:show="show" preset="card" title="环境管理" style="width: 780px; max-width: 95vw">
    <div class="env-manager">
      <!-- 左侧：环境列表 -->
      <div class="env-list">
        <div
          v-for="env in envStore.environments"
          :key="env.id"
          class="env-item"
          :class="{ 'is-active-env': selectedEnvId === env.id }"
          @click="selectEnv(env.id)"
        >
          <span class="env-name">{{ env.name }}</span>
          <n-tag v-if="env.is_active" size="tiny" type="success" style="flex-shrink:0">激活</n-tag>
          <n-button
            size="tiny"
            quaternary
            style="flex-shrink:0; color:#d03050"
            @click.stop="handleDeleteEnv(env.id)"
          >✕</n-button>
        </div>

        <n-button
          size="small"
          dashed
          style="width: 100%; margin-top: 8px"
          @click="showNewEnvInput = true"
        >
          + 新建环境
        </n-button>

        <!-- 新建环境输入行 -->
        <div v-if="showNewEnvInput" style="margin-top: 8px; display:flex; gap:4px">
          <n-input
            ref="newEnvInputRef"
            v-model:value="newEnvName"
            size="small"
            placeholder="环境名称"
            style="flex:1"
            @keyup.enter="handleCreateEnv"
            @keyup.escape="showNewEnvInput = false"
          />
          <n-button size="small" type="primary" @click="handleCreateEnv">确定</n-button>
        </div>
      </div>

      <!-- 右侧：选中环境详情 -->
      <div v-if="selectedEnv" class="env-detail">
        <!-- 环境名 + base_url -->
        <div class="detail-section">
          <div class="section-label">环境名称</div>
          <n-input v-model:value="editName" size="small" style="margin-bottom:8px" />
          <div class="section-label">Base URL</div>
          <n-input
            v-model:value="editBaseUrl"
            size="small"
            placeholder="如 https://api.example.com"
            style="margin-bottom:8px"
          />
          <n-button size="small" type="primary" @click="handleSaveEnv">保存</n-button>
        </div>

        <n-divider style="margin: 12px 0" />

        <!-- 变量 KV 编辑器 -->
        <div class="detail-section">
          <div class="section-label" style="margin-bottom:8px">变量</div>

          <n-empty v-if="!envStore.variables.length" description="暂无变量" size="small" />

          <div
            v-for="v in envStore.variables"
            :key="v.id"
            class="var-row"
          >
            <n-checkbox
              :checked="v.enabled === 1"
              @update:checked="(val) => toggleVariable(v, val)"
            />
            <n-input
              :value="v.key"
              size="small"
              style="width: 130px; flex-shrink:0"
              placeholder="变量名"
              @blur="(e: FocusEvent) => updateVariableKey(v, (e.target as HTMLInputElement).value)"
            />
            <n-input
              :value="v.value"
              size="small"
              style="flex:1"
              placeholder="值"
              @blur="(e: FocusEvent) => updateVariableValue(v, (e.target as HTMLInputElement).value)"
            />
            <n-button size="tiny" quaternary style="color:#d03050" @click="envStore.deleteVariable(v.id)">✕</n-button>
          </div>

          <n-button size="small" dashed style="width:100%; margin-top:4px" @click="handleAddVariable">
            + 添加变量
          </n-button>
        </div>
      </div>

      <div v-else class="env-detail env-detail-empty">
        <n-empty description="选择或新建一个环境" />
      </div>
    </div>

    <template #footer>
      <div style="display:flex; justify-content:flex-end; gap:8px">
        <n-button v-if="selectedEnv" @click="handleActivate">
          {{ selectedEnv.is_active ? '取消激活' : '激活此环境' }}
        </n-button>
        <n-button @click="show = false">关闭</n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NModal, NButton, NInput, NCheckbox, NDivider, NEmpty, NTag } from 'naive-ui'
import { useEnvironmentStore } from '../../stores/environment'
import { useProjectStore } from '../../stores/project'
import type { EnvVariable } from '../../types'

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ 'update:show': [value: boolean] }>()

const show = computed({
  get: () => props.show,
  set: (v) => emit('update:show', v),
})

const envStore = useEnvironmentStore()
const projectStore = useProjectStore()

const selectedEnvId = ref<number | null>(null)
const showNewEnvInput = ref(false)
const newEnvName = ref('')
const newEnvInputRef = ref()
const editName = ref('')
const editBaseUrl = ref('')

const selectedEnv = computed(() =>
  envStore.environments.find(e => e.id === selectedEnvId.value) ?? null
)

// 选中环境后加载变量
watch(selectedEnvId, async (id) => {
  if (id !== null) {
    const env = envStore.environments.find(e => e.id === id)
    if (env) {
      editName.value = env.name
      editBaseUrl.value = env.base_url ?? ''
      await envStore.loadVariables(id)
    }
  }
})

// 弹窗打开时加载环境列表
watch(() => props.show, async (v) => {
  if (v) {
    const pid = projectStore.currentProjectId
    if (pid) {
      await envStore.loadEnvironments(pid)
      // 自动选中激活环境
      const active = envStore.environments.find(e => e.is_active === 1)
      if (active) selectedEnvId.value = active.id
    }
  }
})

function selectEnv(id: number) {
  selectedEnvId.value = id
}

async function handleCreateEnv() {
  const name = newEnvName.value.trim()
  if (!name || !projectStore.currentProjectId) return
  const env = await envStore.createEnvironment(projectStore.currentProjectId, name)
  newEnvName.value = ''
  showNewEnvInput.value = false
  selectedEnvId.value = env.id
}

async function handleDeleteEnv(id: number) {
  await envStore.deleteEnvironment(id)
  if (selectedEnvId.value === id) selectedEnvId.value = null
}

async function handleSaveEnv() {
  if (!selectedEnvId.value) return
  await envStore.updateEnvironment(
    selectedEnvId.value,
    editName.value,
    editBaseUrl.value || null,
  )
}

async function handleActivate() {
  if (!selectedEnv.value || !projectStore.currentProjectId) return
  if (selectedEnv.value.is_active) {
    await envStore.deactivateEnvironment(projectStore.currentProjectId)
  } else {
    await envStore.activateEnvironment(projectStore.currentProjectId, selectedEnv.value.id)
  }
}

async function handleAddVariable() {
  if (!selectedEnvId.value) return
  await envStore.createVariable(selectedEnvId.value, '', '')
}

// on-blur 更新变量 key
async function updateVariableKey(v: EnvVariable, newKey: string) {
  if (newKey === v.key) return
  await envStore.updateVariable(v.id, newKey, v.value, v.description, v.enabled)
}

// on-blur 更新变量 value
async function updateVariableValue(v: EnvVariable, newVal: string) {
  if (newVal === v.value) return
  await envStore.updateVariable(v.id, v.key, newVal, v.description, v.enabled)
}

async function toggleVariable(v: EnvVariable, enabled: boolean) {
  await envStore.updateVariable(v.id, v.key, v.value, v.description, enabled ? 1 : 0)
}
</script>

<style scoped>
.env-manager {
  display: flex;
  gap: 0;
  height: 420px;
  overflow: hidden;
}

.env-list {
  width: 180px;
  flex-shrink: 0;
  border-right: 1px solid var(--n-border-color, #e0e0e6);
  padding: 8px;
  overflow-y: auto;
}

.env-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
  transition: background 0.1s;
}
.env-item:hover { background: var(--n-item-color-hover, rgba(0,0,0,0.05)); }
.env-item.is-active-env { background: var(--n-item-color-active, rgba(24,160,88,0.08)); }
.env-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.env-detail {
  flex: 1;
  padding: 12px 16px;
  overflow-y: auto;
}
.env-detail-empty {
  display: flex;
  align-items: center;
  justify-content: center;
}

.detail-section { }
.section-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--n-text-color-3, #999);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 4px;
}

.var-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}
</style>
