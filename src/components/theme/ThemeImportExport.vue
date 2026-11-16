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
// 导出覆盖「主题工作室」全部可调项，并用系统保存对话框让用户明确知道落在哪。
// 之前用 <a download> 在 WebView 里静默落盘，路径不可见；且漏了玻璃质感四项。
import { ref } from 'vue'
import { NButton, useMessage } from 'naive-ui'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { useThemeStore, type GlassLevel } from '../../stores/theme'

const themeStore = useThemeStore()
const message = useMessage()
const fileInputRef = ref<HTMLInputElement | null>(null)

interface ThemeExport {
  name: string
  version: string
  mode: 'light' | 'dark'
  customTokens: Record<string, string>
  density: 'compact' | 'default' | 'spacious'
  radiusScale: 0.5 | 1.0 | 1.5
  fontSize: 's' | 'm' | 'l'
  glass: boolean
  glassBlurEnabled: boolean
  glassBlur: GlassLevel
  glassAlpha: GlassLevel
}

const GLASS_LEVELS: GlassLevel[] = ['light', 'medium', 'heavy']

async function handleExport() {
  const data: ThemeExport = {
    name: 'ApiCat Custom Theme',
    version: '1.1',
    mode: themeStore.effectiveMode,
    customTokens: { ...themeStore.customTokens },
    density: themeStore.density,
    radiusScale: themeStore.radiusScale,
    fontSize: themeStore.fontSize,
    glass: themeStore.glass,
    glassBlurEnabled: themeStore.glassBlurEnabled,
    glassBlur: themeStore.glassBlur,
    glassAlpha: themeStore.glassAlpha,
  }

  try {
    const savePath = await save({
      defaultPath: `apicat-theme-${data.mode}.json`,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!savePath) return // 用户取消
    await writeTextFile(savePath, JSON.stringify(data, null, 2))
    // 路径给足展示时间，用户能看清落在哪
    message.success(`已导出到：${savePath}`, { duration: 8000 })
  } catch (err) {
    message.error(`导出失败：${(err as Error).message}`)
  }
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

      if (!data || typeof data !== 'object') {
        throw new Error('无效的主题文件格式')
      }

      if (data.customTokens && typeof data.customTokens === 'object') {
        themeStore.customTokens = { ...data.customTokens }
      }

      if (data.mode === 'light' || data.mode === 'dark') {
        themeStore.mode = data.mode
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

      // 玻璃质感（1.1 起导出）；旧文件没有这些字段时保持当前值
      if (typeof data.glass === 'boolean') {
        themeStore.glass = data.glass
      }
      if (typeof data.glassBlurEnabled === 'boolean') {
        themeStore.glassBlurEnabled = data.glassBlurEnabled
      }
      if (GLASS_LEVELS.includes(data.glassBlur)) {
        themeStore.glassBlur = data.glassBlur
      }
      if (GLASS_LEVELS.includes(data.glassAlpha)) {
        themeStore.glassAlpha = data.glassAlpha
      }

      themeStore.applyTheme()
      message.success(`已导入主题「${data.name || '未命名'}」（${file.name}），关闭弹窗即保存`)
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
