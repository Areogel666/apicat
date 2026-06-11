<template>
  <div class="html-preview-wrapper">
    <iframe
      class="html-preview-frame"
      sandbox=""
      :srcdoc="themedSrcdoc"
      referrerpolicy="no-referrer"
      title="HTML 响应预览"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useThemeStore } from '../../../stores/theme'

/**
 * HTML 预览渲染器（sandbox iframe）
 *
 * 安全策略：
 *   - sandbox=""（空字符串）= 最严格沙箱
 *   - referrerpolicy="no-referrer"
 *   - [1.0.3 新增] srcdoc 拼接时注入内联 <style>，跟随当前主题色
 */

const props = defineProps<{ body: string }>()
const themeStore = useThemeStore()

const themedSrcdoc = computed(() => {
  // 显式依赖 themeTick：主题切换 / 自定义 token 修改时 themeTick 递增，
  // 触发 computed 重算，确保 iframe 内 srcdoc 的色值跟随主题实时更新。
  // getComputedStyle 本身不是 Vue 响应式 API，不能自动追踪。
  void themeStore.themeTick

  const cs = getComputedStyle(document.documentElement)
  const bg = cs.getPropertyValue('--bg-base').trim() || '#fff'
  const text = cs.getPropertyValue('--text-primary').trim() || 'rgba(0,0,0,0.88)'
  const link = cs.getPropertyValue('--md-link').trim() || '#0969da'

  return `<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><style>
  body {
    background: ${bg};
    color: ${text};
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    margin: 16px;
    line-height: 1.5;
  }
  a { color: ${link}; }
  pre, code {
    background: rgba(128,128,128,0.1);
    border-radius: 3px;
    padding: 0.15em 0.4em;
    font-family: monospace;
  }
</style></head>
<body>${props.body}</body>
</html>`
})
</script>

<style scoped>
.html-preview-wrapper {
  flex: 1;
  display: flex;
  overflow: hidden;
  border-radius: 4px;
  border: 1px solid var(--border-base);
  background: var(--bg-base);
}

.html-preview-frame {
  flex: 1;
  width: 100%;
  border: none;
  background: var(--bg-base);
}
</style>
