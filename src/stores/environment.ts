import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Environment, EnvVariable } from '../types'

export const useEnvironmentStore = defineStore('environment', () => {
  const environments = ref<Environment[]>([])
  const variables = ref<EnvVariable[]>([])  // 当前选中环境的变量

  // 当前激活环境（is_active=1）
  const activeEnv = computed<Environment | null>(
    () => environments.value.find(e => e.is_active === 1) ?? null
  )
  const activeEnvId = computed<number | null>(() => activeEnv.value?.id ?? null)

  // ── 环境 CRUD ──────────────────────────────────────────────
  async function loadEnvironments(projectId: number) {
    environments.value = await invoke<Environment[]>('list_environments', { project_id: projectId })
  }

  async function createEnvironment(projectId: number, name: string, baseUrl?: string | null) {
    const env = await invoke<Environment>('create_environment', {
      project_id: projectId,
      name,
      base_url: baseUrl ?? null,
    })
    environments.value.push(env)
    return env
  }

  async function updateEnvironment(id: number, name: string, baseUrl?: string | null) {
    const updated = await invoke<Environment>('update_environment', {
      id,
      name,
      base_url: baseUrl ?? null,
    })
    const idx = environments.value.findIndex(e => e.id === id)
    if (idx !== -1) environments.value[idx] = updated
    return updated
  }

  async function deleteEnvironment(id: number) {
    await invoke('delete_environment', { id })
    environments.value = environments.value.filter(e => e.id !== id)
    if (variables.value.length > 0 && variables.value[0]?.env_id === id) {
      variables.value = []
    }
  }

  async function activateEnvironment(projectId: number, envId: number) {
    await invoke('activate_environment', { project_id: projectId, env_id: envId })
    // 本地更新 is_active
    environments.value.forEach(e => { e.is_active = e.id === envId ? 1 : 0 })
  }

  async function deactivateEnvironment(projectId: number) {
    await invoke('deactivate_environment', { project_id: projectId })
    environments.value.forEach(e => { e.is_active = 0 })
  }

  // ── 环境变量 CRUD ──────────────────────────────────────────
  async function loadVariables(envId: number) {
    variables.value = await invoke<EnvVariable[]>('list_env_variables', { env_id: envId })
  }

  async function createVariable(envId: number, key: string, value: string, description?: string | null) {
    const v = await invoke<EnvVariable>('create_env_variable', {
      env_id: envId, key, value, description: description ?? null,
    })
    variables.value.push(v)
    return v
  }

  async function updateVariable(id: number, key: string, value: string, description: string | null, enabled: number) {
    const updated = await invoke<EnvVariable>('update_env_variable', {
      id, key, value, description, enabled,
    })
    const idx = variables.value.findIndex(v => v.id === id)
    if (idx !== -1) variables.value[idx] = updated
    return updated
  }

  async function deleteVariable(id: number) {
    await invoke('delete_env_variable', { id })
    variables.value = variables.value.filter(v => v.id !== id)
  }

  return {
    environments,
    variables,
    activeEnv,
    activeEnvId,
    loadEnvironments,
    createEnvironment,
    updateEnvironment,
    deleteEnvironment,
    activateEnvironment,
    deactivateEnvironment,
    loadVariables,
    createVariable,
    updateVariable,
    deleteVariable,
  }
})
