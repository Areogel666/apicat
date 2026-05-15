/**
 * 响应体格式检测与工具函数
 *
 * 由 JsonViewer.vue（格式路由器）调用，决定用哪个 Renderer 子组件。
 * 检测规则：Content-Type 头优先，缺失时按 body 首字符启发式识别。
 */
import yaml from 'js-yaml'

export type ResponseFormat =
  | 'json'
  | 'xml'
  | 'html'
  | 'yaml'
  | 'markdown'
  | 'text'

/**
 * 视图模式：
 *   - raw     原始文本（带语法高亮）
 *   - pretty  结构化展示（JSON 树 / XML 缩进 / YAML 重排）
 *   - preview 渲染效果（Markdown 渲染 HTML / HTML sandbox iframe）
 *
 * 并非所有 format 支持所有 mode，详见 ViewModeSwitch 的可用项表。
 */
export type ViewMode = 'raw' | 'pretty' | 'preview'

/**
 * 根据 Content-Type 和 body 启发式识别响应格式。
 *
 * 顺序：
 *   1. 解析 Content-Type 中的 mime 主干
 *   2. mime 明确时直接映射
 *   3. mime 是 text/plain 或缺失时，用 body 首字符猜
 */
export function detectFormat(contentType: string, body: string): ResponseFormat {
  const ct = (contentType || '').toLowerCase()
  const head = body.trimStart().slice(0, 200)

  // 优先按 Content-Type 判定
  if (ct.includes('json')) return 'json'
  if (ct.includes('html')) return 'html'
  if (ct.includes('xml')) return 'xml'
  if (ct.includes('yaml') || ct.includes('x-yaml')) return 'yaml'
  if (ct.includes('markdown') || ct.includes('/md')) return 'markdown'
  if (ct.includes('text/')) {
    // text/plain 但 body 形似结构化数据 → 用内容猜
    if (head.startsWith('{') || head.startsWith('[')) return 'json'
    if (head.startsWith('<')) return 'xml'
    return 'text'
  }

  // 无 Content-Type：按 body 首字符启发式
  if (head.startsWith('{') || head.startsWith('[')) return 'json'
  if (/^<!doctype html/i.test(head) || /^<html/i.test(head)) return 'html'
  if (head.startsWith('<')) return 'xml'
  // YAML 启发式：首行形如 `key:` 或 `- key:`，且整体无大括号
  if (/^-?\s*[\w-]+:\s/.test(head) && !head.includes('{')) return 'yaml'
  return 'text'
}

/**
 * 给定格式，返回支持的视图模式列表（用于 ViewModeSwitch 渲染按钮组）。
 * 列表第一项为该格式的默认模式。
 */
export function availableViewModes(format: ResponseFormat): ViewMode[] {
  switch (format) {
    case 'json':
      return ['pretty', 'raw']
    case 'xml':
      return ['pretty', 'raw']
    case 'yaml':
      return ['pretty', 'raw']
    case 'markdown':
      return ['raw', 'preview']
    case 'html':
      return ['raw', 'preview']
    case 'text':
      return ['raw']
  }
}

/**
 * 返回格式的默认视图模式（列表第一项）。
 */
export function defaultViewMode(format: ResponseFormat): ViewMode {
  return availableViewModes(format)[0]
}

/**
 * 返回该格式在 UI 上显示的名称。
 */
export function formatLabel(format: ResponseFormat): string {
  switch (format) {
    case 'json': return 'JSON'
    case 'xml': return 'XML'
    case 'html': return 'HTML'
    case 'yaml': return 'YAML'
    case 'markdown': return 'Markdown'
    case 'text': return 'Text'
  }
}

/**
 * 视图模式在 UI 上显示的名称。
 */
export function viewModeLabel(mode: ViewMode): string {
  switch (mode) {
    case 'raw': return '原始'
    case 'pretty': return '美化'
    case 'preview': return '预览'
  }
}

