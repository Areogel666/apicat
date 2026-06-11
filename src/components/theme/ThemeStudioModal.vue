<template>
  <n-modal
    v-model:show="showModal"
    preset="card"
    title="主题工作室"
    style="width: 800px; max-height: 90vh;"
    :mask-closable="false"
  >
    <div class="theme-studio-body">
      <!-- 预设主题 -->
      <PresetThemeCards />

      <div class="section-divider" />

      <!-- 自定义配色 -->
      <div class="section">
        <div class="section-title">自定义配色</div>
        <TokenEditorGroup />
        <ContrastChecker />
      </div>

      <div class="section-divider" />

      <!-- 风格预设 -->
      <div class="section">
        <div class="section-title">风格预设</div>
        <StylePresetsPanel />
      </div>

      <div class="section-divider" />

      <!-- 实时预览 -->
      <div class="section">
        <div class="section-title">实时预览</div>
        <ThemePreview />
      </div>

      <!-- 底部操作栏 -->
      <div class="footer-bar">
        <ThemeImportExport />
        <n-space>
          <n-button quaternary @click="handleResetAll">全部重置</n-button>
          <n-button type="primary" @click="handleApply">应用主题</n-button>
        </n-space>
      </div>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NModal, NButton, NSpace, useMessage } from 'naive-ui'
import { useThemeStore } from '../../stores/theme'
import PresetThemeCards from './PresetThemeCards.vue'
import TokenEditorGroup from './TokenEditorGroup.vue'
import ContrastChecker from './ContrastChecker.vue'
import StylePresetsPanel from './StylePresetsPanel.vue'
import ThemePreview from './ThemePreview.vue'
import ThemeImportExport from './ThemeImportExport.vue'

const themeStore = useThemeStore()
const message = useMessage()

/** 弹窗开关（由外部 TopBar 控制） */
const showModal = ref(false)

/** 暴露给父组件调用 */
function open() {
  showModal.value = true
}
function close() {
  showModal.value = false
}

defineExpose({ open, close })

async function handleApply() {
  await themeStore.applyCustomTheme()
  message.success('主题已应用')
  showModal.value = false
}

async function handleResetAll() {
  await themeStore.resetAll()
  message.success('已恢复默认主题')
}
</script>

<style scoped>
.theme-studio-body {
  display: flex;
  flex-direction: column;
  gap: 0;
  max-height: 70vh;
  overflow-y: auto;
}
.section {
  padding: 4px 0;
}
.section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 10px;
}
.section-divider {
  border-top: 1px solid var(--border-base);
  margin: 12px 0;
}
.footer-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-top: 12px;
  border-top: 1px solid var(--border-base);
  margin-top: 8px;
}
</style>
