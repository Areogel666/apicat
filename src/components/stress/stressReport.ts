// 压测报告的「取回 / 渲染 / 存盘」侧工具。
//
// 报告内容（说什么）的真源在 Rust：`commands::stress::build_stress_report_markdown`，
// 经 IPC `get_stress_report` 取回 Markdown 文本。App 与 bridge 技能共用同一份内容。
// 本文件只负责：取回 → 渲染成预览 HTML → 包成可分享的 HTML 文档 → 存盘。
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import { invoke } from '@tauri-apps/api/core'
import type { StressRun } from '../../types'

/** markdown-it 默认 preset 即启用 table 规则，无需插件（已实测确认） */
const md = new MarkdownIt({ html: false, linkify: true, breaks: false })

/** 从 Rust 取回报告 Markdown（内容真源） */
export async function fetchReportMarkdown(runId: number): Promise<string> {
  return invoke<string>('get_stress_report', { runId })
}

/**
 * 状态码语义说明。
 * 注意：Rust 侧 `status_hint` 有一份同名实现，用于报告的「说明」列；
 * 本函数只服务于 App 的状态码分布条。两处都是 5 分支的稳定映射，刻意不强行统一。
 */
export function statusHint(code: number): string {
  if (code === 0) return '网络错误（未拿到响应）'
  if (code >= 200 && code < 300) return '成功'
  if (code >= 300 && code < 400) return '重定向'
  if (code >= 400 && code < 500) return '客户端错误'
  if (code >= 500 && code < 600) return '服务端错误'
  return '未知'
}

/** 预览用：渲染 + 双层消毒（与 response/formatters/MarkdownRenderer.vue 同策略） */
export function renderReportHtml(markdown: string): string {
  const raw = md.render(markdown)
  return DOMPurify.sanitize(raw, {
    USE_PROFILES: { html: true },
    FORBID_TAGS: ['style', 'script', 'iframe', 'object', 'embed', 'form', 'input'],
    FORBID_ATTR: ['onerror', 'onload', 'onclick'],
  })
}

const HTML_STYLE = `
:root { color-scheme: light dark; }
* { box-sizing: border-box; }
body {
  margin: 0; padding: 32px 20px;
  font: 14px/1.65 -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
  background: #fafafa; color: #1f1f1f;
}
.report { max-width: 860px; margin: 0 auto; background: #fff; border: 1px solid #e8e8e8; border-radius: 10px; padding: 28px 32px; }
h1 { font-size: 22px; margin: 0 0 12px; }
h2 { font-size: 16px; margin: 28px 0 10px; padding-bottom: 6px; border-bottom: 1px solid #eee; }
table { border-collapse: collapse; width: 100%; margin: 8px 0 4px; font-size: 13px; }
th, td { border: 1px solid #e8e8e8; padding: 6px 10px; text-align: left; }
th { background: #f5f5f5; font-weight: 600; }
blockquote { margin: 0 0 4px; padding: 10px 14px; background: #f7f7f7; border-left: 3px solid #18a058; border-radius: 0 6px 6px 0; color: #444; }
blockquote p { margin: 2px 0; }
code { background: #f2f2f2; padding: 1px 5px; border-radius: 4px; font-family: ui-monospace, Consolas, monospace; font-size: 12px; }
ul { padding-left: 22px; }
li { margin: 4px 0; }
@media (prefers-color-scheme: dark) {
  body { background: #18181c; color: #e8e8e8; }
  .report { background: #1f1f24; border-color: #333; }
  h2 { border-color: #333; }
  th, td { border-color: #333; }
  th { background: #26262c; }
  blockquote { background: #26262c; color: #ccc; }
  code { background: #2a2a31; }
}
`

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, c => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c] as string))
}

/** 导出用：把 Markdown 包成自包含单文件 HTML（内联样式，跟随系统深浅色） */
export function wrapReportHtml(markdown: string, title: string): string {
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<style>${HTML_STYLE}</style>
</head>
<body>
<article class="report">
${md.render(markdown)}</article>
</body>
</html>
`
}

/** 报告存盘（Tauri dialog + fs）。返回是否真的写入了文件（用户取消返回 false） */
export async function saveReportToFile(
  text: string,
  defaultName: string,
  kind: 'Markdown' | 'HTML' = 'Markdown',
): Promise<boolean> {
  try {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const { writeTextFile } = await import('@tauri-apps/plugin-fs')
    const ext = kind === 'HTML' ? 'html' : 'md'
    const path = await save({
      title: '保存压测报告',
      defaultPath: defaultName,
      filters: [{ name: kind, extensions: [ext] }],
    })
    if (!path) return false
    await writeTextFile(path, text)
    return true
  } catch (e) {
    console.warn('[stress] 保存报告失败:', e)
    return false
  }
}

/** 默认文件名：把 SQLite 的 `2026-09-18 14:03:22` 转成文件名安全形式 */
export function reportFileName(run: StressRun, ext: 'md' | 'html'): string {
  const stamp = run.created_at.trim().replace(/[: ]/g, '-').replace(/\.\d+$/, '')
  return `stress-report-${stamp}.${ext}`
}
