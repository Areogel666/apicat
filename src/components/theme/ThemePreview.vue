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
  font-size: 10px;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  margin-bottom: 8px;
}
.mini-app {
  display: flex;
  gap: 8px;
  background: var(--bg-base);
  border-radius: var(--radius-sm);
  padding: 10px;
  border: 1px solid var(--border-base);
}
.mini-sidebar {
  width: 40px;
  height: 60px;
  border-radius: 3px;
  padding: 4px;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.mini-nav-item {
  height: 10px;
  border-radius: 2px;
  width: 100%;
}
.mini-main {
  flex: 1;
  border-radius: 3px;
  padding: 6px 10px;
  font-size: 10px;
  min-height: 60px;
}
.mini-method {
  display: inline-block;
  font-weight: 600;
  padding: 0 4px;
  border: 1px solid;
  border-radius: 2px;
  font-size: 9px;
  margin-right: 4px;
}
.mini-url {
  font-size: 9px;
}
.mini-input {
  width: 100%;
  height: 14px;
  border-radius: 2px;
  border: 1px solid;
  margin-top: 4px;
}
.mini-btn {
  display: inline-block;
  padding: 1px 8px;
  border-radius: 2px;
  font-size: 9px;
  font-weight: 500;
  margin-top: 4px;
}
</style>
