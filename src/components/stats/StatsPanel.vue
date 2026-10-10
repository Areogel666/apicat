<template>
  <div class="stats-panel">
    <div class="stats-panel__toolbar">
      <n-radio-group v-model:value="range" size="small">
        <n-radio-button value="30d">近 30 天</n-radio-button>
        <n-radio-button value="all">全部</n-radio-button>
      </n-radio-group>
      <span class="stats-panel__note">数据仅保存在本机；请求趋势基于本地历史，清理历史后不可恢复</span>
    </div>

    <n-spin :show="loading">
      <n-empty
        v-if="loadError"
        description="统计加载失败"
        class="stats-panel__empty"
      >
        <template #extra>
          <n-button size="tiny" @click="load">重试</n-button>
        </template>
      </n-empty>
      <n-empty
        v-else-if="!loading && data && data.kpi.requests30d === 0 && data.kpi.totalInterfaces === 0"
        description="还没有任何数据——发送第一个请求后这里会出现统计"
        class="stats-panel__empty"
      />
      <template v-else-if="data">
        <!-- 指标条：与主面板工具条同密度，不做卡片堆叠 -->
        <div class="stat-strip">
          <div class="stat">
            <span class="stat__value">{{ data.kpi.totalInterfaces }}</span>
            <span class="stat__label">接口总数</span>
          </div>
          <div class="stat">
            <span class="stat__value">{{ data.kpi.requests30d }}</span>
            <span class="stat__label">近 30 天请求</span>
          </div>
          <div class="stat">
            <span class="stat__value">{{ successRateText }}</span>
            <span class="stat__label">请求成功率（30 天）</span>
          </div>
          <div class="stat">
            <span class="stat__value">{{ data.kpi.stressRunCount }}</span>
            <span class="stat__label">压测次数</span>
          </div>
        </div>

        <div class="stats-grid">
          <section class="stats-section">
            <h3 class="stats-section__title">请求趋势（近 30 天）</h3>
            <StatsChart v-if="trendOption" :option="trendOption" :height="230" />
            <n-empty v-else description="近 30 天暂无请求" class="stats-section__empty" />
          </section>
          <section class="stats-section">
            <h3 class="stats-section__title">状态码分布</h3>
            <StatsChart v-if="statusOption" :option="statusOption" :height="230" />
            <n-empty v-else description="暂无数据" class="stats-section__empty" />
          </section>
          <section class="stats-section">
            <h3 class="stats-section__title">请求方法分布</h3>
            <div v-if="methodRows.length" class="method-rows">
              <div v-for="row in methodRows" :key="row.name" class="method-row">
                <span class="method-row__badge" :style="{ background: row.color }">{{ row.name }}</span>
                <span class="method-row__track">
                  <span class="method-row__bar" :style="{ width: row.percent + '%', background: row.color }"></span>
                </span>
                <span class="method-row__count">{{ row.count }}</span>
              </div>
            </div>
            <n-empty v-else description="暂无数据" class="stats-section__empty" />
          </section>
        </div>

        <div class="stats-grid stats-grid--two">
          <section class="stats-section">
            <h3 class="stats-section__title">数据字典</h3>
            <ul class="stats-list">
              <li><span>字典数</span><b>{{ data.dictionary.dictCount }}</b></li>
              <li><span>词条数</span><b>{{ data.dictionary.itemCount }}</b></li>
              <li><span>字段绑定（项目规则）</span><b>{{ data.dictionary.ruleCount }}</b></li>
              <li><span>字段绑定（接口例外）</span><b>{{ data.dictionary.overrideCount }}</b></li>
            </ul>
          </section>
          <section class="stats-section">
            <h3 class="stats-section__title">接口文档</h3>
            <p v-if="data.docs.fileCount === 0" class="stats-docs-note">
              仅统计已在「项目设置」中配置文档输出目录的项目；当前没有可统计的文档目录。
            </p>
            <ul v-else class="stats-list">
              <li><span>Markdown 文件数</span><b>{{ data.docs.fileCount }}</b></li>
              <li><span>总体量</span><b>{{ formatSize(data.docs.totalSize) }}</b></li>
              <li><span>最近更新</span><b>{{ formatDateTime(data.docs.lastModified) }}</b></li>
            </ul>
          </section>
        </div>
      </template>
    </n-spin>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { NRadioGroup, NRadioButton, NSpin, NEmpty, NButton } from 'naive-ui'
