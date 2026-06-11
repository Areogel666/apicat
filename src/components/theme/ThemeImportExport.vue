<template>
  <div class="import-export">
    <n-button quaternary size="small" @click="handleExport">📤 导出当前主题</n-button>
    <n-button quaternary size="small" @click="triggerImport">📥 导入主题</n-button>
    <input
      ref="fileInputRef"
      type="file"
      accept=".json"
      class="hidden-input"
      @change="handleImport"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NButton, useMessage } from 'naive-ui'
import { useThemeStore } from '../../stores/theme'

const themeStore = useThemeStore()
const message = useMessage()
const fileInputRef = ref<HTMLInputElement | null>(null)

interface ThemeExport {
  name: string
  version: string
  mode: 'light' | 'dark'
  customTokens: Record<string, string>
  density: string
  radiusScale: number
  fontSize: string
}

function handleExport() {
  const data: ThemeExport = {
    name: 'ApiCat Custom Theme',
    version: '1.0',
    mode: themeStore.effectiveMode,
    customTokens: themeStore.customTokens,
    density: themeStore.density,
    radiusScale: themeStore.radiusScale,
    fontSize: themeStore.fontSize,
  }
  const json = JSON.stringify(data, null, 2)
  const blob = new Blob([json], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `apicat-theme-${data.mode}.json`
  a.click()
  URL.revokeObjectURL(url)
  message.success('主题已导出')
}

function triggerImport() {
  fileInputRef.value?.click()
}

function handleImport(e: Event) {
  const target = e.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return

  const reader = new FileReader()
  reader.onload = () => {
    try {
      const data = JSON.parse(reader.result as string)

      // 基础校验
      if (!data || typeof data !== 'object') {
        throw new Error('无效的主题文件格式')
      }

      if (data.customTokens && typeof data.customTokens === 'object') {
        themeStore.customTokens = { ...data.customTokens }
      }

      if (data.density === 'compact' || data.density === 'default' || data.density === 'spacious') {
        themeStore.density = data.density
      }

      if (data.radiusScale === 0.5 || data.radiusScale === 1.0 || data.radiusScale === 1.5) {
        themeStore.radiusScale = data.radiusScale
      }

      if (data.fontSize === 's' || data.fontSize === 'm' || data.fontSize === 'l') {
        themeStore.fontSize = data.fontSize
      }

      themeStore.applyTheme()
      message.success(`已导入主题「${data.name || '未命名'}」，请预览后点击"应用主题"`)
    } catch (err) {
      message.error(`导入失败：${(err as Error).message}`)
    }
  }
  reader.readAsText(file)

  // 重置 input 以允许重复导入同一文件
  target.value = ''
}
</script>

<style scoped>
.import-export {
  display: flex;
  gap: 4px;
}
.hidden-input {
  display: none;
}
</style>
