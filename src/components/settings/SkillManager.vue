<template>
  <n-modal
    :show="show"
    preset="dialog"
    title="AI 技能管理"
    :show-icon="false"
    style="width: 560px"
    @update:show="(v: boolean) => emit('update:show', v)"
  >
    <div class="skill-manager">
      <p class="intro">
        将 ApiCat 的 AI 技能安装到本机的 AI 编码工具，使其能直接操作 ApiCat 本地库。
      </p>

      <div v-if="loading" class="loading">检测中...</div>

      <div v-else class="target-list">
        <div v-for="t in targets" :key="t.id" class="target-card">
          <div class="target-info">
            <div class="target-name">{{ t.name }}</div>
            <div class="target-path">{{ t.path }}</div>
            <div class="target-status">
              <template v-if="!t.agent_installed">
                <span class="status-badge not-found">未检测到</span>
                <span class="status-hint">该工具的 skills 目录不存在</span>
              </template>
              <template v-else-if="t.skills_installed">
                <span class="status-badge installed">已安装</span>
                <span class="status-hint">{{ t.installed_skills.length }} 个技能</span>
              </template>
              <template v-else>
                <span class="status-badge not-installed">未安装</span>
              </template>
            </div>
          </div>
          <div class="target-actions">
            <n-button
              v-if="t.agent_installed && !t.skills_installed"
              size="small"
              type="primary"
              :loading="installing === t.id"
              @click="install(t.id)"
            >安装</n-button>
            <n-button
              v-if="t.skills_installed"
              size="small"
              quaternary
              type="error"
              :loading="installing === t.id"
              @click="uninstall(t.id)"
            >卸载</n-button>
          </div>
        </div>
      </div>

      <div v-if="message" class="result-msg" :class="messageType">{{ message }}</div>
    </div>

    <template #action>
      <n-button @click="emit('update:show', false)">关闭</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { NModal, NButton } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'

interface SkillTarget {
  id: string
  name: string
  path: string
  agent_installed: boolean
  skills_installed: boolean
  installed_skills: string[]
}

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ 'update:show': [v: boolean] }>()

const loading = ref(false)
const targets = ref<SkillTarget[]>([])
const installing = ref<string | null>(null)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')

async function refresh() {
  loading.value = true
  try {
    targets.value = await invoke<SkillTarget[]>('get_skill_targets')
  } catch (e) {
    message.value = `检测失败：${e}`
    messageType.value = 'error'
  } finally {
    loading.value = false
  }
}

async function install(targetId: string) {
  installing.value = targetId
  message.value = ''
  try {
    const result = await invoke<string>('install_skills', { targetId })
    message.value = `安装成功（${result}）`
    messageType.value = 'success'
    await refresh()
  } catch (e) {
    message.value = `安装失败：${e}`
    messageType.value = 'error'
  } finally {
    installing.value = null
  }
}

async function uninstall(targetId: string) {
  installing.value = targetId
  message.value = ''
  try {
    await invoke('uninstall_skills', { targetId })
    message.value = '已卸载'
    messageType.value = 'success'
    await refresh()
  } catch (e) {
    message.value = `卸载失败：${e}`
    messageType.value = 'error'
  } finally {
    installing.value = null
  }
}

watch(() => props.show, (v) => {
  if (v) {
    message.value = ''
    refresh()
  }
})
</script>

<style scoped>
.intro {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 12px;
  line-height: 1.6;
}

.loading {
  text-align: center;
  padding: 24px;
  color: var(--text-tertiary);
}

.target-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.target-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px;
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
}

.target-info {
  flex: 1;
  min-width: 0;
}

.target-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.target-path {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: monospace;
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.target-status {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 4px;
}

.status-badge {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 3px;
  font-weight: 600;
}

.status-badge.installed {
  background: color-mix(in srgb, var(--color-success, #18a058) 12%, transparent);
  color: var(--color-success, #18a058);
}

.status-badge.not-installed {
  background: var(--bg-hover);
  color: var(--text-secondary);
}

.status-badge.not-found {
  background: color-mix(in srgb, var(--color-warning, #f0a020) 12%, transparent);
  color: var(--color-warning, #f0a020);
}

.status-hint {
  font-size: 11px;
  color: var(--text-tertiary);
}

.target-actions {
  flex-shrink: 0;
}

.result-msg {
  margin-top: 12px;
  padding: 8px 12px;
  border-radius: var(--radius-sm);
  font-size: 13px;
}

.result-msg.success {
  background: color-mix(in srgb, var(--color-success, #18a058) 10%, transparent);
  color: var(--color-success, #18a058);
}

.result-msg.error {
  background: color-mix(in srgb, var(--color-error, #d03050) 10%, transparent);
  color: var(--color-error, #d03050);
}
</style>
