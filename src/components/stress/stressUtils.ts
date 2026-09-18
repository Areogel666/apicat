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

/**
 * 让 canvas 的绘图坐标系与 CSS 尺寸一致，并按 devicePixelRatio 提采样。
 *
 * 解决两个问题：
 *   1. 属性 width/height 与 CSS 尺寸不一致导致的图形拉伸（对比图原先 140 vs 150px）
 *   2. 画布宽度写死 620px 而容器更宽时的横向变形
 * 调用方必须用返回的 W/H 做绘制计算，不要再用 canvas.width / canvas.height。
 */
export function prepareCanvas(
  canvas: HTMLCanvasElement,
): { ctx: CanvasRenderingContext2D; W: number; H: number } | null {
  const ctx = canvas.getContext('2d')
  if (!ctx) return null
  const W = canvas.clientWidth || canvas.width
  const H = canvas.clientHeight || canvas.height
  const dpr = window.devicePixelRatio || 1
  const pw = Math.round(W * dpr)
  const ph = Math.round(H * dpr)
  if (canvas.width !== pw || canvas.height !== ph) {
    canvas.width = pw
    canvas.height = ph
  }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  return { ctx, W, H }
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

// ── 两次压测对比（1.0.5：多指标分组双柱 + 对比表）────────────────

export interface CompareRun {
  id: number
  stats_json: string
  created_at: string
}

export interface CompareMetric {
  label: string
  unit: string
  /** 该指标是否「越大越好」——决定变化值的着色 */
  higherIsBetter: boolean
  v1: number
  v2: number
  /** 轮2 相对轮1 的变化百分比；轮1 为 0 时无法计算，返回 null */
  deltaPct: number | null
}

/** 对比指标定义。每项按自身最大值独立归一化，因此组间柱高不可直接比较 */
const COMPARE_METRICS: Array<{
  label: string
  unit: string
  higherIsBetter: boolean
  pick: (st: StressStats) => number
}> = [
  { label: 'TPS',        unit: '',   higherIsBetter: true,  pick: st => st.tps },
  { label: '响应率',      unit: '%',  higherIsBetter: true,  pick: st => st.success_rate },
  { label: '业务成功率',  unit: '%',  higherIsBetter: true,  pick: st => st.biz_success_rate ?? 0 },
  { label: '平均耗时',    unit: 'ms', higherIsBetter: false, pick: st => st.avg_ms },
  { label: 'P95',        unit: 'ms', higherIsBetter: false, pick: st => st.p95_ms },
  { label: 'P99',        unit: 'ms', higherIsBetter: false, pick: st => st.p99_ms },
]

export function formatMetric(v: number, unit: string): string {
  return unit === 'ms' ? `${v.toFixed(0)}ms` : `${v.toFixed(1)}${unit}`
}

function parseStatsOf(run: CompareRun): StressStats | null {
  try { return JSON.parse(run.stats_json) as StressStats } catch { return null }
}

/** 算出两条记录的逐指标对比。任一条解析失败或不足两条时返回空数组 */
export function computeCompareMetrics(runs: CompareRun[]): CompareMetric[] {
  const stats = runs.slice(0, 2).map(parseStatsOf)
  if (stats.length < 2) return []
  const [a, b] = stats
  if (!a || !b) return []
  return COMPARE_METRICS.map(m => {
    const v1 = m.pick(a)
    const v2 = m.pick(b)
    return {
      label: m.label,
      unit: m.unit,
      higherIsBetter: m.higherIsBetter,
      v1,
      v2,
      deltaPct: v1 === 0 ? null : (v2 - v1) / v1 * 100,
    }
  })
}

/** 变化值的着色方向：变好绿、变差红、几乎不变灰 */
export function deltaClass(m: CompareMetric): 'delta-better' | 'delta-worse' | 'delta-flat' {
  if (m.deltaPct == null || Math.abs(m.deltaPct) < 0.05) return 'delta-flat'
  const improved = m.higherIsBetter ? m.deltaPct > 0 : m.deltaPct < 0
  return improved ? 'delta-better' : 'delta-worse'
}

/** 多指标分组双柱对比图。每组独立归一化，柱顶标实际值 */
export function drawCompareChart(canvas: HTMLCanvasElement, metrics: CompareMetric[]): void {
  const prep = prepareCanvas(canvas)
  if (!prep) return
  const { ctx, W, H } = prep

  const bg = readToken('--bg-surface', '#fafafa')
  const textTertiary = readToken('--text-tertiary', '#999')
  const textSecondary = readToken('--text-secondary', '#666')
  const borderColor = readToken('--border-base', '#e8e8e8')
  const colors = [readToken('--color-success', '#18a058'), readToken('--color-info', '#2080f0')]

  ctx.clearRect(0, 0, W, H)
  ctx.fillStyle = bg
  ctx.fillRect(0, 0, W, H)

  if (!metrics.length) {
    ctx.fillStyle = textTertiary
    ctx.font = '12px sans-serif'
    ctx.textAlign = 'center'
    ctx.fillText('勾选两条历史记录后点「对比」', W / 2, H / 2)
    return
  }

  const PAD = { top: 26, right: 12, bottom: 42, left: 12 }
  const innerW = W - PAD.left - PAD.right
  const innerH = H - PAD.top - PAD.bottom
  const groupW = innerW / metrics.length
  const barW = Math.max(8, Math.min(26, groupW / 2 - 8))
  const gap = 6

  // 基线
  ctx.strokeStyle = borderColor
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(PAD.left, H - PAD.bottom)
  ctx.lineTo(W - PAD.right, H - PAD.bottom)
  ctx.stroke()

  metrics.forEach((m, gi) => {
    const cx = PAD.left + groupW * gi + groupW / 2
    // 每组按自身最大值归一化，避免「TPS 数值远大于百分比」把其它组压成一条线
    const max = Math.max(m.v1, m.v2, 1e-9)
    const bars: Array<[number, number, string]> = [
      [cx - barW - gap / 2, m.v1, colors[0]],
      [cx + gap / 2, m.v2, colors[1]],
    ]

    bars.forEach(([x, v, color]) => {
      const bh = Math.max(2, (v / max) * innerH)
      ctx.fillStyle = color
      ctx.fillRect(x, H - PAD.bottom - bh, barW, bh)
      ctx.fillStyle = textSecondary
      ctx.font = '10px sans-serif'
      ctx.textAlign = 'center'
      ctx.fillText(formatMetric(v, m.unit), x + barW / 2, H - PAD.bottom - bh - 4)
    })

    ctx.fillStyle = textTertiary
    ctx.font = '11px sans-serif'
    ctx.textAlign = 'center'
    ctx.fillText(m.label, cx, H - PAD.bottom + 16)
  })
}