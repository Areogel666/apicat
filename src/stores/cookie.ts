import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CookieItem } from '../types'
import { nowUtcString } from '../utils/time'

export type CookieScope = 'global' | 'project' | 'env'

/** 「作用域」筛选值：'all' 表示不过滤 */
export type CookieScopeFilter = CookieScope | 'all'

export const useCookieStore = defineStore('cookie', () => {
  const cookies = ref<CookieItem[]>([])
  const loading = ref(false)
  /** 打开 Cookie 管理的信号（MainPanel 只读区 → TopBar 弹窗），同 envStore.requestOpenManager 模式 */
  const openManagerTick = ref(0)

  function requestOpenManager() {
    openManagerTick.value++
  }

  /**
   * 按作用域拉取。scope='all' 时一次拉全三层（1.0.7 改成一张表 + 筛选列后，
   * 弹窗只调这一个方法，不再分 tab 各自加载——旧版把加载挂在 n-tab-pane 的
   * @click 上点不着，导致切项目后看到旧数据）。
   */
  async function loadCookies(opts?: {
    scope?: CookieScopeFilter
    projectId?: number | null
    envId?: number | null
  }) {
    const scope = opts?.scope ?? 'all'
    loading.value = true
    try {
      if (scope === 'all') {
        const [g, p, e] = await Promise.all([
          invoke<CookieItem[]>('list_cookies', { scopeType: 'global', projectId: null, envId: null }),
          opts?.projectId
            ? invoke<CookieItem[]>('list_cookies', { scopeType: 'project', projectId: opts.projectId, envId: null })
            : Promise.resolve([]),
          // env 层不传 envId = 取所有环境，供「环境」筛选下拉用
          invoke<CookieItem[]>('list_cookies', { scopeType: 'env', projectId: null, envId: null }),
        ])
        cookies.value = [...g, ...p, ...e]
      } else {
        cookies.value = await invoke<CookieItem[]>('list_cookies', {
          scopeType: scope,
          projectId: opts?.projectId ?? null,
          envId: opts?.envId ?? null,
        })
      }
    } finally {
      loading.value = false
    }
  }

  /** 新建（domain 统一在后端 normalize_domain 规范化） */
  async function createCookie(payload: {
    scopeType: CookieScope
    projectId: number | null
    envId: number | null
    domain: string
    name: string
    value: string
    path: string
    expiresAt?: string | null
    httpOnly?: boolean
    secure?: boolean
  }) {
    const c = await invoke<CookieItem>('create_cookie', {
      scopeType: payload.scopeType,
      projectId: payload.projectId,
      envId: payload.envId,
      domain: payload.domain,
      name: payload.name,
      value: payload.value,
      path: payload.path,
      expiresAt: payload.expiresAt ?? null,
      httpOnly: payload.httpOnly ?? false,
      secure: payload.secure ?? false,
    })
    cookies.value.unshift(c)
    return c
  }

  /**
   * 更新。Q32-A：可改 domain/name/value/path/expires_at/http_only/secure/enabled，
   * 不可改作用域（改作用域 = 删了重建）。
   */
  async function updateCookie(
    id: number,
    patch: {
      domain?: string
      name?: string
      value?: string
      path?: string
      expiresAt?: string | null
      httpOnly?: boolean
      secure?: boolean
      enabled?: number
    },
  ) {
    const updated = await invoke<CookieItem>('update_cookie', { id, ...patch })
    const idx = cookies.value.findIndex(c => c.id === id)
    if (idx !== -1) cookies.value[idx] = updated
    return updated
  }

  async function deleteCookie(id: number) {
    await invoke('delete_cookie', { id })
    cookies.value = cookies.value.filter(c => c.id !== id)
  }

  /**
   * 清理（Q19-C）。两个可勾选项，可同时勾。
   * - 已过期：expires_at 非空且 <= now
   * - 无过期时间：expires_at 为空（会话 cookie，Q23-A 落库但永不过期，只能手动清）
   */
  async function cleanupCookies(opts: { expired?: boolean; session?: boolean }) {
    const now = nowUtcString()
    const doomed = cookies.value.filter(c => {
      if (opts.expired && c.expires_at && c.expires_at <= now) return true
      if (opts.session && !c.expires_at) return true
      return false
    })
    await Promise.all(doomed.map(c => deleteCookie(c.id)))
    return doomed.length
  }

  return {
    cookies,
    loading,
    openManagerTick,
    requestOpenManager,
    loadCookies,
    createCookie,
    updateCookie,
    deleteCookie,
    cleanupCookies,
  }
})
