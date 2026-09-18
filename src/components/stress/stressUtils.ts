// 压测面板公共工具：统计格式化 / 报告生成 / Canvas 绘制
// 由 StressResultPanel（弹窗）与 StressTab（编辑区内嵌 tab）共用，避免两份实现漂移。
import type { StressStats, StressChartPoint } from '../../types'

// 1.0.4：耗时直方图桶标签（与 Rust LATENCY_BUCKETS 一致：10 桶）
export const LATENCY_LABELS = ['<1', '1-2', '2-5', '5-10', '10-20', '20-50', '50-100', '100-200', '200-500', '>500']
/** 期望状态码默认值，与 Rust DEFAULT_EXPECT_STATUS 保持一致 */
export const DEFAULT_EXPECT_STATUS = '2xx'
/** 耗时直方图最大柱高（px） */
export const MAX_HIST_BAR_HEIGHT = 40

/** 从 :root CSS 变量读取色值，用于 canvas 绘制时跟随主题 */
export function readToken(name: string, fallback: string): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return v || fallback
}

export function histHeight(bucket: { pct: number }): number {
  return Math.max(2, Math.round(bucket.pct / 100 * MAX_HIST_BAR_HEIGHT))
}

/** 状态码 → 颜色 token */
export function statusColor(code: number): string {
  if (code === 0) return readToken('--status-network', '#909399')
  if (code >= 200 && code < 300) return readToken('--status-2xx', '#18a058')
  if (code >= 300 && code < 400) return readToken('--status-3xx', '#2080f0')
  if (code >= 400 && code < 500) return readToken('--status-4xx', '#f0a020')
  return readToken('--status-5xx', '#d03050')
}

export function pct(cnt: number, status_counts?: Array<[number, number]>): number {
  const total = (status_counts ?? []).reduce((s, [_, n]) => s + n, 0)
  if (!total) return 0
  return Math.round(cnt / total * 100)
}

export function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

// 报告生成已下沉到 Rust（commands::stress::build_stress_report_markdown），
// 前端只做取回/渲染/存盘，见 ./stressReport.ts

/** 历史条目的一行摘要。区分响应率与业务成功率，旧记录不显示业务口径 */
export function summarizeStats(run: { config_json: string; stats_json: string }): string {
  let st: StressStats | null = null
  try { st = JSON.parse(run.stats_json) } catch {}
  if (!st) return ''
  const biz = st.biz_success_rate == null ? '' : ` 业务${st.biz_success_rate.toFixed(1)}%`
  return `总${st.total} 响应${st.success_rate.toFixed(1)}%${biz} TPS${st.tps.toFixed(1)} P95${st.p95_ms}ms`
}

/** 实时折线图：TPS / avg / p95 三线（Y 轴归一化） */
export function drawStressChart(canvas: HTMLCanvasElement, points: StressChartPoint[]) {
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  const W = canvas.width
  const H = canvas.height
  const PAD = { top: 16, right: 16, bottom: 24, left: 48 }
  const innerW = W - PAD.left - PAD.right
  const innerH = H - PAD.top - PAD.bottom

  ctx.clearRect(0, 0, W, H)

  const bgColor      = readToken('--bg-surface', '#fafafa')
  const textTertiary = readToken('--text-tertiary', '#999')
  const borderColor  = readToken('--border-base', '#e8e8e8')
  const tpsColor     = readToken('--color-success', '#18a058')
  const avgColor     = readToken('--color-info', '#2080f0')
  const p95Color     = readToken('--color-warning', '#f0a020')

  ctx.fillStyle = bgColor
  ctx.fillRect(0, 0, W, H)

  if (points.length < 2) {
    ctx.fillStyle = textTertiary
    ctx.font = '12px sans-serif'
    ctx.textAlign = 'center'
    ctx.fillText('等待数据...', W / 2, H / 2)
    return
  }

  const maxTps = Math.max(...points.map(p => p.tps), 1)
  const maxMs = Math.max(...points.map(p => p.p95_ms), 1)

  ctx.strokeStyle = borderColor
  ctx.lineWidth = 1
  for (let i = 0; i <= 4; i++) {
    const y = PAD.top + (innerH * i) / 4
    ctx.beginPath()
    ctx.moveTo(PAD.left, y)
    ctx.lineTo(PAD.left + innerW, y)
    ctx.stroke()
  }

  const xForIdx = (i: number) => PAD.left + (i / (points.length - 1)) * innerW

  function drawLine(values: number[], maxVal: number, color: string, lineWidth = 1.5) {
    ctx!.strokeStyle = color
    ctx!.lineWidth = lineWidth
    ctx!.lineJoin = 'round'
    ctx!.beginPath()
    values.forEach((v, i) => {
      const x = xForIdx(i)
      const y = PAD.top + innerH - (v / maxVal) * innerH
      if (i === 0) ctx!.moveTo(x, y)
      else ctx!.lineTo(x, y)
    })
    ctx!.stroke()
  }

  drawLine(points.map(p => p.tps), maxTps, tpsColor, 2)
  drawLine(points.map(p => p.avg_ms), maxMs, avgColor, 1.5)
  drawLine(points.map(p => p.p95_ms), maxMs, p95Color, 1.5)

  ctx.fillStyle = textTertiary
  ctx.font = '10px sans-serif'
  ctx.textAlign = 'center'
  const labelCount = Math.min(5, points.length)
  for (let i = 0; i < labelCount; i++) {
    const idx = Math.floor((i / (labelCount - 1)) * (points.length - 1))
    const x = xForIdx(idx)
    ctx.fillText(`${points[idx].time.toFixed(0)}s`, x, H - 4)
  }
}

/** 双条历史对比：两条压测场均耗时归一化柱状图。返回每条的可读标题。 */
export function drawCompareChart(
  canvas: HTMLCanvasElement,
  runs: Array<{ id: number; stats_json: string }>,
): string[] {
  const ctx = canvas.getContext('2d')
  if (!ctx) return []
  const titles: string[] = runs.map((r) => {
    try { return `场均 ${(JSON.parse(r.stats_json) as StressStats).avg_ms.toFixed(1)}ms` } catch { return '—' }
  })

  const vals = runs.map((r) => {
    try { return (JSON.parse(r.stats_json) as StressStats).avg_ms || 0 } catch { return 0 }
  })
  const max = Math.max(...vals, 1)

  const W = canvas.width
  const H = canvas.height
  const PAD = { top: 10, right: 16, bottom: 20, left: 48 }
  ctx.clearRect(0, 0, W, H)
  const bg = readToken('--bg-surface', '#fafafa')
  ctx.fillStyle = bg
  ctx.fillRect(0, 0, W, H)

  const colors = [readToken('--color-success', '#18a058'), readToken('--color-info', '#2080f0')]
  runs.forEach((_, i) => {
    if (i >= 2) return
    const v = vals[i]
    const barH = (v / max) * (H - PAD.top - PAD.bottom)
    const slotW = (W - PAD.left - PAD.right) / runs.length
    const x = PAD.left + slotW * i + 12
    const bw = slotW - 24
    ctx.fillStyle = colors[i]
    ctx.fillRect(x, H - PAD.bottom - barH, bw, barH)
    ctx.fillStyle = readToken('--text-secondary', '#666')
    ctx.font = '11px sans-serif'
    ctx.fillText(v.toFixed(0) + 'ms', x, H - PAD.bottom - barH - 4)
    ctx.fillText(`轮${i + 1}`, x + bw / 2 - 8, H - 4)
  })
  return titles
}