import StatsChart from './StatsChart.vue'
import { useThemeStore } from '../../stores/theme'
import type { UsageStats } from '../../types'

const range = ref<'30d' | 'all'>('30d')
const loading = ref(false)
const loadError = ref(false)
const data = ref<UsageStats | null>(null)

async function load() {
  loading.value = true
  loadError.value = false
  try {
    data.value = await invoke<UsageStats>('get_usage_stats', { range: range.value })
  } catch (e) {
    console.warn('[stats] 统计加载失败:', e)
    loadError.value = true
  } finally {
    loading.value = false
  }
}
onMounted(load)
watch(range, load)

const successRateText = computed(() => {
  const r = data.value?.kpi.successRate30d ?? -1
  return r < 0 ? '—' : `${r}%`
})

// ---- 主题联动：图表色从 CSS token 取值，主题/密度/字号档位变化时重算 ----
// canvas 读不到 CSS var，只能取 computed value；data-theme 换档即触发 tick。
const themeStore = useThemeStore()
const themeTick = ref(0)
let themeObserver: MutationObserver | null = null
onMounted(() => {
  themeObserver = new MutationObserver(() => { themeTick.value++ })
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme', 'data-density', 'data-font-size'],
  })
})
onBeforeUnmount(() => themeObserver?.disconnect())

