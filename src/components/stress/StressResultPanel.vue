<template>
  <n-modal
    v-model:show="show"
    preset="card"
    title="压测结果"
    style="width: 680px"
    :closable="!stressStore.isRunning"
    :mask-closable="!stressStore.isRunning"
  >
    <!-- 统计数字卡片 -->
    <div class="stats-grid" v-if="stressStore.stats">
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.total }}</div>
        <div class="stat-label">总请求</div>
      </div>
      <div class="stat-card success">
        <div class="stat-value">{{ stressStore.stats.success }}</div>
        <div class="stat-label">成功</div>
      </div>
      <div class="stat-card fail">
        <div class="stat-value">{{ stressStore.stats.failed }}</div>
        <div class="stat-label">失败</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.success_rate.toFixed(1) }}%</div>
        <div class="stat-label">成功率</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.tps.toFixed(1) }}</div>
        <div class="stat-label">TPS</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.avg_ms.toFixed(0) }}ms</div>
        <div class="stat-label">平均耗时</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.min_ms }}ms</div>
        <div class="stat-label">最小耗时</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.p50_ms }}ms</div>
        <div class="stat-label">P50</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.p90_ms }}ms</div>
        <div class="stat-label">P90</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.p95_ms }}ms</div>
        <div class="stat-label">P95</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.p99_ms }}ms</div>
        <div class="stat-label">P99</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.max_ms }}ms</div>
        <div class="stat-label">最大耗时</div>
      </div>
    </div>

    <!-- 实时折线图（canvas）-->
    <div class="chart-area">
      <canvas ref="canvasRef" width="620" height="180" class="stress-canvas" />
      <div class="chart-legend">
        <span class="legend-item tps-color">▬ TPS</span>
        <span class="legend-item avg-color">▬ 平均耗时(ms)</span>
        <span class="legend-item p95-color">▬ P95(ms)</span>
      </div>
    </div>

    <!-- 1.0.4：耗时分布直方图 + 状态码分布 -->
    <div v-if="stressStore.stats?.done" class="done-details">
      <div class="stress-section-title">耗时分布（ms）</div>
      <div class="hist-bar">
        <div v-for="(bucket, i) in latencyBuckets" :key="i" class="hist-col">
          <div class="hist-col__bar" :style="{
            height: histHeight(bucket) + 'px',
            background: readToken('--color-primary', '#18a058'),
          }"></div>
          <span class="hist-col__label">{{ bucket.label }}</span>
        </div>
      </div>

      <div class="stress-section-title">状态码分布</div>
      <div v-if="!stressStore.stats?.status_counts?.length" class="hist-empty">暂无统计</div>
      <div v-else class="status-dist">
        <div v-for="[code, cnt] in stressStore.stats.status_counts" :key="code" class="status-row">
          <span class="status-code" :style="{ color: statusColor(code) }">
            {{ code === 0 ? '网络错误' : code }}
          </span>
          <div class="status-track">
            <div class="status-fill" :style="{
              width: pct(cnt) + '%',
              background: statusColor(code),
            }"></div>
          </div>
          <span class="status-count">{{ cnt }}</span>
        </div>
      </div>
    </div>

    <!-- 进行中提示 / 完成提示 -->
    <div class="status-bar" v-if="stressStore.isRunning">
      <n-spin size="small" />
      <span>压测进行中... {{ stressStore.stats?.elapsed_sec.toFixed(1) }}s</span>
    </div>
    <div class="status-bar done" v-else-if="stressStore.stats?.done">
      ✅ 压测完成，耗时 {{ stressStore.stats?.elapsed_sec.toFixed(2) }}s
    </div>
    <div class="status-bar error" v-if="stressStore.error">
      ❌ {{ stressStore.error }}
    </div>

    <!-- 1.0.4：压测历史记录 -->
    <div class="stress-section-title">历史记录（勾选两条后点「对比」）</div>
    <div v-if="!stressStore.history.length" class="hist-empty">暂无历史</div>
    <div v-else class="history-list">
      <div
        v-for="run in stressStore.history"
        :key="run.id"
        class="history-item"
        :class="{ active: compareSelection.includes(run.id) }"
        @click="toggleCompare(run.id)"
      >
        <div class="history-item__main">
          <span class="history-item__time">{{ formatTime(run.created_at) }}</span>
          <span class="history-item__meta">{{ summarizeStats(run) }}</span>
        </div>
        <div class="history-item__actions">
          <n-button size="tiny" quaternary @click.stop="onExportReport(run)">📄 报告</n-button>
          <n-button size="tiny" quaternary title="删除" @click.stop="stressStore.removeRun(run.id)">✕</n-button>
        </div>
      </div>
      <div class="history-compare-bar">
        <n-button size="tiny" secondary :disabled="compareSelection.length < 2" @click="drawCompare">📊 对比选中（{{ compareSelection.length }}/2）</n-button>
        <n-button v-if="compareSelection.length" size="tiny" quaternary @click="compareSelection = []">清除</n-button>
      </div>
    </div>

    <!-- 1.0.4：双条对比折线 -->
    <div v-if="compareCanvasVisible" class="compare-area">
      <canvas ref="compareCanvasRef" width="620" height="160" class="stress-canvas" />
      <div class="chart-legend">
        <span v-for="(seed, i) in ['#18a058', '#2080f0']" :key="i" class="legend-item" :style="{ color: seed }">
          ▬ {{ compareLegend[i] || '—' }}
        </span>
      </div>
    </div>

    <template #footer>
      <n-button @click="handleClose" :disabled="stressStore.isRunning">关闭</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, h, computed, watch, onUnmounted, nextTick } from 'vue'
