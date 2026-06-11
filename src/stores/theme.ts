/**
 * 主题 Store（M3-B 主题底座 + 1.0.3 主题工作室扩展）
 *
 * 职责：
 * 1. 用户主题偏好的三态管理：light / dark / system（跟随系统）
 * 2. 写 <html data-theme> 切换 CSS 变量（驱动自定义组件）
 * 3. 提供 NaiveUI NConfigProvider 所需的 theme + themeOverrides（驱动 NaiveUI 内置组件）
 * 4. 持久化到 app-settings.json
 * 5. [1.0.3 新增] 自定义 token 增量覆盖
 * 6. [1.0.3 新增] 风格预设：密度 / 圆角 / 字体大小
 *
 * 设计要点：
 * - effectiveMode = 'system' 时跟随 prefers-color-scheme，其他模式忽略系统
 * - matchMedia change 监听仅在 mode='system' 时生效，避免污染手动模式
 * - themeOverrides 是 computed，从 resolvedTokens 读取，每次切主题递增 themeTick 触发重算
 * - customTokens 只存增量（用户改过的 token），读取时与默认值合并
 * - init() 必须在 App.vue onMounted 中尽早调用，避免初始闪白
 */
import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { darkTheme, type GlobalThemeOverrides } from 'naive-ui'
import { readSetting, writeSetting, deleteSetting } from './_persistedSettings'
import { DEFAULT_LIGHT_TOKENS, DEFAULT_DARK_TOKENS } from '../components/theme/tokenDefaults'

export type ThemeMode = 'light' | 'dark' | 'system'

const THEME_KEY = 'theme.mode'

export const useThemeStore = defineStore('theme', () => {
  const mode = ref<ThemeMode>('system')
  const systemDark = ref(window.matchMedia('(prefers-color-scheme: dark)').matches)
  const themeTick = ref(0)

  // ====== 1.0.3 新增字段 ======
  /** 用户自定义 token（增量存储，只含被修改的 key） */
  const customTokens = ref<Record<string, string>>({})
  /** 布局密度 */
  const density = ref<'compact' | 'default' | 'spacious'>('default')
  /** 圆角缩放倍数 */
  const radiusScale = ref<0.5 | 1.0 | 1.5>(1.0)
  /** 基础字号 */
  const fontSize = ref<'s' | 'm' | 'l'>('m')

  /** 实际生效的两态：light 或 dark */
  const effectiveMode = computed<'light' | 'dark'>(() => {
    if (mode.value === 'system') return systemDark.value ? 'dark' : 'light'
    return mode.value
  })

  /** 合并默认 token + 用户自定义 token */
  const resolvedTokens = computed<Record<string, string>>(() => {
    const base = effectiveMode.value === 'dark'
      ? { ...DEFAULT_DARK_TOKENS }
      : { ...DEFAULT_LIGHT_TOKENS }
    return { ...base, ...customTokens.value }
  })

  /** NaiveUI 主题对象（null = 默认 light theme） */
  const naiveTheme = computed(() => effectiveMode.value === 'dark' ? darkTheme : null)

  /**
   * NaiveUI overrides：从 resolvedTokens 读取主色，确保 NaiveUI 组件主色与自定义组件一致。
   */
  const naiveOverrides = computed<GlobalThemeOverrides>(() => {
    void themeTick.value
    const t = resolvedTokens.value
    const primary = t['--color-primary']
    if (!primary) return {}
    return {
      common: {
        primaryColor:        primary,
        primaryColorHover:   t['--color-primary-hover'] || primary,
        primaryColorPressed: t['--color-primary-press'] || primary,
      },
    }
  })

  /** 写 <html data-theme> + 自定义 token + 风格预设 */
  function applyTheme() {
    const el = document.documentElement

    // 1. data-theme（不变）
    el.setAttribute('data-theme', effectiveMode.value)

    // 2. 覆盖自定义 token 到 <html style>
    const overrides = Object.entries(customTokens.value)
      .map(([k, v]) => `${k}: ${v}`)
      .join('; ')
    if (overrides) {
      el.setAttribute('style', overrides)
    } else {
      el.removeAttribute('style')
    }

    // 3. 风格预设
    el.setAttribute('data-density', density.value)
    el.setAttribute('data-font-size', fontSize.value)

    // 4. 圆角缩放（JS 计算，避免 CSS calc() 兼容问题）
    const scale = radiusScale.value
    el.style.setProperty('--radius-sm', `${4 * scale}px`)
    el.style.setProperty('--radius-md', `${6 * scale}px`)
    el.style.setProperty('--radius-lg', `${10 * scale}px`)

    // 5. tick
    themeTick.value++
  }

  /** 监听系统主题变化（仅当 mode=system 时生效） */
  const mq = window.matchMedia('(prefers-color-scheme: dark)')
  mq.addEventListener('change', e => {
    systemDark.value = e.matches
    if (mode.value === 'system') applyTheme()
  })

  // effectiveMode 变化（包括 mode 切换 + system 跟随）→ 同步刷新 DOM
  watch(effectiveMode, applyTheme)

  /** 用户切换主题：写偏好 + 应用 */
  async function setMode(m: ThemeMode) {
    mode.value = m
    await writeSetting(THEME_KEY, m)
  }

  // ====== 1.0.3 新增 action ======

  /** 应用自定义主题：保存所有自定义到 app-settings.json 并刷新 DOM */
  async function applyCustomTheme() {
    await writeSetting('theme.customTokens', customTokens.value)
    await writeSetting('theme.density', density.value)
    await writeSetting('theme.radiusScale', radiusScale.value)
    await writeSetting('theme.fontSize', fontSize.value)
    applyTheme()
  }

  /** 重置单个 token 为默认值 */
  function resetToken(key: string) {
    const newTokens = { ...customTokens.value }
    delete newTokens[key]
    customTokens.value = newTokens
  }

  /** 全部重置为默认主题 */
  async function resetAll() {
    customTokens.value = {}
    density.value = 'default'
    radiusScale.value = 1.0
    fontSize.value = 'm'
    await deleteSetting('theme.customTokens')
    await deleteSetting('theme.density')
    await deleteSetting('theme.radiusScale')
    await deleteSetting('theme.fontSize')
    applyTheme()
  }

  /**
   * 启动初始化：读偏好 → 应用主题。
   * 必须在 App mount 早期调用（onMounted 中），避免首屏闪白。
   */
  async function init() {
    const saved = await readSetting<ThemeMode>(THEME_KEY)
    if (saved === 'light' || saved === 'dark' || saved === 'system') {
      mode.value = saved
    }

    // 1.0.3：恢复自定义主题
    const ct = await readSetting<Record<string, string>>('theme.customTokens')
    if (ct) customTokens.value = ct

    const d = await readSetting<string>('theme.density')
    if (d === 'compact' || d === 'default' || d === 'spacious') density.value = d

    const rs = await readSetting<number>('theme.radiusScale')
    if (rs === 0.5 || rs === 1.0 || rs === 1.5) radiusScale.value = rs

    const fs = await readSetting<string>('theme.fontSize')
    if (fs === 's' || fs === 'm' || fs === 'l') fontSize.value = fs

    applyTheme()
  }

  return {
    // 1.0.2 原有
    mode,
    effectiveMode,
    naiveTheme,
    naiveOverrides,
    setMode,
    init,
    // 1.0.3 新增
    customTokens,
    density,
    radiusScale,
    fontSize,
    resolvedTokens,
    applyTheme,
    applyCustomTheme,
    resetToken,
    resetAll,
  }
})
