/**
 * 预设主题模版常量
 *
 * 5 套预设：深蓝 / 竹林（ApiCat 默认）/ 暖橘 / 紫苑 / 冰晶（1.0.9 玻璃质感）
 * 每套包含 token 覆盖 + 风格预设值
 */

export interface ThemePreset {
  id: string
  name: string
  description: string
  mode: 'light' | 'dark'
  /** 要覆盖的 token（不填的 token 保持默认值） */
  tokens: Record<string, string>
  /** 代表色（仅主色，用于卡片色块展示；背景/次色不再暗示） */
  swatches: string[]
  density: 'compact' | 'default' | 'spacious'
  radiusScale: 0.5 | 1.0 | 1.5
  fontSize: 's' | 'm' | 'l'
  /** 1.0.9 玻璃质感：表面半透明（tokens 覆盖 --bg-surface/--bg-elevated 为 rgba）+ backdrop blur */
  glass?: boolean
}

export const PRESET_THEMES: ThemePreset[] = [
  {
    id: 'ocean-blue',
    name: '深蓝',
    description: '专业冷静的深蓝主题',
    mode: 'dark',
    swatches: ['#4a9eff'],
    tokens: {
      '--color-primary': '#4a9eff',
      '--color-primary-hover': '#3685e0',
      '--color-primary-press': '#2b6cb0',
      '--color-primary-soft': 'rgba(74,158,255,0.14)',
      '--color-success': '#63cd96',
      '--color-warning': '#f0c060',
      '--color-error': '#e88080',
      '--color-info': '#70c0e8',
    },
    density: 'default',
    radiusScale: 1.0,
    fontSize: 'm',
  },
  {
    id: 'bamboo',
    name: '竹林',
    description: 'ApiCat 默认绿色主题',
    mode: 'dark',
    swatches: ['#63cd96'],
    tokens: {
      '--color-primary': '#63cd96',
      '--color-primary-hover': '#7fdfa8',
      '--color-primary-press': '#4baa78',
      '--color-primary-soft': 'rgba(99,205,150,0.16)',
      '--color-success': '#63cd96',
      '--color-warning': '#f2c97d',
      '--color-error': '#e88080',
      '--color-info': '#70c0e8',
    },
    density: 'default',
    radiusScale: 1.0,
    fontSize: 'm',
  },
  {
    id: 'warm-orange',
    name: '暖橘',
    description: '温暖明亮的浅色主题',
    mode: 'light',
    swatches: ['#e88a6e'],
    tokens: {
      '--color-primary': '#e88a6e',
      '--color-primary-hover': '#f09e88',
      '--color-primary-press': '#c76a52',
      '--color-primary-soft': 'rgba(232,138,110,0.14)',
      '--color-success': '#18a058',
      '--color-warning': '#f0a020',
      '--color-error': '#d03050',
      '--color-info': '#2080f0',
    },
    density: 'default',
    radiusScale: 1.0,
    fontSize: 'm',
  },
  {
    id: 'purple-haze',
    name: '紫苑',
    description: '优雅浪漫的紫色主题',
    mode: 'dark',
    swatches: ['#b39ff0'],
    tokens: {
      '--color-primary': '#b39ff0',
      '--color-primary-hover': '#c5b5f5',
      '--color-primary-press': '#9178d6',
      '--color-primary-soft': 'rgba(179,159,240,0.16)',
      '--color-success': '#63cd96',
      '--color-warning': '#f0c060',
      '--color-error': '#ff6b9d',
      '--color-info': '#70c0e8',
    },
    density: 'default',
    radiusScale: 1.0,
    fontSize: 'm',
  },
  {
    id: 'frosted',
    name: '冰晶',
    description: '冰蓝玻璃质感——半透明卡片与弹窗',
    mode: 'dark',
    glass: true,
    swatches: ['#7fb8e6'],
    tokens: {
      '--color-primary': '#7fb8e6',
      '--color-primary-hover': '#96c9f0',
      '--color-primary-press': '#5b93c4',
      '--color-primary-soft': 'rgba(127,184,230,0.16)',
      '--color-success': '#63cd96',
      '--color-warning': '#f0c060',
      '--color-error': '#e88080',
      '--color-info': '#70c0e8',
      // 玻璃表面：半透明 rgba 透出 --bg-base 底色；backdrop blur 由 data-glass CSS 规则负责。
      // alpha 不低于 0.7——文字（rgba(255,255,255,0.92)）在任何底图上对比度都有兜底
      '--bg-surface': 'rgba(31, 35, 44, 0.72)',
      '--bg-elevated': 'rgba(42, 46, 56, 0.76)',
    },
    density: 'default',
    radiusScale: 1.5,
    fontSize: 'm',
  },
]