import { NModal, NButton, NSpin, useDialog } from 'naive-ui'
import { useStressStore } from '../../stores/stress'
import { useThemeStore } from '../../stores/theme'
import { useRequestStore } from '../../stores/request'
import type { StressStats } from '../../types'

const show = defineModel<boolean>('show', { required: true })
const stressStore = useStressStore()
const themeStore = useThemeStore()
const requestStore = useRequestStore()
const dialog = useDialog()
const canvasRef = ref<HTMLCanvasElement | null>(null)

// 1.0.4：耗时直方图桶标签（与 Rust LATENCY_BUCKETS 一致：10 桶）
const LATENCY_LABELS = ['<1', '1-2', '2-5', '5-10', '10-20', '20-50', '50-100', '100-200', '200-500', '>500']
const histMaxBucket = 40

const latencyBuckets = computed(() => {
  const hist = stressStore.stats?.latency_hist ?? []
  const total = hist.reduce((s, n) => s + n, 0) || 1
  return LATENCY_LABELS.map((label, i) => ({
    label,
    count: hist[i] ?? 0,
    pct: (hist[i] ?? 0) / total * 100,
  }))
})

function histHeight(bucket: { pct: number }): number {
  return Math.max(2, Math.round(bucket.pct / 100 * histMaxBucket))
}

/** 状态码 → 颜色 token */
function statusColor(code: number): string {
  if (code === 0) return readToken('--status-network', '#909399')
  if (code >= 200 && code < 300) return readToken('--status-2xx', '#18a058')
  if (code >= 300 && code < 400) return readToken('--status-3xx', '#2080f0')
  if (code >= 400 && code < 500) return readToken('--status-4xx', '#f0a020')
  return readToken('--status-5xx', '#d03050')
}

function pct(cnt: number): number {
  const total = (stressStore.stats?.status_counts ?? []).reduce((s, [_, c]) => s + c, 0)
  if (!total) return 0
  return Math.round(cnt / total * 100)
}

// 1.0.4：历史记录状态 —— 对比集（最多 2 条）
const compareSelection = ref<number[]>([])
const compareCanvasRef = ref<HTMLCanvasElement | null>(null)
const compareCanvasVisible = ref(false)
const compareLegend = ref<string[]>([])

