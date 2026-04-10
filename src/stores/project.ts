import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Project } from '../types'

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const currentProjectId = ref<number | null>(null)
  const loading = ref(false)
  // 导入到当前项目时 currentProjectId 值不变，watch 不触发。
  // 通过递增此计数器通知 Sidebar 强制重载侧边栏数据。
  const sidebarReloadTick = ref(0)

  const currentProject = computed(() =>
    projects.value.find(p => p.id === currentProjectId.value) ?? null
  )

  async function loadProjects() {
    loading.value = true
    try {
      let list = await invoke<Project[]>('list_projects')
      
      // 如果没有任何项目，自动创建一个默认项目
      if (list.length === 0) {
        const defaultProj = await invoke<Project>('create_project', { name: 'Default Project', description: null })
        list = [defaultProj]
      }

      projects.value = list
      // 默认选中第一个项目
      if (projects.value.length > 0 && !currentProjectId.value) {
        currentProjectId.value = projects.value[0].id
      }
    } finally {
      loading.value = false
    }
  }

  async function createProject(name: string, description?: string) {
    const project = await invoke<Project>('create_project', { name, description: description ?? null })
    projects.value.unshift(project)
    currentProjectId.value = project.id
    return project
  }

  async function updateProject(id: number, name: string, description?: string) {
    const updated = await invoke<Project>('update_project', { id, name, description: description ?? null })
    const idx = projects.value.findIndex(p => p.id === id)
    if (idx !== -1) projects.value[idx] = updated
    return updated
  }

  async function deleteProject(id: number) {
    await invoke('delete_project', { id })
    projects.value = projects.value.filter(p => p.id !== id)
    if (currentProjectId.value === id) {
      currentProjectId.value = projects.value[0]?.id ?? null
    }
  }

  return {
    projects,
    currentProjectId,
    currentProject,
    loading,
    sidebarReloadTick,
    loadProjects,
    createProject,
    updateProject,
    deleteProject,
  }
})
