<template>
  <n-modal
    v-model:show="show"
    preset="dialog"
    title="导入接口"
    :show-icon="false"
    style="width: 460px"
    positive-text="导入"
    negative-text="取消"
    :loading="loading"
    @positive-click="handleImport"
    @negative-click="show = false"
  >
    <div class="import-body">
      <!-- 导入格式 -->
      <div class="field-row">
        <span class="field-label">导入格式</span>
        <n-radio-group v-model:value="format" size="small">
          <n-radio value="postman">Postman Collection v2.1</n-radio>
          <n-radio value="openapi">OpenAPI 3.x（JSON / YAML）</n-radio>
          <n-radio value="apicat">ApiCat 格式</n-radio>
        </n-radio-group>
      </div>

      <!-- 目标项目 -->
      <div class="field-row">
        <span class="field-label">导入到</span>
        <n-select
          v-model:value="targetProjectId"
          :options="projectOptions"
          size="small"
          style="flex: 1"
          placeholder="新建项目（0=新建）"
        />
      </div>

      <!-- 文件选择 -->
      <div class="field-row">
        <span class="field-label">文件</span>
        <n-button size="small" @click="selectFile">📂 选择文件</n-button>
        <span class="file-name" v-if="selectedFileName">{{ selectedFileName }}</span>
      </div>

      <div class="import-error" v-if="errorMsg">❌ {{ errorMsg }}</div>
      <div class="import-success" v-if="successMsg">✅ {{ successMsg }}</div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NModal, NRadioGroup, NRadio, NSelect, NButton } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { useProjectStore } from '../../stores/project'

const show = defineModel<boolean>('show', { required: true })
const emit = defineEmits<{ imported: [projectId: number] }>()

const projectStore = useProjectStore()
const format = ref<'postman' | 'openapi' | 'apicat'>('postman')
const targetProjectId = ref<number>(0)
const selectedFilePath = ref<string | null>(null)
const selectedFileName = ref('')
const loading = ref(false)
const errorMsg = ref('')
const successMsg = ref('')

const projectOptions = computed(() => [
  { label: '新建项目', value: 0 },
  ...projectStore.projects.map(p => ({ label: p.name, value: p.id })),
])

async function selectFile() {
  errorMsg.value = ''
  const filters = format.value === 'openapi'
    ? [{ name: 'OpenAPI', extensions: ['json', 'yaml', 'yml'] }]
    : [{ name: 'JSON', extensions: ['json'] }]

  const selected = await open({ filters, multiple: false })
  if (!selected || typeof selected !== 'string') return
  selectedFilePath.value = selected
  selectedFileName.value = selected.split(/[/\\]/).pop() ?? selected
}

async function handleImport() {
  if (!selectedFilePath.value) {
    errorMsg.value = '请先选择文件'
    return false
  }
  loading.value = true
  errorMsg.value = ''
  successMsg.value = ''

  try {
    const content = await readTextFile(selectedFilePath.value)
    const pid = targetProjectId.value

    let importedPid: number
    // Tauri 2.x #[command] 宏把 Rust snake_case 参数名转为 camelCase IPC key
    if (format.value === 'postman') {
      importedPid = await invoke<number>('import_postman', { projectId: pid, jsonContent: content })
    } else if (format.value === 'apicat') {
      importedPid = await invoke<number>('import_apicat', { projectId: pid, jsonContent: content })
    } else {
      const isYaml = selectedFilePath.value.endsWith('.yaml') || selectedFilePath.value.endsWith('.yml')
      importedPid = await invoke<number>('import_openapi', { projectId: pid, content, isYaml })
    }

    await projectStore.loadProjects()
    successMsg.value = `导入成功，项目 ID: ${importedPid}`
    emit('imported', importedPid)
    setTimeout(() => { show.value = false }, 2000)
  } catch (e) {
    errorMsg.value = String(e)
  } finally {
    loading.value = false
  }
  return false
}
</script>

<style scoped>
.import-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 4px 0;
}
.field-row {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}
.field-label {
  width: 70px;
  font-size: 13px;
  flex-shrink: 0;
  padding-top: 3px;
}
.file-name {
  font-size: 12px;
  color: var(--n-text-color-3, #999);
  word-break: break-all;
  align-self: center;
}
.import-error  { font-size: 12px; color: #d03050; }
.import-success { font-size: 12px; color: #18a058; }
</style>