function toggleCompare(id: number) {
  const i = compareSelection.value.indexOf(id)
  if (i >= 0) {
    compareSelection.value.splice(i, 1)
  } else if (compareSelection.value.length < 2) {
    compareSelection.value.push(id)
  } else {
    // 已有 2 条：替换第一条
    compareSelection.value = [compareSelection.value[1], id]
  }
}

function buildReport(run: { config_json: string; stats_json: string; created_at: string }): string {
  let cfg: { concurrent?: number; mode?: string; value?: number } = {}
  let st: StressStats | null = null
  try { cfg = JSON.parse(run.config_json) } catch {}
  try { st = JSON.parse(run.stats_json) } catch {}
  if (!st) return '（无统计数据）'
  const hist = (st.latency_hist ?? []).map((n, i) => `${LATENCY_LABELS[i]}ms:${n}`).join(' / ')
  const status = (st.status_counts ?? []).map(([c, n]) => `${c === 0 ? '网络错误' : c}:${n}`).join(' / ')
  return [
    `# ApiCat 压测报告`,
    `- 时间：${run.created_at}`,
    `- 并发=${cfg.concurrent} 模式=${cfg.mode} 值=${cfg.value}`,
    `- 总请求 ${st.total} 成功 ${st.success} 失败 ${st.failed} 成功率 ${st.success_rate.toFixed(1)}%`,
    `- 耗时 min ${st.min_ms ?? 0}ms / avg ${(st.avg_ms ?? 0).toFixed(1)}ms / P50 ${st.p50_ms ?? 0} / P90 ${st.p90_ms ?? 0} / P95 ${st.p95_ms ?? 0} / P99 ${st.p99_ms ?? 0} / max ${st.max_ms ?? 0}ms`,
    `- TPS ${st.tps.toFixed(1)}`,
    `## 耗时分布`,
    hist,
    `## 状态码分布`,
    status || '（无）',
  ].join('\n')
}

function onExportReport(run: { config_json: string; stats_json: string; created_at: string }) {
  const text = buildReport(run)
  dialog.success({
    title: '压测报告（Markdown）',
    style: 'width: 560px',
    content: () => h('pre', {
      style: 'max-height:360px;overflow:auto;font-family:monospace;font-size:12px;white-space:pre-wrap;color:var(--text-primary)',
    }, text),
    action: () => {
      void navigator.clipboard.writeText(text).catch(() => {})
      void saveReportToFile(text)
      return '已复制'
    },
  })
}

/** 1.0.4：报告存本地（Tauri dialog + fs，与 ExportDialog 同模式） */
async function saveReportToFile(text: string) {
  try {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const { writeTextFile } = await import('@tauri-apps/plugin-fs')
    const path = await save({
      title: '保存压测报告',
      defaultPath: `stress-report-${Date.now()}.md`,
      filters: [{ name: 'Markdown', extensions: ['md'] }],
    })
    if (path) await writeTextFile(path, text)
  } catch (e) {
    console.warn('[stress] 保存报告失败:', e)
  }
}

