<template>
  <n-modal
    :show="show"
    preset="card"
    title="版本回退"
    :style="{ width: '620px', maxWidth: '94vw' }"
    :mask-closable="!installing"
    @update:show="onShowChange"
  >
    <n-alert type="warning" :bordered="false" class="rollback-warn">
      回退会安装所选版本并重启应用。旧版本可能无法识别新版本产生的数据，建议先导出项目备份。
    </n-alert>

    <div v-if="loading" class="rollback-hint">正在获取版本列表...</div>
    <div v-else-if="error" class="rollback-hint error">
      {{ error }}
      <n-button size="tiny" class="retry" @click="load">重试</n-button>
    </div>
    <div v-else-if="!candidates.length" class="rollback-hint">没有可回退的历史版本</div>
    <div v-else class="rollback-list">
      <div
        v-for="v in candidates"
        :key="v.tag"
        class="rollback-row"
        :class="{ active: selected === v.tag }"
        @click="selected = v.tag"
      >
        <div class="row-main">
          <span class="tag">v{{ v.version }}</span>
          <n-tag v-if="v.prerelease" size="tiny" type="warning">预发布</n-tag>
          <span class="date">{{ formatDate(v.published_at) }}</span>
        </div>
        <div v-if="v.name" class="row-name">{{ v.name }}</div>
      </div>
    </div>

    <template #footer>
      <n-space justify="end">
        <n-button quaternary :disabled="installing" @click="onShowChange(false)">取消</n-button>
        <n-button type="primary" :disabled="!selected" :loading="installing" @click="doRollback">
          {{ installText }}
        </n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NModal, NAlert, NButton, NSpace, NTag, useMessage } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'
import { relaunch } from '@tauri-apps/plugin-process'
import type { AppVersionInfo } from '../../types'

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ (e: 'update:show', v: boolean): void }>()

const message = useMessage()
const loading = ref(false)
const installing = ref(false)
const error = ref('')
const allVersions = ref<AppVersionInfo[]>([])
const currentVersion = ref('')
const selected = ref<string | null>(null)

/** 版本比较只处理 X.Y.Z 数字段（GitHub tag 已由 Rust 侧校验过格式） */
function parseVersion(v: string): number[] {
  return v.split('.').map(n => Number.parseInt(n, 10) || 0)
}
function compareVersion(a: string, b: string): number {
  const pa = parseVersion(a)
  const pb = parseVersion(b)
  for (let i = 0; i < 3; i++) {
    if (pa[i] !== pb[i]) return pa[i] - pb[i]
  }
  return 0
}

/** 只列严格低于当前版本的版本（回退语义），新版本仍走「检查更新」 */
const candidates = computed(() =>
  allVersions.value
    .filter(v => compareVersion(v.version, currentVersion.value) < 0)
    .sort((a, b) => compareVersion(b.version, a.version)),
)

const installText = computed(() => {
  if (installing.value) return '正在下载并安装...'
  return selected.value ? `回退到 v${selected.value.replace(/^v/, '')}` : '回退'
})

function formatDate(iso: string | null): string {
  if (!iso) return ''
  const d = new Date(iso) // GitHub 返回带 Z 的 ISO 串，跨引擎解析一致
  return Number.isNaN(d.getTime()) ? '' : d.toLocaleDateString()
}

async function load() {
  loading.value = true
  error.value = ''
  selected.value = null
  try {
    currentVersion.value = await getVersion()
    allVersions.value = await invoke<AppVersionInfo[]>('list_app_versions')
  } catch (e) {
    error.value = `获取版本列表失败：${e}`
  } finally {
    loading.value = false
  }
}

async function doRollback() {
  const tag = selected.value
  if (!tag) return
  installing.value = true
  try {
    await invoke('install_app_version', { tag })
    message.success(`v${tag.replace(/^v/, '')} 安装完成，正在重启...`)
    setTimeout(() => relaunch(), 1200)
  } catch (e) {
    message.error(`回退失败：${e}`)
  } finally {
    installing.value = false
  }
}

function onShowChange(v: boolean) {
  if (!v && installing.value) return
  emit('update:show', v)
}

watch(() => props.show, v => {
  if (v) void load()
})
</script>

<style scoped>
.rollback-warn {
  margin-bottom: var(--spacing-md);
}

.rollback-hint {
  padding: var(--spacing-lg);
  text-align: center;
  color: var(--text-tertiary);
  font-size: var(--font-size-sm);
}

.rollback-hint.error {
  color: var(--color-error, #d03050);
}

.retry {
  margin-left: var(--spacing-sm);
}

.rollback-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
  max-height: 46vh;
  overflow-y: auto;
}

.rollback-row {
  padding: var(--spacing-sm) var(--spacing-md);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}

.rollback-row:hover {
  background: var(--bg-hover);
}

.rollback-row.active {
  border-color: var(--color-primary, #18a058);
  background: color-mix(in srgb, var(--color-primary, #18a058) 8%, transparent);
}

.row-main {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}

.tag {
  font-family: monospace;
  font-weight: 600;
  color: var(--text-primary);
}

.date {
  margin-left: auto;
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
}

.row-name {
  margin-top: 2px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
