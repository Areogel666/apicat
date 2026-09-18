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
        <div class="stat-label">响应率</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ bizRateText }}</div>
        <div class="stat-label">业务成功率</div>
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
              width: pct(cnt, stressStore.stats?.status_counts) + '%',
              background: statusColor(code),
            }"></div>
          </div>
          <span class="status-count">{{ cnt }}</span>
          <span class="status-hint">{{ statusHint(code) }}</span>
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
          <n-button size="tiny" quaternary @click.stop="openReport(run)">📄 报告</n-button>
          <n-button size="tiny" quaternary title="删除" @click.stop="stressStore.removeRun(run.id)">✕</n-button>
        </div>
      </div>
      <div class="history-compare-bar">
        <n-button size="tiny" secondary :disabled="compareSelection.length < 2" @click="drawCompare">📊 对比选中（{{ compareSelection.length }}/2）</n-button>
        <n-button v-if="compareSelection.length" size="tiny" quaternary @click="compareSelection = []">清除</n-button>
      </div>
    </div>

    <!-- 1.0.5：多指标分组对比图 + 对比表 -->
    <div v-if="compareCanvasVisible" class="compare-area">
      <canvas ref="compareCanvasRef" height="200" class="stress-canvas compare-canvas" />
      <div class="chart-legend">
        <span
          v-for="(run, i) in compareRuns"
          :key="run.id"
          class="legend-item"
          :style="{ color: compareColors[i] }"
        >
          ▮ 轮{{ i + 1 }} · {{ formatTime(run.created_at) }}
        </span>
      </div>
      <div class="compare-note">
        每组柱子按该指标自身的最大值归一化（组间高度不可直接比较），柱顶数字为实际值。
      </div>
      <table class="compare-table">
        <thead>
          <tr><th>指标</th><th>轮1</th><th>轮2</th><th>变化</th></tr>
        </thead>
        <tbody>
          <tr v-for="m in compareMetrics" :key="m.label">
            <td>{{ m.label }}</td>
            <td>{{ formatMetric(m.v1, m.unit) }}</td>
            <td>{{ formatMetric(m.v2, m.unit) }}</td>
            <td :class="deltaClass(m)">
              {{ m.deltaPct == null ? '—' : `${m.deltaPct >= 0 ? '+' : ''}${m.deltaPct.toFixed(1)}%` }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <template #footer>
      <n-button @click="handleClose" :disabled="stressStore.isRunning">关闭</n-button>
    </template>
  </n-modal>

  <StressReportModal v-model:show="showReport" :run="reportRun" />
</template>

<script setup lang="ts">
import { ref, computed, watch, onUnmounted, nextTick } from 'vue'
import { NModal, NButton, NSpin } from 'naive-ui'
import { useStressStore } from '../../stores/stress'
import { useThemeStore } from '../../stores/theme'
import { useRequestStore } from '../../stores/request'
import StressReportModal from './StressReportModal.vue'
import {
  LATENCY_LABELS,
  histHeight,
  statusColor,
  pct,
  summarizeStats,
  formatTime,
  drawCompareChart,
  computeCompareMetrics,
  formatMetric,
  deltaClass,
  type CompareMetric,
  type CompareRun,
} from './stressUtils'
import { statusHint } from './stressReport'
import type { StressRun } from '../../types'

const show = defineModel<boolean>('show', { required: true })
const stressStore = useStressStore()
const themeStore = useThemeStore()
const requestStore = useRequestStore()
const canvasRef = ref<HTMLCanvasElement | null>(null)

const latencyBuckets = computed(() => {
  const hist = stressStore.stats?.latency_hist ?? []
  const total = hist.reduce((s, n) => s + n, 0) || 1
  return LATENCY_LABELS.map((label, i) => ({
    label,
    count: hist[i] ?? 0,
    pct: (hist[i] ?? 0) / total * 100,
  }))
})

/** 业务成功率：旧记录未采集，显示 — 而不是编造 0% */
const bizRateText = computed(() => {
  const r = stressStore.stats?.biz_success_rate
  return r == null ? '—' : `${r.toFixed(1)}%`
})

// 1.0.4：历史记录状态 —— 对比集（最多 2 条）
const compareSelection = ref<number[]>([])
const compareCanvasRef = ref<HTMLCanvasElement | null>(null)
const compareCanvasVisible = ref(false)
const compareRuns = ref<CompareRun[]>([])
const compareMetrics = ref<CompareMetric[]>([])

/** 主题切换后重新读 token（原先用硬编码色种子，不跟主题） */
const compareColors = computed(() => {
  void themeStore.effectiveMode
  return [readToken('--color-success', '#18a058'), readToken('--color-info', '#2080f0')]
})

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

// ── 报告预览（1.0.5：内容由 Rust 生成，与技能走 bridge 拿到的完全同源） ──
const showReport = ref(false)
const reportRun = ref<StressRun | null>(null)

function openReport(run: StressRun) {
  reportRun.value = run
  showReport.value = true
}

/** 1.0.5：多指标分组对比。原实现在置 visible 后立刻绘制，canvas 尚未布局，
 *  prepareCanvas 取到的 clientWidth 会是 0 —— 补 nextTick 是本次修复的一部分。 */
async function drawCompare() {
  const runs = compareSelection.value
    .map(id => stressStore.history.find(r => r.id === id))
    .filter((r): r is StressRun => Boolean(r))
  if (runs.length < 2) return
  compareRuns.value = runs
  compareMetrics.value = computeCompareMetrics(runs)
  compareCanvasVisible.value = true
  await nextTick()
  if (compareCanvasRef.value) drawCompareChart(compareCanvasRef.value, compareMetrics.value)
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

/* 高度由容器分别指定，避免属性尺寸与 CSS 不一致导致拉伸（prepareCanvas 以 CSS 为准） */
.stress-canvas {
  display: block;
  width: 100%;
}
.chart-area .stress-canvas { height: 180px; }
.compare-canvas { height: 200px; }

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
.status-hint { width: 132px; font-size: var(--font-size-sm); color: var(--text-tertiary); flex-shrink: 0; }

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
.compare-note {
  padding: var(--spacing-xs) var(--spacing-sm);
  font-size: var(--font-size-sm);
  color: var(--text-tertiary);
  border-top: 1px solid var(--border-base);
}
.compare-table { border-collapse: collapse; width: 100%; font-size: var(--font-size-sm); }
.compare-table th, .compare-table td {
  border-top: 1px solid var(--border-base);
  padding: 4px 10px;
  text-align: left;
}
.compare-table th { color: var(--text-tertiary); font-weight: 600; }
.delta-better { color: var(--color-success); }
.delta-worse { color: var(--color-error); }
.delta-flat { color: var(--text-tertiary); }
</style>