/** 1.0.4：叠绘两条历史折线（TPS/耗时 归一化双系列） */
function drawCompare() {
  compareLegend.value = []
  compareCanvasVisible.value = true
  const canvas = compareCanvasRef.value
  if (!canvas) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const runs = compareSelection.value
    .map(id => stressStore.history.find(r => r.id === id))
    .filter((r): r is NonNullable<typeof r> => Boolean(r))
  if (runs.length < 2) return

  // 归一化两条耗时（avg 逐轮对比），画两条横线高度对比
  const vals = runs.map((r) => {
    try { return (JSON.parse(r.stats_json) as StressStats).avg_ms || 0 } catch { return 0 }
  })
  compareLegend.value = runs.map((_, i) => `轮${i + 1} 场均 ${vals[i].toFixed(1)}ms`)
  const max = Math.max(...vals, 1)

  const W = canvas.width
  const H = canvas.height
  const PAD = { top: 10, right: 16, bottom: 20, left: 48 }
  ctx.clearRect(0, 0, W, H)
  const bg = readToken('--bg-surface', '#fafafa')
  ctx.fillStyle = bg
  ctx.fillRect(0, 0, W, H)

  const colors = [readToken('--color-success', '#18a058'), readToken('--color-info', '#2080f0')]
  if (runs.length >= 1) {
    const v0 = vals[0]
    const barH0 = (v0 / max) * (H - PAD.top - PAD.bottom)
    const x0 = PAD.left + 12
    const bw0 = (W - PAD.left - PAD.right) / runs.length - 24
    ctx.fillStyle = colors[0]
    ctx.fillRect(x0, H - PAD.bottom - barH0, bw0, barH0)
    ctx.fillStyle = readToken('--text-secondary', '#666')
    ctx.font = '11px sans-serif'
    ctx.fillText(v0.toFixed(0) + 'ms', x0, H - PAD.bottom - barH0 - 4)
    ctx.fillText('轮1', x0 + bw0 / 2 - 8, H - 4)
  }
  if (runs.length >= 2) {
    const v1 = vals[1]
    const barH1 = (v1 / max) * (H - PAD.top - PAD.bottom)
    const x1 = PAD.left + (W - PAD.left - PAD.right) / runs.length + 12
    const bw1 = (W - PAD.left - PAD.right) / runs.length - 24
    ctx.fillStyle = colors[1]
    ctx.fillRect(x1, H - PAD.bottom - barH1, bw1, barH1)
    ctx.fillStyle = readToken('--text-secondary', '#666')
    ctx.font = '11px sans-serif'
    ctx.fillText(v1.toFixed(0) + 'ms', x1, H - PAD.bottom - barH1 - 4)
    ctx.fillText('轮2', x1 + bw1 / 2 - 8, H - 4)
  }
}

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

function summarizeStats(run: { config_json: string; stats_json: string }): string {
  let st: StressStats | null = null
  try { st = JSON.parse(run.stats_json) } catch {}
  if (!st) return ''
  return `总${st.total} 成功率${st.success_rate.toFixed(1)}% TPS${st.tps.toFixed(1)} P95${st.p95_ms}ms`
}

/** 从 :root CSS 变量读取色值，用于 canvas 绘制时跟随主题 */
function readToken(name: string, fallback: string): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return v || fallback
}

// ── Canvas 折线图绘制 ────────────────────────────────────────

