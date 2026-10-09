/**
 * 毫秒时间戳 → UTC 时间串，格式与 SQLite `datetime('now')` / Rust `cookies::TS_FMT`
 * 一致（`YYYY-MM-DD HH:MM:SS`，无时区标记）。
 */
export function tsToUtcString(ts: number): string {
  const n = new Date(ts)
  const p = (x: number) => String(x).padStart(2, '0')
  return `${n.getUTCFullYear()}-${p(n.getUTCMonth() + 1)}-${p(n.getUTCDate())} ${p(n.getUTCHours())}:${p(n.getUTCMinutes())}:${p(n.getUTCSeconds())}`
}

/**
 * 当前时刻的 UTC 时间串。
 *
 * **必须用 UTC**：`expires_at` 落库是 UTC，用本地时间比较会把「还在有效期内」
 * 判成已过期（或反之），而且偏差随机器时区变、测试里看不出来。
 */
export function nowUtcString(): string {
  return tsToUtcString(Date.now())
}

/**
 * UTC 时间串 → 毫秒时间戳（给 `n-date-picker` 用）。无法解析返回 null。
 *
 * 显式按 UTC 解析而非 `new Date(s)`：后者对 `YYYY-MM-DD HH:MM:SS` 这种无时区
 * 标记的串，各引擎行为不一致（有的按本地时区），会整体偏移数小时。
 */
export function utcStringToTs(s: string | null | undefined): number | null {
  if (!s) return null
  const m = s.match(/^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2}):(\d{2})/)
  if (!m) return null
  return Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6])
}
