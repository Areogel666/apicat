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
        <div class="stats-kpis">
          <div class="stats-kpi">
            <div class="stats-kpi__value">{{ data.kpi.totalInterfaces }}</div>
            <div class="stats-kpi__label">接口总数</div>
          </div>
          <div class="stats-kpi">
            <div class="stats-kpi__value">{{ data.kpi.requests30d }}</div>
            <div class="stats-kpi__label">近 30 天请求</div>
          </div>
          <div class="stats-kpi">
            <div class="stats-kpi__value">{{ successRateText }}</div>
            <div class="stats-kpi__label">请求成功率（30 天）</div>
          </div>
          <div class="stats-kpi">
            <div class="stats-kpi__value">{{ data.kpi.stressRunCount }}</div>
            <div class="stats-kpi__label">压测次数</div>
          </div>
        </div>

        <div class="stats-grid">
          <section class="stats-card">
            <h3>请求趋势（近 30 天）</h3>
            <StatsChart v-if="trendOption" :option="trendOption" :height="240" />
            <n-empty v-else description="近 30 天暂无请求" />
          </section>
          <section class="stats-card">
            <h3>状态码分布</h3>
            <StatsChart v-if="statusOption" :option="statusOption" :height="240" />
            <n-empty v-else description="暂无数据" />
          </section>
          <section class="stats-card">
            <h3>请求方法分布</h3>
            <StatsChart v-if="methodOption" :option="methodOption" :height="240" />
            <n-empty v-else description="暂无数据" />
          </section>
        </div>

        <div class="stats-grid stats-grid--two">
          <section class="stats-card">
            <h3>数据字典</h3>
            <ul class="stats-list">
              <li><span>字典数</span><b>{{ data.dictionary.dictCount }}</b></li>
              <li><span>词条数</span><b>{{ data.dictionary.itemCount }}</b></li>
              <li><span>字段绑定（项目规则）</span><b>{{ data.dictionary.ruleCount }}</b></li>
              <li><span>字段绑定（接口例外）</span><b>{{ data.dictionary.overrideCount }}</b></li>
            </ul>
          </section>
          <section class="stats-card">
            <h3>接口文档</h3>
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
import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { NRadioGroup, NRadioButton, NSpin, NEmpty, NButton } from 'naive-ui'
import StatsChart from './StatsChart.vue'
import type { UsageStats } from '../../types'

const range = ref<'30d' | 'all'>('30d')
const loading = ref(false)
const data = ref<UsageStats | null>(null)

const loadError = ref(false)

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
    xAxis: { type: 'category', data: days },
    yAxis: { type: 'value', minInterval: 1 },
    tooltip: { trigger: 'axis' },
    series: [{ type: 'line', data: counts, smooth: true, areaStyle: {} }],
    grid: { left: 40, right: 16, top: 24, bottom: 28 },
  }
})

const statusOption = computed(() => {
  const d = data.value?.statusDist ?? []
  if (d.length === 0) return null
  return {
    tooltip: { trigger: 'item' },
    legend: { bottom: 0 },
    series: [{ type: 'pie', radius: ['38%', '68%'], data: d.map((x) => ({ name: x.name, value: x.count })) }],
  }
})

const methodOption = computed(() => {
  const d = data.value?.methodDist ?? []
  if (d.length === 0) return null
  return {
    tooltip: { trigger: 'item' },
    legend: { bottom: 0 },
    series: [{ type: 'pie', radius: '62%', data: d.map((x) => ({ name: x.name, value: x.count })) }],
  }
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
</script>

<style scoped>
.stats-panel {
  padding: var(--spacing-md);
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
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}
.stats-panel__empty {
  margin-top: 80px;
}
.stats-kpis {
  display: flex;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}
.stats-kpi {
  flex: 1;
  background: var(--bg-elevated);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  padding: var(--spacing-md);
  text-align: center;
}
.stats-kpi__value {
  font-size: var(--font-size-lg);
  font-weight: 600;
}
.stats-kpi__label {
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
  margin-top: var(--spacing-xs);
}
.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}
.stats-grid--two {
  grid-template-columns: repeat(2, 1fr);
}
.stats-card {
  background: var(--bg-elevated);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  padding: var(--spacing-md);
}
.stats-card h3 {
  margin: 0 0 var(--spacing-sm);
  font-size: var(--font-size-base);
}
.stats-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.stats-list li {
  display: flex;
  justify-content: space-between;
  padding: var(--spacing-xs) 0;
  border-bottom: 1px solid var(--border-base);
}
.stats-list li:last-child {
  border-bottom: none;
}
.stats-docs-note {
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}
</style>
