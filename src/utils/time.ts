/**
 * 当前时刻的 UTC 时间串，格式与 SQLite `datetime('now')` / Rust `cookies::TS_FMT`
 * 一致（`YYYY-MM-DD HH:MM:SS`，无时区标记）。
 *
 * **必须用 UTC**：`expires_at` 落库是 UTC，用本地时间比较会把「还在有效期内」
 * 判成已过期（或反之），而且偏差随机器时区变、测试里看不出来。
 */
export function nowUtcString(): string {
  const n = new Date()
  const p = (x: number) => String(x).padStart(2, '0')
  return `${n.getUTCFullYear()}-${p(n.getUTCMonth() + 1)}-${p(n.getUTCDate())} ${p(n.getUTCHours())}:${p(n.getUTCMinutes())}:${p(n.getUTCSeconds())}`
}
