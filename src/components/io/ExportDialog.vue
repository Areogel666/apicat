<template>
  <n-modal
    v-model:show="show"
    preset="dialog"
    title="导出接口"
    :show-icon="false"
    style="width: 400px"
    positive-text="导出"
    negative-text="取消"
    :loading="loading"
    @positive-click="handleExport"
    @negative-click="show = false"
  >
    <div class="export-body">
      <!-- 导出格式 -->
      <div class="field-row">
        <span class="field-label">导出格式</span>
        <n-radio-group v-model:value="format" size="small">
          <n-radio value="apicat">ApiCat 格式（含环境变量）</n-radio>
          <n-radio value="postman">Postman Collection v2.1</n-radio>
        </n-radio-group>
      </div>

      <!-- 导出项目 -->
      <div class="field-row">
        <span class="field-label">导出项目</span>
        <n-select
          v-model:value="exportProjectId"
          :options="projectOptions"
          size="small"
          style="flex: 1"
        />
      </div>

      <div class="export-error"   v-if="errorMsg">❌ {{ errorMsg }}</div>
      <div class="export-success" v-if="successMsg">✅ {{ successMsg }}</div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NModal, NRadioGroup, NRadio, NSelect } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { useProjectStore } from '../../stores/project'

const show = defineModel<boolean>('show', { required: true })

const projectStore = useProjectStore()
const format = ref<'apicat' | 'postman'>('apicat')
const exportProjectId = ref<number | null>(projectStore.currentProjectId ?? null)
const loading = ref(false)
const errorMsg = ref('')
const successMsg = ref('')

const projectOptions = computed(() =>
  projectStore.projects.map(p => ({ label: p.name, value: p.id }))
)

async function handleExport() {
  if (!exportProjectId.value) {
    errorMsg.value = '请选择要导出的项目'
    return false
  }
  loading.value = true
  errorMsg.value = ''
  successMsg.value = ''

  try {
    let content: string
    let defaultFilename: string

    if (format.value === 'apicat') {
      // Tauri 2.x #[command] 宏把 Rust snake_case 参数名转为 camelCase IPC key
      content = await invoke<string>('export_apicat', { projectId: exportProjectId.value })
      defaultFilename = 'apicat-export.json'
    } else {
      content = await invoke<string>('export_postman', { projectId: exportProjectId.value })
      defaultFilename = 'postman-collection.json'
    }

    const savePath = await save({
      defaultPath: defaultFilename,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!savePath) { loading.value = false; return false }

    await writeTextFile(savePath, content)
    successMsg.value = `已保存到：${savePath}`
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
.export-body {
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
.export-error   { font-size: 12px; color: #d03050; }
.export-success { font-size: 12px; color: #18a058; }
</style>
