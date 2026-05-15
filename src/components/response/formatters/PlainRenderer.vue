<template>
  <pre class="plain-body"><code>{{ body }}</code></pre>
</template>

<script setup lang="ts">
/**
 * 纯文本渲染器
 * 用于 Content-Type: text/plain 或无任何格式特征的响应。
 * 不做任何语法高亮，保留原样（包括空白符）。
 */
defineProps<{ body: string }>()
</script>

<style scoped>
.plain-body {
  flex: 1;
  overflow: auto;
  margin: 0;
  padding: 8px;
  background: var(--bg-surface);
  border-radius: 4px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
  font-size: 12.5px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--text-primary);
}
</style>
