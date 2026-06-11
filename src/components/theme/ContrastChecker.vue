<template>
  <div v-if="hasModifications" class="contrast-panel">
    <!-- 检测摘要条 -->
    <div class="contrast-summary">
      <span class="summary-label">文字对比度</span>
      <div class="contrast-bar">
        <div
          v-for="(seg, i) in barSegments"
          :key="i"
          class="seg"
          :style="{ background: seg.color, flex: seg.width }"
        />
      </div>
      <div class="contrast-grade">
        <span>Fail</span>
        <span>AA</span>
        <span>AAA ✓</span>
      </div>
    </div>

    <!-- 每条检测结果 -->
    <div
      v-for="result in results"
      :key="result.label"
      class="result-row"
      :class="{ warn: result.ratio < result.minAA }"
    >
      <span class="result-label">{{ result.label }}</span>
      <span class="result-ratio">{{ result.ratio.toFixed(1) }}:1</span>
      <span class="result-grade">
        {{ result.ratio >= 7 ? 'AAA ✓' : result.ratio >= result.minAA ? 'AA ✓' : '✗ 未达标' }}
      </span>
    </div>

    <!-- 不及格警告 -->
    <div v-if="failedItems.length > 0" class="contrast-warn">
      <span>⚠</span>
      <span>
        <template v-for="item in failedItems" :key="item.label">
          {{ item.label }} 对比度 {{ item.ratio.toFixed(1) }}:1，未达 AA 标准（{{ item.minAA }}:1）。
          建议加深文字色或提亮背景色。
        </template>
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useThemeStore } from '../../stores/theme'

const themeStore = useThemeStore()

/** 仅在用户修改过 token 时才检测（避免默认主题下显示无意义警告） */
const hasModifications = computed(() => Object.keys(themeStore.customTokens).length > 0)

/** sRGB 分量转线性分量 */
function toLinear(c: number): number {
  c = c / 255
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4)
}

/** 计算相对亮度（支持 hex 和 rgba） */
function relativeLuminance(color: string): number {
  const hex = color.trim()
  if (!hex.startsWith('#')) {
    // rgba(...) 格式：提取 RGB 分量
    const match = hex.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/)
    if (match) {
      const r = parseInt(match[1])
      const g = parseInt(match[2])
      const b = parseInt(match[3])
      return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b)
    }
    return 0
  }
  const r = parseInt(hex.slice(1, 3), 16)
  const g = parseInt(hex.slice(3, 5), 16)
  const b = parseInt(hex.slice(5, 7), 16)
  return 0.2126 * toLinear(r) + 0.7152 * toLinear(g) + 0.0722 * toLinear(b)
}

/** 计算对比度比率 */
function contrastRatio(fg: string, bg: string): number {
  const l1 = relativeLuminance(fg)
  const l2 = relativeLuminance(bg)
  const lighter = Math.max(l1, l2)
  const darker = Math.min(l1, l2)
  if (darker === 0 && lighter === 0) return 21
  return (lighter + 0.05) / (darker + 0.05)
}

interface CheckItem {
  label: string
  fg: string
  bg: string
  minAA: number
}

const CHECKS: CheckItem[] = [
  { label: '正文 → 页面背景', fg: '--text-primary', bg: '--bg-base', minAA: 4.5 },
  { label: '辅助文字 → 页面背景', fg: '--text-secondary', bg: '--bg-base', minAA: 4.5 },
  { label: '正文 → 卡片背景', fg: '--text-primary', bg: '--bg-surface', minAA: 4.5 },
  { label: '辅助文字 → 卡片背景', fg: '--text-secondary', bg: '--bg-surface', minAA: 4.5 },
]

interface ResultItem {
  label: string
  ratio: number
  minAA: number
}

const results = computed<ResultItem[]>(() => {
  const t = themeStore.resolvedTokens
  return CHECKS.map(c => ({
    label: c.label,
    ratio: contrastRatio(t[c.fg] || '#000', t[c.bg] || '#fff'),
    minAA: c.minAA,
  }))
})

const failedItems = computed(() => results.value.filter(r => r.ratio < r.minAA))

const barSegments = computed(() => {
  if (results.value.length === 0) return []
  const minRatio = Math.min(...results.value.map(r => r.ratio))
  const maxRatio = Math.max(...results.value.map(r => r.ratio))
  const range = maxRatio - minRatio || 1
  return results.value.map(r => {
    const pct = Math.max(1, (r.ratio - minRatio) / range * 100)
    const color = r.ratio >= 7 ? '#63cd96' : r.ratio >= r.minAA ? '#f0c060' : '#e88080'
    return { width: pct, color }
  })
})
</script>

<style scoped>
.contrast-panel {
  margin-top: 10px;
  padding: 10px 12px;
  background: var(--bg-elevated);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-base);
}
.contrast-summary {
  margin-bottom: 6px;
}
.summary-label {
  font-size: 11px;
  color: var(--text-secondary);
}
.contrast-bar {
  display: flex;
  gap: 1px;
  margin-top: 4px;
  height: 4px;
  border-radius: 2px;
  overflow: hidden;
}
.seg {
  height: 100%;
  border-radius: 1px;
}
.contrast-grade {
  display: flex;
  justify-content: space-between;
  font-size: 9px;
  color: var(--text-tertiary);
  margin-top: 2px;
}
.result-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 0;
  font-size: 11px;
}
.result-label {
  flex: 1;
  color: var(--text-secondary);
}
.result-ratio {
  font-family: monospace;
  width: 48px;
  text-align: right;
}
.result-grade {
  font-size: 10px;
  width: 48px;
  text-align: right;
  color: var(--color-success);
}
.result-row.warn .result-grade {
  color: var(--color-error);
}
.contrast-warn {
  background: rgba(232, 128, 128, 0.08);
  border: 1px solid rgba(232, 128, 128, 0.2);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  font-size: 11px;
  color: var(--color-error);
  display: flex;
  align-items: flex-start;
  gap: 4px;
  margin-top: 6px;
}
</style>
