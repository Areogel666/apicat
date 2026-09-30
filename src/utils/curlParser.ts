import type { ParamItem } from '../types'

/**
 * cURL 命令解析（从浏览器「复制为 cURL」或手工命令导入接口）。
 *
 * 1.0.7 补齐（Q4-B / Q11-A / Q12-A）：
 *   - `-b/--cookie` 与 `-H 'Cookie: ...'` 语义相同，统一归一成一条 `Cookie` 串
 *   - `-u` → Authorization: Basic，`-A` → User-Agent，`-e` → Referer
 *   - `-L/-k/--compressed` 不再静默吞，收集到 `ignored` 由调用方显式提示
 *
 * 从 Sidebar.vue 抽出为纯函数，便于 `curlParser.check.ts` 直接跑自检。
 */
export interface ParsedCurl {
  method: string
  url: string
  headers: ParamItem[]
  bodyType: string
  body: string
  /** 归一后的 cookie 串（`a=1; b=2`），调用方写进接口 header 的 Cookie 行 */
  cookie: string
  /** 已识别但不支持/无需配置的参数说明 */
  ignored: string[]
}

export function parseCurl(curlText: string): ParsedCurl {
  const text = curlText.trim().replace(/\s*\\\s*\n\s*/g, ' ') // join line continuations
  const urlMatch = text.match(/https?:\/\/[^\s'"]+/)
  const url = urlMatch ? urlMatch[0].replace(/['"]/g, '') : ''
  const methodMatch = text.match(/-X\s+([A-Z]+)/i)
  const method = methodMatch
    ? methodMatch[1].toUpperCase()
    : (text.includes('-d ') || text.includes('--data') || text.includes('--data-urlencode') ? 'POST' : 'GET')

  const headers: ParamItem[] = []
  const headerRe = /-H\s+['"]([^'"]+)['"]/g
  let hm: RegExpExecArray | null
  while ((hm = headerRe.exec(text)) !== null) {
    const colonIdx = hm[1].indexOf(':')
    if (colonIdx > 0) {
      headers.push({
        key: hm[1].substring(0, colonIdx).trim(),
        value: hm[1].substring(colonIdx + 1).trim(),
        enabled: true,
      })
    }
  }

  // ── Cookie 归一（Q11-A）：`-b` 与 `-H 'Cookie: ...'` 走同一条路 ──
  const ignored: string[] = []
  const cookieParts: string[] = []
  const bQuoted = text.match(/(?:^|\s)(?:-b|--cookie)\s+['"]([^'"]*)['"]/)
  const bBare = bQuoted ? null : text.match(/(?:^|\s)(?:-b|--cookie)\s+(\S+)/)
  const bValue = (bQuoted?.[1] ?? bBare?.[1] ?? '').trim()
  if (bValue) {
    // curl 的 -b 既能接 cookie 串也能接 cookie jar 文件路径；无 `=` 视为文件，显式忽略
    if (bValue.includes('=')) {
      cookieParts.push(bValue)
    } else {
      ignored.push(`-b ${bValue}（cookie jar 文件不支持，已忽略）`)
    }
  }
  const cookieHeaderIdx = headers.findIndex(h => h.key.toLowerCase() === 'cookie')
  if (cookieHeaderIdx >= 0) {
    const v = headers[cookieHeaderIdx].value
    if (v) cookieParts.push(v)
    headers.splice(cookieHeaderIdx, 1) // 折进统一的 Cookie 串，避免双头
  }
  const cookie = cookieParts.join('; ')

  // ── `-u/--user` → Authorization: Basic（Q12-A）──
  const uQuoted = text.match(/(?:^|\s)(?:-u|--user)\s+['"]([^'"]+)['"]/)
  const uBare = uQuoted ? null : text.match(/(?:^|\s)(?:-u|--user)\s+(\S+)/)
  const uValue = (uQuoted?.[1] ?? uBare?.[1] ?? '').trim()
  if (uValue && !headers.some(h => h.key.toLowerCase() === 'authorization')) {
    // btoa 仅支持 Latin-1，先 UTF-8 编码以支持非 ASCII 账号/密码
    const encoded = btoa(unescape(encodeURIComponent(uValue)))
    headers.push({ key: 'Authorization', value: `Basic ${encoded}`, enabled: true })
  }

  // ── `-A/--user-agent`、`-e/--referer` → 对应 header（Q12-A）──
  const flagToHeader: Array<readonly [RegExp, RegExp, string]> = [
    [/(?:^|\s)(?:-A|--user-agent)\s+['"]([^'"]+)['"]/, /(?:^|\s)(?:-A|--user-agent)\s+(\S+)/, 'User-Agent'],
    [/(?:^|\s)(?:-e|--referer)\s+['"]([^'"]+)['"]/, /(?:^|\s)(?:-e|--referer)\s+(\S+)/, 'Referer'],
  ]
  for (const [quoted, bare, key] of flagToHeader) {
    const q = text.match(quoted)
    const b = q ? null : text.match(bare)
    const v = (q?.[1] ?? b?.[1] ?? '').trim()
    if (v && !headers.some(h => h.key.toLowerCase() === key.toLowerCase())) {
      headers.push({ key, value: v, enabled: true })
    }
  }

  // ── 传输层行为无法映射成 header，显式提示（Q12-A）──
  // 每项给全该 flag 的所有别名；单别名项不能走 split(' / ')，否则拼出 `(?:x|undefined)`
  for (const [aliases, note] of [
    [['-L', '--location'], '跟随重定向'],
    [['-k', '--insecure'], 'App 默认已忽略证书校验'],
    [['--compressed'], '压缩传输'],
  ] as const) {
    const re = new RegExp(`(?:^|\\s)(?:${aliases.join('|')})(?:\\s|$)`)
    if (re.test(text)) ignored.push(`${aliases.join(' / ')}（${note}，无需配置）`)
  }

  // 匹配 --data / -d / --data-binary / --data-raw / --data-urlencode（curl 的 data 参数全家族）
  // 用反向引用 \1 配对同一引号，body 内含双引号也不截断（如 --data-raw '{"a":1}' 的 JSON）
  const dataMatch =
    text.match(/(?:--data(?:-binary|-raw|-urlencode)?|-d)\s+(["'])([\s\S]*?)\1/s) ||
    text.match(/(?:--data(?:-binary|-raw|-urlencode)?|-d)\s+\$(["'])([\s\S]*?)\1/s)

  let body = ''
  if (dataMatch) {
    const rawValue = dataMatch[2] // [1] 是引号字符，[2] 才是 body 内容（反向引用正则）
    // --data-urlencode：值已经 URL 编码，解码后作为 body
    if (text.includes('--data-urlencode')) {
      try {
        body = decodeURIComponent(rawValue)
      } catch {
        body = rawValue
      }
    } else {
      body = rawValue
    }
  }

  let bodyType = 'none'
  if (body) {
    const ct = headers.find(h => h.key.toLowerCase() === 'content-type')?.value ?? ''
    if (ct.includes('json')) bodyType = 'raw_json'
    else if (ct.includes('form') && ct.includes('urlencoded')) bodyType = 'form_urlencoded'
    else bodyType = 'raw_text'
  }

  return { method, url, headers, bodyType, body, cookie, ignored }
}
