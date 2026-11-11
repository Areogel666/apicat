/**
 * 响应 JSON 里的 Unix 时间戳识别（仅用于悬停提示，不改变原始值）。
 *
 * 识别条件（宁缺毋滥，避免把普通 ID 误当时间）：
 * - 字段名像时间字段：timestamp / unix_time / _time / _ts / _at / createTime 等；
 * - 值为 10 位秒级或 13 位毫秒级数字（number 或纯数字字符串）；
 * - 换算后落在 2000–2100 年，排除明显的 ID/随机数。
 */

const MIN_MS = Date.UTC(2000, 0, 1)
const MAX_MS = Date.UTC(2100, 0, 1)

function keyLooksLikeTime(key: string): boolean {
  const k = key.toLowerCase()
  return (
    k.includes('timestamp') ||
    /(^|_)(time|ts)($|_)/.test(k) ||
    /(_at|_ts)$/.test(k) ||
    /(create|update|modify|expire|start|end)time$/.test(k)
  )
}

function epochMs(value: unknown): { ms: number; unit: '秒' | '毫秒' } | null {
  const raw =
    typeof value === 'number'
      ? String(value)
      : typeof value === 'string'
        ? value.trim()
        : ''
  if (!/^\d{10}$|^\d{13}$/.test(raw)) return null
  const n = Number(raw)
  if (!Number.isSafeInteger(n)) return null
  const ms = raw.length === 10 ? n * 1000 : n
  if (ms < MIN_MS || ms >= MAX_MS) return null
  return { ms, unit: raw.length === 10 ? '秒' : '毫秒' }
}

const pad = (n: number) => String(n).padStart(2, '0')

/** 命中时返回悬停 title；未命中返回 null */
export function unixTimestampTooltip(key: string | null, value: unknown): string | null {
  if (!key || !keyLooksLikeTime(key)) return null
  const hit = epochMs(value)
  if (!hit) return null
  const d = new Date(hit.ms)
  const local = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  const utc = d.toISOString().slice(0, 19).replace('T', ' ')
  return `${local}（本地时间）\n${utc} UTC\n${hit.unit}级时间戳`
}
