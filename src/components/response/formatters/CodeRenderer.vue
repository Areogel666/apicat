<template>
  <pre class="code-content hljs"><code v-html="highlighted" /></pre>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import hljs from 'highlight.js/lib/core'
import xml from 'highlight.js/lib/languages/xml'          // covers html + xml
import yaml from 'highlight.js/lib/languages/yaml'
import markdown from 'highlight.js/lib/languages/markdown'
import json from 'highlight.js/lib/languages/json'
import 'highlight.js/styles/github.css'

/**
 * 通用代码高亮渲染器（hljs 驱动）
 *
 * 用途：
 *   - XML / HTML 源码高亮
 *   - YAML 源码高亮
 *   - Markdown 源码高亮（Raw 模式）
 *   - JSON 原始模式（备用，目前 JsonRenderer 用的是无高亮 <pre>）
 *   - plaintext（安全回退）
 *
 * 只注册需要的语言，避免引入整个 hljs 包（~500KB）。
 */

// 注册一次（hljs 内部有去重逻辑，重复 registerLanguage 会被忽略）
hljs.registerLanguage('xml', xml)
hljs.registerLanguage('yaml', yaml)
hljs.registerLanguage('markdown', markdown)
hljs.registerLanguage('json', json)

type Lang = 'xml' | 'yaml' | 'markdown' | 'json' | 'plaintext'

const props = defineProps<{
  body: string
  /**
   * 高亮语言：
   *   - 'xml' 同时覆盖 HTML
   *   - 'plaintext' 不做语法高亮，仅保留 <pre> 样式
   */
  language: Lang
}>()

const highlighted = computed(() => {
  if (!props.body) return ''
  if (props.language === 'plaintext') {
    return escapeHtml(props.body)
  }
  try {
    return hljs.highlight(props.body, { language: props.language }).value
  } catch {
    // 高亮失败兜底为纯文本转义，绝不抛出到上层
    return escapeHtml(props.body)
  }
})

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}
</script>

<style scoped>
.code-content {
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
