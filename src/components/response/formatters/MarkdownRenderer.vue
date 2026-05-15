<template>
  <div class="markdown-preview-wrapper">
    <div class="markdown-body" v-html="rendered" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'

/**
 * Markdown 渲染器（Preview 模式）
 *
 * 安全策略（双保险）：
 *   1. markdown-it 配置 html: false —— 源码中的原始 HTML 标签被当作文本处理
 *   2. DOMPurify 再过滤一层，移除所有危险标签和属性（兜底防 markdown-it 升级引入漏洞）
 *
 * 不做：
 *   - GFM 扩展（表格、任务列表）—— markdown-it 默认未启用，本轮不加 plugin
 *   - 自定义主题 —— 使用最小化的 .markdown-body 样式
 */

const props = defineProps<{ body: string }>()

// html: false 禁止源码中的内嵌 HTML
// linkify: true 自动识别裸 URL 为链接
// breaks: false 单换行不转 <br>（标准 Markdown 行为）
const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: false,
})

const rendered = computed(() => {
  const raw = md.render(props.body)
  return DOMPurify.sanitize(raw, {
    USE_PROFILES: { html: true },
    // 额外禁用即使在 html 白名单里也可能有风险的标签
    FORBID_TAGS: ['style', 'script', 'iframe', 'object', 'embed', 'form', 'input'],
    FORBID_ATTR: ['onerror', 'onload', 'onclick'],
  })
})
</script>

<style scoped>
/*
 * Markdown 预览容器：
 *   外层 .markdown-preview-wrapper 负责 flex/滚动/边框背景
 *   内层 .markdown-body 负责排版样式
 */
.markdown-preview-wrapper {
  flex: 1;
  overflow: auto;
  padding: 16px 20px;
  background: var(--bg-surface);
  border-radius: 4px;
}

/*
 * 最小化 markdown 排版样式（不引入 github-markdown-css）
 * 覆盖常见元素即可，细节对标 GitHub 渲染的 60% 视觉
 */
.markdown-body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-primary);
}

.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4),
.markdown-body :deep(h5),
.markdown-body :deep(h6) {
  margin: 1.2em 0 0.6em;
  font-weight: 600;
  line-height: 1.25;
}
.markdown-body :deep(h1) { font-size: 1.8em; border-bottom: 1px solid #d0d7de; padding-bottom: 0.3em; }
.markdown-body :deep(h2) { font-size: 1.5em; border-bottom: 1px solid #d0d7de; padding-bottom: 0.3em; }
.markdown-body :deep(h3) { font-size: 1.25em; }
.markdown-body :deep(h4) { font-size: 1.1em; }

.markdown-body :deep(p) {
  margin: 0 0 0.8em;
}

.markdown-body :deep(a) {
  color: #0969da;
  text-decoration: none;
}
.markdown-body :deep(a:hover) {
  text-decoration: underline;
}

.markdown-body :deep(code) {
  background: rgba(175, 184, 193, 0.2);
  padding: 0.15em 0.4em;
  border-radius: 4px;
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
  font-size: 0.9em;
}

.markdown-body :deep(pre) {
  background: #f6f8fa;
  padding: 12px;
  border-radius: 6px;
  overflow: auto;
  margin: 0.8em 0;
}

.markdown-body :deep(pre code) {
  background: transparent;
  padding: 0;
  font-size: 0.85em;
}

.markdown-body :deep(blockquote) {
  margin: 0.8em 0;
  padding: 0 1em;
  color: #57606a;
  border-left: 4px solid #d0d7de;
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  padding-left: 2em;
  margin: 0.5em 0 0.8em;
}

.markdown-body :deep(li) {
  margin: 0.2em 0;
}

.markdown-body :deep(hr) {
  border: none;
  border-top: 1px solid #d0d7de;
  margin: 1.5em 0;
}

.markdown-body :deep(table) {
  border-collapse: collapse;
  margin: 0.8em 0;
}
.markdown-body :deep(th),
.markdown-body :deep(td) {
  border: 1px solid #d0d7de;
  padding: 6px 12px;
}
.markdown-body :deep(th) {
  background: #f6f8fa;
}

.markdown-body :deep(img) {
  max-width: 100%;
  height: auto;
}
</style>
