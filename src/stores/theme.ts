/**
 * 主题 Store（M3-B 主题底座）
 *
 * 职责：
 * 1. 用户主题偏好的三态管理：light / dark / system（跟随系统）
 * 2. 写 <html data-theme> 切 CSS 变量（驱动自定义组件）
 * 3. 提供 NaiveUI NConfigProvider 所需的 theme + themeOverrides（驱动 NaiveUI 内置组件）
 * 4. 持久化到 app-settings.json（key: theme.mode）
 *
 * 设计要点：
 * - effectiveMode = 'system' 时跟随 prefers-color-scheme，其他模式忽略系统
 * - matchMedia change 监听仅在 mode='system' 时生效，避免污染手动模式
 * - themeOverrides 用 computed 从 CSS 变量读，每次切主题递增 themeTick 触发重算
 *   （CSS 变量先变 → tick 变 → overrides 重算 → NaiveUI 同步主色）
 * - init() 必须在 App.vue onMounted 中尽早调用，避免初始闪白
 */
import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { darkTheme, type GlobalThemeOverrides } from 'naive-ui'
import { readSetting, writeSetting } from './_persistedSettings'

export type ThemeMode = 'light' | 'dark' | 'system'

const THEME_KEY = 'theme.mode'

export const useThemeStore = defineStore('theme', () => {
  const mode = ref<ThemeMode>('system')
  const systemDark = ref(window.matchMedia('(prefers-color-scheme: dark)').matches)
  // 切主题后递增，触发 naiveOverrides 重新读 CSS 变量
  const themeTick = ref(0)

  /** 实际生效的两态：light 或 dark */
  const effectiveMode = computed<'light' | 'dark'>(() => {
    if (mode.value === 'system') return systemDark.value ? 'dark' : 'light'
    return mode.value
  })

  /** NaiveUI 主题对象（null = 默认 light theme） */
  const naiveTheme = computed(() => effectiveMode.value === 'dark' ? darkTheme : null)

  /**
   * NaiveUI overrides：从 CSS 变量读主色，确保 NaiveUI 组件主色与自定义组件一致。
   * 依赖 themeTick 触发重算（CSS 变量切换瞬间 + 1）。
   */
  const naiveOverrides = computed<GlobalThemeOverrides>(() => {
    void themeTick.value  // 显式依赖触发
    const cs = getComputedStyle(document.documentElement)
    const v = (name: string) => cs.getPropertyValue(name).trim()
    const primary = v('--color-primary')
    // 极早期阶段（init 前）CSS 变量可能还没渲染，返回空 overrides 走 NaiveUI 默认
    if (!primary) return {}
    return {
      common: {
        primaryColor:        primary,
        primaryColorHover:   v('--color-primary-hover'),
        primaryColorPressed: v('--color-primary-press'),
      },
    }
  })

  /** 写 <html data-theme>，驱动 CSS 变量切换 */
  function applyTheme() {
    document.documentElement.setAttribute('data-theme', effectiveMode.value)
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

  /**
   * 启动初始化：读偏好 → 应用主题。
   * 必须在 App mount 早期调用（onMounted 中），避免首屏闪白。
   * 读失败 / 不存在 → 维持默认 'system'。
   */
  async function init() {
    const saved = await readSetting<ThemeMode>(THEME_KEY)
    if (saved === 'light' || saved === 'dark' || saved === 'system') {
      mode.value = saved
    }
    // 即使没读到偏好也要 apply 一次，确保 <html data-theme> 有值
    applyTheme()
  }

  return {
    mode,
    effectiveMode,
    naiveTheme,
    naiveOverrides,
    setMode,
    init,
  }
})
