<template>
  <div class="token-editor">
    <div
      v-for="group in groups"
      :key="group.key"
      class="group"
    >
      <div class="group-header" @click="toggleGroup(group.key)">
        <span class="arrow" :class="{ expanded: expandedGroups.has(group.key) }">▶</span>
        <span class="group-label">{{ group.label }}</span>
      </div>
      <div v-if="expandedGroups.has(group.key)" class="group-body">
        <!-- 色板推荐（仅品牌色组显示） -->
        <div v-if="group.key === 'brand'" class="palette-strip">
          <span class="palette-hint">推荐色板</span>
          <span
            v-for="color in recommendedColors"
            :key="color"
            class="chip"
            :style="{ background: color }"
            :class="{ active: resolvedTokens['--color-primary'] === color }"
            @click="setPrimaryColor(color)"
          />
        </div>
        <TokenColorRow
          v-for="token in group.tokens"
          :key="token.key"
          :token-key="token.key"
          :current-value="resolvedTokens[token.key] || ''"
          :default-value="defaultVal(token.key)"
          :description="token.description"
          @update:value="(v: string) => onTokenChange(token.key, v)"
          @reset="onTokenReset(token.key)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, computed } from 'vue'
import { useThemeStore } from '../../stores/theme'
import { DEFAULT_LIGHT_TOKENS, DEFAULT_DARK_TOKENS } from './tokenDefaults'
import TokenColorRow from './TokenColorRow.vue'

const themeStore = useThemeStore()

// 推荐色板（7 色）
const recommendedColors = [
  '#63cd96', '#4a9eff', '#e88a6e', '#b39ff0',
  '#f0c060', '#40c9c6', '#ff6b9d',
]

/** 获取当前模式下的 token 默认值 */
function defaultVal(key: string): string {
  const base = themeStore.effectiveMode === 'dark'
    ? DEFAULT_DARK_TOKENS
    : DEFAULT_LIGHT_TOKENS
  return base[key] || ''
}

/** 哪些组默认展开 */
const expandedGroups = reactive(new Set<string>(['brand', 'background']))

function toggleGroup(key: string) {
  if (expandedGroups.has(key)) {
    expandedGroups.delete(key)
  } else {
    expandedGroups.add(key)
  }
}

interface TokenDef {
  key: string
  description: string
}

interface GroupDef {
  key: string
  label: string
  tokens: TokenDef[]
}

/** 4 组 token 定义 */
const groups: GroupDef[] = [
  {
    key: 'brand',
    label: '品牌色',
    tokens: [
      { key: '--color-primary', description: '主色 / 按钮、链接、选中态' },
      { key: '--color-primary-hover', description: '悬停高亮' },
      { key: '--color-primary-press', description: '按下加深' },
      { key: '--color-primary-soft', description: '柔和背景色' },
    ],
  },
  {
    key: 'background',
    label: '背景色',
    tokens: [
      { key: '--bg-base', description: '页面根背景' },
      { key: '--bg-surface', description: '卡片 / 面板背景' },
      { key: '--bg-elevated', description: '浮层 / 弹窗背景' },
      { key: '--bg-hover', description: '行悬停背景' },
      { key: '--bg-active', description: '按下态背景' },
      { key: '--bg-selected', description: '选中态背景' },
      { key: '--border-base', description: '默认边框' },
      { key: '--border-strong', description: '强调边框' },
    ],
  },
  {
    key: 'text',
    label: '文字色',
    tokens: [
      { key: '--text-primary', description: '正文 / 标题' },
      { key: '--text-secondary', description: '辅助信息' },
      { key: '--text-tertiary', description: '占位 / 水印' },
      { key: '--text-disabled', description: '禁用态文字' },
      { key: '--text-inverse', description: '反色文字（深色底上）' },
    ],
  },
  {
    key: 'semantic',
    label: '语义色 · 边框 · 阴影 · HTTP Method',
    tokens: [
      { key: '--color-success', description: '成功' },
      { key: '--color-warning', description: '警告' },
      { key: '--color-error', description: '错误' },
      { key: '--color-info', description: '信息' },
    ],
  },
]

// 通过 resolvedTokens 读当前生效值（computed 保持响应式，修改后自动刷新）
const resolvedTokens = computed(() => themeStore.resolvedTokens)

function onTokenChange(key: string, value: string) {
  themeStore.customTokens = { ...themeStore.customTokens, [key]: value }
  themeStore.applyTheme()
}

function onTokenReset(key: string) {
  themeStore.resetToken(key)
  themeStore.applyTheme()
}

function setPrimaryColor(color: string) {
  // 选择推荐色板时自动计算 hover/press/soft
  const hover = lightenColor(color, 0.08)
  const press = darkenColor(color, 0.12)
  const soft = hexToRgba(color, 0.14)
  const newTokens = {
    ...themeStore.customTokens,
    '--color-primary': color,
    '--color-primary-hover': hover,
    '--color-primary-press': press,
    '--color-primary-soft': soft,
  }
  themeStore.customTokens = newTokens
  themeStore.applyTheme()
}

// ---- 简易颜色工具（内联，避免引入依赖） ----

function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace('#', '')
  return [
    parseInt(h.slice(0, 2), 16),
    parseInt(h.slice(2, 4), 16),
    parseInt(h.slice(4, 6), 16),
  ]
}

function rgbToHex(r: number, g: number, b: number): string {
  const clamp = (v: number) => Math.max(0, Math.min(255, Math.round(v)))
  return '#' + [clamp(r), clamp(g), clamp(b)]
    .map(v => v.toString(16).padStart(2, '0'))
    .join('')
}

function lightenColor(hex: string, amount: number): string {
  const [r, g, b] = hexToRgb(hex)
  return rgbToHex(
    r + (255 - r) * amount,
    g + (255 - g) * amount,
    b + (255 - b) * amount,
  )
}

function darkenColor(hex: string, amount: number): string {
  const [r, g, b] = hexToRgb(hex)
  return rgbToHex(r * (1 - amount), g * (1 - amount), b * (1 - amount))
}

function hexToRgba(hex: string, alpha: number): string {
  const [r, g, b] = hexToRgb(hex)
  return `rgba(${r},${g},${b},${alpha})`
}
</script>

<style scoped>
.token-editor {
  display: flex;
  flex-direction: column;
}
.group {
  margin-bottom: 2px;
}
.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 12px;
  background: var(--bg-elevated);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  border: 1px solid transparent;
}
.group-header:hover {
  border-color: var(--border-base);
}
.arrow {
  font-size: 10px;
  transition: transform 0.2s;
  color: var(--text-tertiary);
  width: 12px;
}
.arrow.expanded {
  transform: rotate(90deg);
}
.arrow:not(.expanded) {
  transform: rotate(0deg);
}
.group-label {
  flex: 1;
}
.group-body {
  padding: 6px 4px 4px;
}
.palette-strip {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
  padding: 0 12px;
}
.palette-hint {
  font-size: 10px;
  color: var(--text-tertiary);
  line-height: 28px;
}
.chip {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: border-color 0.15s;
}
.chip:hover {
  border-color: var(--text-tertiary);
}
.chip.active {
  border-color: var(--text-primary);
  box-shadow: 0 0 0 2px rgba(128,128,128,0.2);
}
</style>
