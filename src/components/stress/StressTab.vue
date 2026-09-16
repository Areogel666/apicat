<template>
  <div class="stress-tab">
    <!-- 配置条 -->
    <div class="stress-tab__toolbar">
      <div class="cfg-group">
        <span class="cfg-label">并发</span>
        <n-input-number v-model:value="config.concurrent" :min="1" :max="500" size="small" style="width: 90px" />
      </div>
      <div class="cfg-group">
        <span class="cfg-label">模式</span>
        <n-radio-group v-model:value="config.mode" size="small">
          <n-radio-button value="count">总量</n-radio-button>
          <n-radio-button value="duration">时长</n-radio-button>
        </n-radio-group>
      </div>
      <div class="cfg-group">
        <span class="cfg-label">{{ config.mode === 'count' ? '请求数' : '秒' }}</span>
        <n-input-number
          v-model:value="config.value"
          :min="1"
          :max="config.mode === 'count' ? 10000 : 3600"
          size="small"
          style="width: 90px"
        />
      </div>
      <div class="cfg-group">
        <span class="cfg-label">请求参数</span>
        <n-radio-group v-model:value="paramSource" size="small">
          <n-radio-button value="current">编辑区</n-radio-button>
          <n-radio-button value="testcase">用例</n-radio-button>
        </n-radio-group>
        <n-select
          v-if="paramSource === 'testcase'"
          v-model:value="selectedTestCaseId"
          :options="testCaseOptions"
          size="small"
          style="width: 130px; margin-left: 6px"
          placeholder="选择用例"
        />
      </div>
      <n-button type="primary" size="small" :loading="stressStore.isRunning" :disabled="!canStart"
        @click="onStart">
        ⚡ 开始压测
      </n-button>
    </div>

    <!-- 实时统计卡片 -->
    <div v-if="stressStore.stats" class="stats-bar">
      <div class="stat-cell">
        <span class="stat-label">总请求</span>
        <span class="stat-value">{{ stressStore.stats.total }}</span>
      </div>
      <div class="stat-cell">
        <span class="stat-label">成功率</span>
        <span class="stat-value">{{ stressStore.stats.success_rate.toFixed(1) }}%</span>
      </div>
      <div class="stat-cell">
        <span class="stat-label">TPS</span>
        <span class="stat-value">{{ stressStore.stats.tps.toFixed(1) }}</span>
      </div>
      <div class="stat-cell">
        <span class="stat-label">avg</span>
        <span class="stat-value">{{ stressStore.stats.avg_ms.toFixed(1) }}ms</span>
      </div>
      <div class="stat-cell">
        <span class="stat-label">P50/P90/P95/P99</span>
        <span class="stat-value">{{ stressStore.stats.p50_ms }}/{{ stressStore.stats.p90_ms }}/{{ stressStore.stats.p95_ms }}/{{ stressStore.stats.p99_ms }}</span>
      </div>
      <div class="stat-cell">
        <span class="stat-label">max</span>
        <span class="stat-value">{{ stressStore.stats.max_ms }}ms</span>
      </div>
      <div v-if="stressStore.stats.done" class="stat-cell stat-done">✅ 完成</div>
      <div v-else-if="stressStore.isRunning" class="stat-cell stat-running">
        <n-spin size="small" /> 进行中 {{ stressStore.stats?.elapsed_sec.toFixed(1) }}s
      </div>
    </div>
    <div v-if="stressStore.error" class="stress-error">❌ {{ stressStore.error }}</div>

    <!-- 实时折线图 -->
    <div v-if="stressStore.chartPoints.length > 1" class="chart-box">
      <canvas ref="chartCanvas" width="620" height="150" class="stress-canvas" />
      <div class="chart-legend">
        <span class="legend-tps">▬ TPS</span>
        <span class="legend-avg">▬ 平均耗时(ms)</span>
        <span class="legend-p95">▬ P95(ms)</span>
      </div>
    </div>

    <!-- 历史 -->
    <div class="history-block">
      <div class="history-head">
        <span class="section-title">压测历史（本接口）</span>
        <div class="history-actions">
          <n-button size="tiny" secondary :disabled="compareSelection.length < 2" @click="doCompare">📊 对比（{{ compareSelection.length }}/2）</n-button>
          <n-button v-if="compareSelection.length" size="tiny" quaternary @click="compareSelection = []">清除</n-button>
        </div>
      </div>
      <n-empty v-if="!stressStore.history.length" description="暂无压测历史，点击上方「开始压测」" size="small" style="padding: 8px 0" />
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
      </div>

      <!-- 双条对比图 -->
      <div v-if="compareVisible" class="compare-box">
        <canvas ref="compareCanvas" width="620" height="140" class="stress-canvas" />
        <div class="chart-legend">
          <span v-for="(t, i) in compareLegend" :key="i" class="legend-item" :style="{ color: compareColors[i] }">
            ▬ 轮{{ i + 1 }} {{ t }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { NButton, NInputNumber, NRadioGroup, NRadioButton, NSelect, NSpin, NEmpty, useMessage } from 'naive-ui'
import { useStressStore } from '../../stores/stress'
import { useRequestStore } from '../../stores/request'
import { useTestCaseStore } from '../../stores/testCase'
import type { StressConfig, StressRun } from '../../types'
import {
  buildReport, saveReportToFile, formatTime, summarizeStats,
  drawStressChart, drawCompareChart, readToken,
} from './stressUtils'

const emit = defineEmits<{
  /** 开始压测：config + 可选选中的用例 id（为 null 用编辑区参数） */
  start: [config: StressConfig, testCaseId: number | null]
}>()

const stressStore = useStressStore()
const requestStore = useRequestStore()
const testCaseStore = useTestCaseStore()
const message = useMessage()

const config = reactive<StressConfig>({ concurrent: 10, mode: 'count', value: 100 })
const paramSource = ref<'current' | 'testcase'>('current')
const selectedTestCaseId = ref<number | null>(null)

const chartCanvas = ref<HTMLCanvasElement | null>(null)
const compareCanvas = ref<HTMLCanvasElement | null>(null)
const compareVisible = ref(false)
const compareSelection = ref<number[]>([])
const compareLegend = ref<string[]>([])
const compareColors = [readToken('--color-success', '#18a058'), readToken('--color-info', '#2080f0')]

const activeRequestId = computed(() => requestStore.activeRequestId)
const canStart = computed(() => requestStore.activeRequest != null && !stressStore.isRunning)

const testCaseOptions = computed(() =>
  testCaseStore.getByRequestId(activeRequestId.value ?? 0).map(tc => ({
    label: tc.name || `用例 #${tc.id}`,
    value: tc.id,
  }))
)

function onStart() {
  if (!canStart.value) return
  const tcId = paramSource.value === 'testcase' ? selectedTestCaseId.value : null
  emit('start', { ...config }, tcId)
}

// ── 历史加载（切接口 / 挂载时） ───────────────────────────────
async function loadHistory() {
  if (activeRequestId.value != null) {
    await stressStore.loadHistory(activeRequestId.value)
  }
}
watch(activeRequestId, () => { void loadHistory() })
onMounted(loadHistory)
onUnmounted(() => stressStore.cleanup())

// ── 图表重绘 ─────────────────────────────────────────────────
watch(() => stressStore.chartPoints.length, () => {
  if (stressStore.chartPoints.length <= 1) return
  if (chartCanvas.value) drawStressChart(chartCanvas.value, stressStore.chartPoints)
})

// ── 历史对比 ─────────────────────────────────────────────────
function toggleCompare(id: number) {
  const i = compareSelection.value.indexOf(id)
  if (i >= 0) {
    compareSelection.value.splice(i, 1)
  } else if (compareSelection.value.length < 2) {
    compareSelection.value.push(id)
  } else {
    compareSelection.value = [compareSelection.value[1], id]
  }
}

async function doCompare() {
  const runs = compareSelection.value
    .map(id => stressStore.history.find(r => r.id === id))
    .filter((r): r is StressRun => Boolean(r))
  if (runs.length < 2) return
  compareVisible.value = true
  await nextTick()
  if (compareCanvas.value) {
    compareLegend.value = drawCompareChart(compareCanvas.value, runs)
  }
}

// ── 报告导出 ─────────────────────────────────────────────────
async function onExportReport(run: StressRun) {
  const text = buildReport(run)
  message.info(`已复制报告内容至剪贴板，正在保存到本地…`)
  await saveReportToFile(text)
  void navigator.clipboard.writeText(text).catch(() => {})
}
</script>

<style scoped>
.stress-tab {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm);
  overflow-y: auto;
  flex: 1;
  min-height: 0;
}

