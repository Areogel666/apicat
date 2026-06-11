/**
 * 主题 token 默认值常量
 *
 * 与 tokens.css 中的 :root / [data-theme="dark"] 值保持同步。
 * themeStore.resolvedTokens 用这些常量做基准，合并用户 customTokens。
 * 不通过 getComputedStyle 从 DOM 动态读 —— 静态常量更可靠。
 */

/** Light 模式下的全部 ~40 个 token 默认值 */
export const DEFAULT_LIGHT_TOKENS: Record<string, string> = {
  /* 中性色 */
  '--bg-base':        '#ffffff',
  '--bg-surface':     '#fafafa',
  '--bg-elevated':    '#ffffff',
  '--bg-hover':       'rgba(0, 0, 0, 0.04)',
  '--bg-active':      'rgba(0, 0, 0, 0.08)',
  '--bg-selected':    'rgba(24, 160, 88, 0.10)',
  '--border-base':    '#e5e5e5',
  '--border-strong':  '#d0d0d0',
  '--text-primary':   'rgba(0, 0, 0, 0.88)',
  '--text-secondary': 'rgba(0, 0, 0, 0.56)',
  '--text-tertiary':  'rgba(0, 0, 0, 0.36)',
  '--text-disabled':  'rgba(0, 0, 0, 0.24)',
  '--text-inverse':   '#ffffff',
  /* 品牌色 */
  '--color-primary':        '#18a058',
  '--color-primary-hover':  '#36ad6a',
  '--color-primary-press':  '#0c7a43',
  '--color-primary-soft':   'rgba(24, 160, 88, 0.12)',
  /* 语义色 */
  '--color-success':  '#18a058',
  '--color-warning':  '#f0a020',
  '--color-error':    '#d03050',
  '--color-info':     '#2080f0',
  /* JSON */
  '--json-key':       '#0550ae',
  '--json-string':    '#0a3069',
  '--json-number':    '#0550ae',
  '--json-boolean':   '#8250df',
  '--json-null':      '#999',
  /* Markdown */
  '--md-border':         '#d0d7de',
  '--md-link':           '#0969da',
  '--md-code-bg':        'rgba(175, 184, 193, 0.2)',
  '--md-pre-bg':         '#f6f8fa',
  '--md-blockquote':     '#57606a',
  '--md-blockquote-bar': '#d0d7de',
  '--md-th-bg':          '#f6f8fa',
  /* 形态 */
  '--radius-sm': '4px',
  '--radius-md': '6px',
  '--radius-lg': '10px',
}

/** Dark 模式下的全部 ~40 个 token 默认值 */
export const DEFAULT_DARK_TOKENS: Record<string, string> = {
  '--bg-base':        '#18181c',
  '--bg-surface':     '#1f1f23',
  '--bg-elevated':    '#2a2a30',
  '--bg-hover':       'rgba(255, 255, 255, 0.06)',
  '--bg-active':      'rgba(255, 255, 255, 0.10)',
  '--bg-selected':    'rgba(99, 205, 150, 0.16)',
  '--border-base':    '#2e2e33',
  '--border-strong':  '#3a3a40',
  '--text-primary':   'rgba(255, 255, 255, 0.92)',
  '--text-secondary': 'rgba(255, 255, 255, 0.62)',
  '--text-tertiary':  'rgba(255, 255, 255, 0.40)',
  '--text-disabled':  'rgba(255, 255, 255, 0.24)',
  '--text-inverse':   '#18181c',
  '--color-primary':        '#63cd96',
  '--color-primary-hover':  '#7fdfa8',
  '--color-primary-press':  '#4baa78',
  '--color-primary-soft':   'rgba(99, 205, 150, 0.16)',
  '--color-success':  '#63cd96',
  '--color-warning':  '#f2c97d',
  '--color-error':    '#e88080',
  '--color-info':     '#70c0e8',
  '--json-key':       '#79c0ff',
  '--json-string':    '#a5d6ff',
  '--json-number':    '#79c0ff',
  '--json-boolean':   '#d2a8ff',
  '--json-null':      '#6e7681',
  '--md-border':         '#30363d',
  '--md-link':           '#79c0ff',
  '--md-code-bg':        'rgba(110, 118, 129, 0.25)',
  '--md-pre-bg':         '#161b22',
  '--md-blockquote':     '#8b949e',
  '--md-blockquote-bar': '#30363d',
  '--md-th-bg':          '#161b22',
  '--radius-sm': '4px',
  '--radius-md': '6px',
  '--radius-lg': '10px',
}