function drawChart() {
  const canvas = canvasRef.value
  if (!canvas) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const points = stressStore.chartPoints
  const W = canvas.width
  const H = canvas.height
  const PAD = { top: 16, right: 16, bottom: 24, left: 48 }
  const innerW = W - PAD.left - PAD.right
  const innerH = H - PAD.top - PAD.bottom

  ctx.clearRect(0, 0, W, H)

  // 从 token 读取主题相关色值（每次 draw 重新读，主题切换后调用 drawChart 即跟随）
  const bgColor      = readToken('--bg-surface',     '#fafafa')
  const textTertiary = readToken('--text-tertiary',  '#999')
  const borderColor  = readToken('--border-base',    '#e8e8e8')
  const tpsColor     = readToken('--color-success',  '#18a058')
  const avgColor     = readToken('--color-info',     '#2080f0')
  const p95Color     = readToken('--color-warning',  '#f0a020')

  // 背景
  ctx.fillStyle = bgColor
  ctx.fillRect(0, 0, W, H)

  if (points.length < 2) {
    ctx.fillStyle = textTertiary
    ctx.font = '12px sans-serif'
    ctx.textAlign = 'center'
    ctx.fillText('等待数据...', W / 2, H / 2)
    return
  }

  // 计算 Y 轴最大值（TPS 和 ms 都画在同一 Y 轴，用归一化）
  const maxTps = Math.max(...points.map(p => p.tps), 1)
  const maxMs = Math.max(...points.map(p => p.p95_ms), 1)

  // 网格线
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

  // 绘制折线函数
  function drawLine(
    values: number[],
    maxVal: number,
    color: string,
    lineWidth = 1.5
  ) {
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

  drawLine(points.map(p => p.tps), maxTps, tpsColor, 2)      // TPS — 绿色
  drawLine(points.map(p => p.avg_ms), maxMs, avgColor, 1.5)  // avg — 蓝色
  drawLine(points.map(p => p.p95_ms), maxMs, p95Color, 1.5)  // p95 — 橙色

  // X 轴时间标签
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

// 监听 chartPoints 变化，重新绘制
watch(
  () => stressStore.chartPoints.length,
  () => nextTick(drawChart)
)

// 弹窗打开时初始化 canvas
watch(show, (v) => {
  if (v) {
    // 1.0.4：打开面板时加载该接口压测历史
    if (requestStore.activeRequestId != null) {
      stressStore.loadHistory(requestStore.activeRequestId)
    }
    nextTick(drawChart)
  }
})

// 主题切换 → 重绘 canvas（CSS 变量已变，需要重新读取）
watch(() => themeStore.effectiveMode, () => {
  if (show.value) nextTick(drawChart)
})

function handleClose() {
  stressStore.reset()
  show.value = false
}

onUnmounted(() => {
  stressStore.cleanup()
})
</script>

<style scoped>
.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  margin-bottom: 16px;
}

.stat-card {
  background: var(--bg-elevated);
  border-radius: 6px;
  padding: 10px 12px;
  text-align: center;
}
.stat-card.success .stat-value { color: var(--color-success); }
.stat-card.fail .stat-value { color: var(--color-error); }

.stat-value {
  font-size: 20px;
  font-weight: 600;
  line-height: 1.2;
}
.stat-label {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

.chart-area {
  border: 1px solid var(--border-base);
  border-radius: 6px;
  overflow: hidden;
  margin-bottom: 12px;
}

.stress-canvas {
  display: block;
  width: 100%;
  height: 180px;
}

.chart-legend {
  display: flex;
  gap: 16px;
  padding: 6px 12px;
  font-size: 11px;
  background: var(--bg-elevated);
  border-top: 1px solid var(--border-base);
}
.tps-color { color: var(--color-success); }
.avg-color { color: var(--color-info); }
.p95-color { color: var(--color-warning); }

.status-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-tertiary);
  padding: 4px 0;
}
.status-bar.done { color: var(--color-success); }
.status-bar.error { color: var(--color-error); }

/* 1.0.4：耗时直方图 + 状态码分布 */
.stress-section-title {
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-tertiary);
  margin: var(--spacing-md) 0 var(--spacing-xs);
}
.hist-bar {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 60px;
  margin-bottom: var(--spacing-md);
}
.hist-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-end;
  height: 100%;
  gap: 2px;
}
.hist-col__bar {
  width: 100%;
}
.hist-col__label {
  font-size: 10px;
  color: var(--text-tertiary);
  white-space: nowrap;
}
.hist-empty { color: var(--text-tertiary); font-size: var(--font-size-sm); padding: var(--spacing-sm) 0; }
.status-dist { display: flex; flex-direction: column; gap: var(--spacing-xs); }
.status-row { display: flex; align-items: center; gap: var(--spacing-sm); }
.status-code { width: 64px; font-size: var(--font-size-sm); flex-shrink: 0; }
.status-track {
  flex: 1;
  height: 14px;
  background: var(--bg-hover);
  border-radius: var(--radius-sm);
  overflow: hidden;
}
.status-fill { height: 100%; }
.status-count { width: 48px; text-align: right; font-size: var(--font-size-sm); color: var(--text-secondary); }

/* 1.0.4：压测历史 */
.history-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
  max-height: 180px;
  overflow-y: auto;
}
.history-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.history-item:hover { background: var(--bg-hover); }
.history-item.active { border-color: var(--color-primary); background: var(--bg-selected); }
.history-item__main { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.history-item__time { font-size: var(--font-size-sm); color: var(--text-secondary); }
.history-item__meta { font-size: var(--font-size-sm); color: var(--text-tertiary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.history-item__actions { display: flex; align-items: center; flex-shrink: 0; }
.history-compare-bar {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding-top: var(--spacing-xs);
}
.compare-area {
  margin-top: var(--spacing-md);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  overflow: hidden;
}
</style>