.stress-tab__toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-md);
  padding-bottom: var(--spacing-sm);
  border-bottom: 1px solid var(--border-base);
}

.cfg-group {
  display: flex;
  align-items: center;
  gap: 6px;
}
.cfg-label {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  white-space: nowrap;
}

.stats-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.stat-cell {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 10px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
}
.stat-label { font-size: 10px; color: var(--text-tertiary); }
.stat-value { font-size: var(--font-size-base); font-weight: 600; font-family: monospace; }
.stat-done { color: var(--color-success); }
.stat-running { flex-direction: row; align-items: center; color: var(--text-secondary); gap: 6px; }
.stress-error { color: var(--color-error); font-size: var(--font-size-sm); }

.chart-box, .compare-box {
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  overflow: hidden;
}
.stress-canvas { display: block; width: 100%; height: 150px; }
.chart-legend {
  display: flex;
  gap: 16px;
  padding: 4px 10px;
  font-size: 11px;
  background: var(--bg-elevated);
  border-top: 1px solid var(--border-base);
}
.legend-tps { color: var(--color-success); }
.legend-avg { color: var(--color-info); }
.legend-p95 { color: var(--color-warning); }
.legend-item { }

.history-block {
  border-top: 1px solid var(--border-base);
  padding-top: var(--spacing-sm);
}
.history-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--spacing-xs);
}
.section-title {
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-tertiary);
}
.history-actions { display: flex; align-items: center; gap: 6px; }

.history-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
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
</style>