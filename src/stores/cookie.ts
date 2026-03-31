import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CookieItem } from '../types'

export const useCookieStore = defineStore('cookie', () => {
  const globalCookies = ref<CookieItem[]>([])
  const projectCookies = ref<CookieItem[]>([])

  async function loadGlobalCookies() {
    // Tauri 2.x #[command] 宏把 Rust snake_case 参数名转为 camelCase IPC key
    globalCookies.value = await invoke<CookieItem[]>('list_cookies', {
      scopeType: 'global',
      projectId: null,
    })
  }

  async function loadProjectCookies(projectId: number) {
    projectCookies.value = await invoke<CookieItem[]>('list_cookies', {
      scopeType: 'project',
      projectId,
    })
  }

  async function createCookie(
    scopeType: string,
    projectId: number | null,
    domain: string,
    name: string,
    value: string,
    path: string,
  ) {
    const c = await invoke<CookieItem>('create_cookie', {
      scopeType,
      projectId,
      domain, name, value, path,
    })
    if (scopeType === 'global') {
      globalCookies.value.push(c)
    } else {
      projectCookies.value.push(c)
    }
    return c
  }

  async function updateCookie(id: number, value: string, path: string, enabled: number) {
    const updated = await invoke<CookieItem>('update_cookie', { id, value, path, enabled })
    const updateList = (list: CookieItem[]) => {
      const idx = list.findIndex(c => c.id === id)
      if (idx !== -1) list[idx] = updated
    }
    updateList(globalCookies.value)
    updateList(projectCookies.value)
    return updated
  }

  async function deleteCookie(id: number) {
    await invoke('delete_cookie', { id })
    globalCookies.value = globalCookies.value.filter(c => c.id !== id)
    projectCookies.value = projectCookies.value.filter(c => c.id !== id)
  }

  return {
    globalCookies,
    projectCookies,
    loadGlobalCookies,
    loadProjectCookies,
    createCookie,
    updateCookie,
    deleteCookie,
  }
})