/**
 * XML / HTML 美化（加缩进换行）
 *
 * 策略：基于正则分词，把 body 切成 token 流，按标签开闭调整缩进。
 * 不依赖 DOM 解析（DOMParser 对非法 XML 会报错，对部分 HTML 片段也挑剔），
 * 这里做纯文本层面的格式化，对格式良好的 XML/HTML 效果已经足够。
 *
 * 处理 token：
 *   - <?xml ... ?>     声明
 *   - <!-- ... -->     注释
 *   - <![CDATA[...]]>  CDATA
 *   - <foo ...>        开标签（不含 <br>/<img> 等自闭合，通过 /> 区分）
 *   - <foo />          自闭合
 *   - </foo>           闭标签
 *   - 文本节点         去空白后保留
 *
 * 异常情况（不配对的标签、残缺片段）：退化为原文本，不抛异常。
 */
export function formatXml(body: string, indent: string = '  '): string {
  if (!body || !body.trim()) return body

  try {
    // 移除标签间的空白文本（保留标签内的文本内容）
    // 正则：先把原始流按标签边界切开
    const tokens: string[] = []
    const tagRegex = /<\?[\s\S]*?\?>|<!--[\s\S]*?-->|<!\[CDATA\[[\s\S]*?\]\]>|<\/?[^>]+?>/g

    let lastIndex = 0
    let match: RegExpExecArray | null
    while ((match = tagRegex.exec(body)) !== null) {
      // 标签前的文本（可能是空白或节点内容）
      const text = body.slice(lastIndex, match.index)
      if (text.trim()) {
        tokens.push(text.trim())
      }
      tokens.push(match[0])
      lastIndex = match.index + match[0].length
    }
    // 最后残留的文本
    const tail = body.slice(lastIndex)
    if (tail.trim()) tokens.push(tail.trim())

    if (tokens.length === 0) return body

    const lines: string[] = []
    let depth = 0

    for (let i = 0; i < tokens.length; i++) {
      const t = tokens[i]
      const isDeclaration = /^<\?/.test(t)
      const isComment = /^<!--/.test(t)
      const isCData = /^<!\[CDATA\[/.test(t)
      const isClose = /^<\//.test(t)
      const isSelfClose = /\/>$/.test(t) || /^<(br|hr|img|input|meta|link|area|base|col|embed|source|track|wbr)\b[^>]*>$/i.test(t)
      const isOpen = !isClose && !isDeclaration && !isComment && !isCData && !isSelfClose && /^</.test(t)
      const isText = !t.startsWith('<')

      // 特殊情况：开标签后紧跟文本再紧跟其闭标签（单行短节点 <foo>bar</foo>）
      if (isOpen && i + 2 < tokens.length) {
        const next = tokens[i + 1]
        const nextNext = tokens[i + 2]
        if (!next.startsWith('<') && /^<\//.test(nextNext)) {
          lines.push(indent.repeat(depth) + t + next + nextNext)
          i += 2
          continue
        }
      }

      if (isClose) {
        depth = Math.max(0, depth - 1)
        lines.push(indent.repeat(depth) + t)
      } else if (isOpen) {
        lines.push(indent.repeat(depth) + t)
        depth++
      } else if (isText) {
        lines.push(indent.repeat(depth) + t)
      } else {
        // 声明、注释、CDATA、自闭合
        lines.push(indent.repeat(depth) + t)
      }
    }

    return lines.join('\n')
  } catch {
    return body
  }
}

/**
 * YAML 美化（load → dump 重排）
 *
 * 效果：
 *   - 统一缩进为 2 空格
 *   - 标准化引号用法
 *   - 按 key 保持插入顺序（js-yaml 默认）
 *
 * 解析失败时返回原 body —— 用户仍能看到源码（由 CodeRenderer 做 YAML 语法高亮）。
 */
export function formatYaml(body: string, indent: number = 2): string {
  if (!body || !body.trim()) return body
  try {
    const parsed = yaml.load(body)
    // yaml.dump 对 null / undefined 会输出空字符串或 'null\n'，回退原文更安全
    if (parsed === null || parsed === undefined) return body
    return yaml.dump(parsed, {
      indent,
      lineWidth: -1,     // 不换行，避免长字符串被截断
      noRefs: true,      // 禁用 YAML 锚点/别名，输出更易读
      sortKeys: false,   // 保留原始 key 顺序
    })
  } catch {
    return body
  }
}
