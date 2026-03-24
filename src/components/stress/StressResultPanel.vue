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
        <div class="stat-value">{{ stressStore.stats.p50_ms }}ms</div>
        <div class="stat-label">P50</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.p95_ms }}ms</div>
        <div class="stat-label">P95</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ stressStore.stats.p99_ms }}ms</div>
        <div class="stat-label">P99</div>
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

    <template #footer>
      <n-button @click="handleClose" :disabled="stressStore.isRunning">关闭</n-button>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, watch, onUnmounted, nextTick } from 'vue'
import { NModal, NButton, NSpin } from 'naive-ui'
import { useStressStore } from '../../stores/stress'

const show = defineModel<boolean>('show', { required: true })
const stressStore = useStressStore()
const canvasRef = ref<HTMLCanvasElement | null>(null)

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

  // 背景
  ctx.fillStyle = '#fafafa'
  ctx.fillRect(0, 0, W, H)

  if (points.length < 2) {
    ctx.fillStyle = '#999'
    ctx.font = '12px sans-serif'
    ctx.textAlign = 'center'
    ctx.fillText('等待数据...', W / 2, H / 2)
    return
  }

  // 计算 Y 轴最大值（TPS 和 ms 都画在同一 Y 轴，用归一化）
  const maxTps = Math.max(...points.map(p => p.tps), 1)
  const maxMs = Math.max(...points.map(p => p.p95_ms), 1)

  // 网格线
  ctx.strokeStyle = '#e8e8e8'
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

  drawLine(points.map(p => p.tps), maxTps, '#18a058', 2)      // TPS — 绿色
  drawLine(points.map(p => p.avg_ms), maxMs, '#2080f0', 1.5)  // avg — 蓝色
  drawLine(points.map(p => p.p95_ms), maxMs, '#f0a020', 1.5)  // p95 — 橙色

  // X 轴时间标签
  ctx.fillStyle = '#999'
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
  if (v) nextTick(drawChart)
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
  background: var(--n-color-modal, #f6f6f6);
  border-radius: 6px;
  padding: 10px 12px;
  text-align: center;
}
.stat-card.success .stat-value { color: #18a058; }
.stat-card.fail .stat-value { color: #d03050; }

.stat-value {
  font-size: 20px;
  font-weight: 600;
  line-height: 1.2;
}
.stat-label {
  font-size: 11px;
  color: var(--n-text-color-3, #999);
  margin-top: 2px;
}

.chart-area {
  border: 1px solid var(--n-border-color, #e0e0e6);
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
  background: var(--n-color-modal, #f6f6f6);
  border-top: 1px solid var(--n-border-color, #e0e0e6);
}
.tps-color { color: #18a058; }
.avg-color { color: #2080f0; }
.p95-color { color: #f0a020; }

.status-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--n-text-color-3, #999);
  padding: 4px 0;
}
.status-bar.done { color: #18a058; }
.status-bar.error { color: #d03050; }
</style>