function cssVar(name: string): string {
  void themeTick.value // 建立响应依赖：token 换档 → option 重算
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

/** 近 30 天逐日序列补零（Rust 只返回有数据的日期） */
const trendOption = computed(() => {
  if (!data.value || data.value.trend.length === 0) return null
  const map = new Map(data.value.trend.map((d) => [d.date, d.count]))
  const days: string[] = []
  const counts: number[] = []
  const now = new Date()
  for (let i = 29; i >= 0; i--) {
    const d = new Date(now)
    d.setUTCDate(d.getUTCDate() - i)
    const key = d.toISOString().slice(0, 10)
    days.push(key.slice(5))
    counts.push(map.get(key) ?? 0)
  }
  return {
    color: [cssVar('--color-primary')],
    xAxis: {
      type: 'category', data: days,
      axisLine: { lineStyle: { color: cssVar('--border-strong') } },
      axisLabel: { color: cssVar('--text-secondary') },
      axisTick: { show: false },
    },
    yAxis: {
      type: 'value', minInterval: 1,
      axisLabel: { color: cssVar('--text-tertiary') },
      splitLine: { lineStyle: { color: cssVar('--border-base') } },
    },
    tooltip: {
      trigger: 'axis',
      backgroundColor: cssVar('--bg-elevated'),
      borderColor: cssVar('--border-strong'),
      textStyle: { color: cssVar('--text-primary'), fontSize: 12 },
    },
    series: [{
      type: 'line', data: counts, smooth: true,
      areaStyle: { color: cssVar('--color-primary-soft') },
    }],
    grid: { left: 40, right: 16, top: 24, bottom: 28 },
  }
})

// 状态桶 → 项目 status token（图表颜色说项目的语言）
const STATUS_VAR: Record<string, string> = {
  '2xx/3xx': '--status-2xx',
  '4xx': '--status-4xx',
  '5xx': '--status-5xx',
  '网络失败': '--status-network',
}

const statusOption = computed(() => {
  const d = data.value?.statusDist ?? []
  if (d.length === 0) return null
  return {
    color: d.map((x) => cssVar(STATUS_VAR[x.name] ?? '--text-tertiary')),
    tooltip: {
      trigger: 'item',
      backgroundColor: cssVar('--bg-elevated'),
      borderColor: cssVar('--border-strong'),
      textStyle: { color: cssVar('--text-primary'), fontSize: 12 },
    },
    legend: {
      bottom: 0, icon: 'circle', itemWidth: 8, itemHeight: 8,
      textStyle: { color: cssVar('--text-secondary'), fontSize: 12 },
    },
    series: [{ type: 'pie', radius: ['44%', '68%'], data: d.map((x) => ({ name: x.name, value: x.count })) }],
  }
})

/** 方法分布：行式条形清单，逐条用 --method-* 徽章色（比饼图贴合项目方法徽章语言） */
const methodRows = computed(() => {
  const d = data.value?.methodDist ?? []
  if (d.length === 0) return []
  const max = Math.max(...d.map((x) => x.count))
  return d.map((x) => ({
    name: x.name,
    count: x.count,
    color: cssVar(`--method-${x.name.toLowerCase()}`) || cssVar('--text-tertiary'),
    percent: max > 0 ? Math.round((x.count / max) * 100) : 0,
  }))
})

/** RFC3339 → 本地展示 'YYYY-MM-DD HH:MM'（只截显示，不做时区换算） */
function formatDateTime(s: string | null): string {
  return s ? s.slice(0, 16).replace('T', ' ') : '—'
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

// themeStore 仅用于建立 store 依赖（mode 变化伴随 data-theme 变更，observer 兜底）
void themeStore
</script>

<style scoped>
.stats-panel {
  padding: var(--spacing-md) var(--spacing-lg);
  overflow-y: auto;
  height: 100%;
}
.stats-panel__toolbar {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}
.stats-panel__note {
  color: var(--text-tertiary);
  font-size: var(--font-size-sm);
}
.stats-panel__empty {
  margin-top: 80px;
}

/* 指标条：数值+标签左对齐横排，细分隔线分组，不做卡片 */
.stat-strip {
  display: flex;
  background: var(--bg-surface);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  margin-bottom: var(--spacing-md);
  overflow: hidden;
}
.stat {
  flex: 1;
  padding: var(--spacing-md) var(--spacing-lg);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}
.stat + .stat {
  border-left: 1px solid var(--border-base);
}
.stat__value {
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.2;
}
.stat__label {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

/* 区块：surface 底 + hairline 标题行，与主面板分区语言一致 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}
.stats-grid--two {
  grid-template-columns: repeat(2, 1fr);
}
@media (max-width: 980px) {
  .stats-grid,
  .stats-grid--two {
    grid-template-columns: 1fr;
  }
  .stat-strip {
    flex-wrap: wrap;
  }
  .stat {
    min-width: 45%;
  }
}
.stats-section {
  background: var(--bg-surface);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  padding: var(--spacing-md);
  min-width: 0;
}
.stats-section__title {
  margin: 0 0 var(--spacing-sm);
  padding-bottom: var(--spacing-sm);
  border-bottom: 1px solid var(--border-base);
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-secondary);
}
.stats-section__empty {
  margin: var(--spacing-lg) 0;
}

/* 方法分布：行式条形清单 */
.method-rows {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  padding-top: var(--spacing-xs);
}
.method-row {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}
.method-row__badge {
  min-width: 44px;
  padding: 0 var(--spacing-xs);
  border-radius: var(--radius-sm);
  color: var(--text-inverse);
  font-size: var(--font-size-sm);
  font-weight: 600;
  text-align: center;
  line-height: var(--row-height);
  height: var(--row-height);
  flex-shrink: 0;
}
.method-row__track {
  flex: 1;
  height: 6px;
  background: var(--bg-active);
  border-radius: 3px;
  overflow: hidden;
}
.method-row__bar {
  display: block;
  height: 100%;
  border-radius: 3px;
  transition: width 0.2s ease;
}
.method-row__count {
  min-width: 40px;
  text-align: right;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

/* 清单：行式 + hairline 分隔 */
.stats-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.stats-list li {
  display: flex;
  justify-content: space-between;
  padding: var(--spacing-xs) 0;
  font-size: var(--font-size-base);
  border-bottom: 1px solid var(--border-base);
}
.stats-list li:last-child {
  border-bottom: none;
}
.stats-list li span {
  color: var(--text-secondary);
}
.stats-list li b {
  font-weight: 600;
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}
.stats-docs-note {
  color: var(--text-tertiary);
  font-size: var(--font-size-sm);
  margin: var(--spacing-sm) 0;
}
</style>
