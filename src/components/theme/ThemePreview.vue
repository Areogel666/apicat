<template>
  <div class="theme-preview-card">
    <div class="preview-label">实时预览</div>
    <div class="mini-app" :style="previewStyle">
      <!-- 侧边栏 -->
      <div class="mini-sidebar" :style="{ background: t('--bg-elevated') }">
        <div class="mini-nav-item" :style="{ background: t('--bg-hover') }" />
        <div class="mini-nav-item" />
        <div class="mini-nav-item" />
      </div>
      <!-- 主面板 -->
      <div class="mini-main" :style="{ background: t('--bg-elevated') }">
        <!-- HTTP 方法标签 -->
        <span
          class="mini-method"
          :style="{ color: t('--color-primary'), borderColor: t('--color-primary') }"
        >GET</span>
        <span class="mini-url" :style="{ color: t('--text-secondary') }">/api/users</span>
        <!-- 输入框 -->
        <div
          class="mini-input"
          :style="{
            background: t('--bg-base'),
            borderColor: t('--border-base'),
          }"
        />
        <!-- 按钮 -->
        <span
          class="mini-btn"
          :style="{
            background: t('--color-primary'),
            color: '#000',
          }"
        >Send</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useThemeStore } from '../../stores/theme'

const themeStore = useThemeStore()

/** 读取当前生效 token 值 */
function t(key: string): string {
  return themeStore.resolvedTokens[key] || ''
}

/** 预览容器注入 token 变量，确保预览卡片内颜色反映当前编辑值 */
const previewStyle = computed(() => {
  const tokens = themeStore.resolvedTokens
  const vars = Object.entries(tokens)
    .map(([k, v]) => `${k}: ${v}`)
    .join('; ')
  return vars
})
</script>

<style scoped>
.theme-preview-card {
  background: var(--bg-elevated);
  border-radius: var(--radius-md);
  padding: 12px;
  border: 1px solid var(--border-base);
}
.preview-label {
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  margin-bottom: var(--spacing-sm);
}
.mini-app {
  display: flex;
  gap: var(--spacing-sm);
  background: var(--bg-base);
  border-radius: var(--radius-sm);
  padding: var(--spacing-sm);
  border: 1px solid var(--border-base);
}
.mini-sidebar {
  width: 40px;
  border-radius: var(--radius-sm);
  padding: var(--spacing-xs);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}
.mini-nav-item {
  height: calc(var(--row-height) / 2 - 1px);
  border-radius: var(--radius-sm);
  width: 100%;
}
.mini-main {
  flex: 1;
  border-radius: var(--radius-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  font-size: var(--font-size-sm);
  min-height: calc(var(--row-height) * 2 - 4px);
}
.mini-method {
  display: inline-block;
  font-weight: 600;
  padding: 0 var(--spacing-xs);
  border: 1px solid;
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  margin-right: var(--spacing-xs);
}
.mini-url {
  font-size: var(--font-size-sm);
}
.mini-input {
  width: 100%;
  height: calc(var(--input-height) / 2);
  border-radius: var(--radius-sm);
  border: 1px solid;
  margin-top: var(--spacing-xs);
}
.mini-btn {
  display: inline-block;
  padding: 1px var(--spacing-sm);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  font-weight: 500;
  margin-top: var(--spacing-xs);
}
</style>
