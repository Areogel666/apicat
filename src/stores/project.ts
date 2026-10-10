import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Project } from '../types'
import { readSetting, writeSetting, deleteSetting } from './_persistedSettings'

// 持久化 key（命名空间约定见 _persistedSettings.ts）
const LAST_OPENED_PROJECT_KEY = 'lastOpenedProjectId'

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const currentProjectId = ref<number | null>(null)
  const loading = ref(false)
  // 导入到当前项目时 currentProjectId 值不变，watch 不触发。
  // 通过递增此计数器通知 Sidebar 强制重载侧边栏数据。
  const sidebarReloadTick = ref(0)

  // 暂停 watch 写入持久化的标志位。
  // 用于 restoreLastProject() 期间：恢复时设置 currentProjectId 不应回写到 store。
  const _pauseAutoPersist = ref(false)

  const currentProject = computed(() =>
    projects.value.find(p => p.id === currentProjectId.value) ?? null
  )

  // ── 持久化 watch ──────────────────────────────────────────
  // 任何 currentProjectId 变更（用户切换、新建项目、删除当前项目回退）都会触发。
  // _pauseAutoPersist=true 时跳过（恢复阶段，避免无意义回写）。
  // 写入失败已在 _persistedSettings.ts 内部 warn 兜底，不影响业务。
  watch(currentProjectId, (newId) => {
    if (_pauseAutoPersist.value) return
    if (newId === null) {
      void deleteSetting(LAST_OPENED_PROJECT_KEY)
    } else {
      void writeSetting(LAST_OPENED_PROJECT_KEY, newId)
    }
  })

  /** 首次安装播种示例数据：仅走「无任何项目 → 自动建 Default」分支，老用户升级不补种。
   *  播种在后台跑（不 await），失败仅 warn，绝不阻塞首启。 */
  async function seedSampleData(defaultProject: Project): Promise<void> {
    try {
      const col = await invoke<{ id: number }>('create_collection', {
        projectId: defaultProject.id,
        name: '示例集合',
      })
      const getReq = await invoke<{ id: number }>('create_request', {
        collectionId: col.id, name: '示例：HTTPBin GET', method: 'GET', url: 'https://httpbin.org/get',
      })
      const postReq = await invoke<{ id: number }>('create_request', {
        collectionId: col.id, name: '示例：HTTPBin POST', method: 'POST', url: 'https://httpbin.org/post',
      })
      // POST 示例带 JSON body（update_request 为全量覆写，CLAUDE.md 写操作真源节）
      // update_request 为全量覆写且 IPC 参数全为必填（request.rs:92-104），缺一即被 Tauri reject
      await invoke('update_request', {
        id: postReq.id, name: '示例：HTTPBin POST', method: 'POST', url: 'https://httpbin.org/post',
        params: '[]', headers: '[]', bodyType: 'json', body: '{"name":"ApiCat"}',
        authType: 'none', authConfig: '{}',
      })
      const caseResp = await invoke<{ id: number }>('create_test_case', {
        requestId: getReq.id, collectionId: col.id,
        name: '示例：校验返回 200', method: 'GET', url: 'https://httpbin.org/get', caseType: 'happy_path',
      })
      // update_test_case 仅 case_type/assertions 做 COALESCE，method/url 会直接覆写，必须回传
      await invoke('update_test_case', {
        id: caseResp.id, name: '示例：校验返回 200',
        method: 'GET', url: 'https://httpbin.org/get',
        assertions: JSON.stringify([{ type: 'status_code', operator: 'eq', expected: '200' }]),
      })
      // 播种是后台异步，Sidebar 树早已按空集合加载完毕；递增 tick 强制刷新（同 ImportDialog 模式）
      sidebarReloadTick.value++
    } catch (e) {
      console.warn('[seed] 示例数据播种失败:', e)
    }
  }
  async function loadProjects() {
    loading.value = true
    try {
      let list = await invoke<Project[]>('list_projects')

      // 如果没有任何项目，自动创建一个默认项目
      if (list.length === 0) {
        const defaultProj = await invoke<Project>('create_project', { name: 'Default Project', description: null })
        list = [defaultProj]
        void seedSampleData(defaultProj)
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

  /**
   * 启动恢复：读取上次打开的项目 id，若仍存在则切换到该项目。
   * 必须在 loadProjects() 之后调用，否则 projects[] 还是空的没法校验。
   *
   * 兜底场景：
   *   - 首次启动 / store 文件不存在 → readSetting 返回 null → 维持 loadProjects 已选的默认值
   *   - 上次的项目已被删除 → 校验不通过 → 清理脏 key，维持默认值
   *   - plugin-store IPC 失败 → readSetting 内部 warn，返回 null，行为同首次启动
   */
  async function restoreLastProject() {
    _pauseAutoPersist.value = true
    try {
      const id = await readSetting<number>(LAST_OPENED_PROJECT_KEY)
      if (id === null) return
      if (projects.value.some(p => p.id === id)) {
        currentProjectId.value = id
      } else {
        // 项目已被删除，清理脏数据
        await deleteSetting(LAST_OPENED_PROJECT_KEY)
      }
    } finally {
      _pauseAutoPersist.value = false
    }
  }

  /**
   * 启动序列：加载项目列表 → 恢复上次打开的项目。App 启动时必须走这个入口。
   *
   * 整体暂停持久化是必须的：loadProjects() 会给 currentProjectId 兜底选中 projects[0]
   * （list_projects 按 created_at DESC 排序 → 即创建时间最新的项目），watch 随即把它写进
   * lastOpenedProjectId；restoreLastProject() 紧接着读同一个 key，读到的就是刚写进去的值 ——
   * 「记住上次打开的项目」于是恒等于「打开最新创建的项目」。
   */
  async function initProjects() {
    _pauseAutoPersist.value = true
    try {
      await loadProjects()
      await restoreLastProject()
    } finally {
      _pauseAutoPersist.value = false
    }
  }

  async function createProject(name: string, description?: string) {
    const project = await invoke<Project>('create_project', { name, description: description ?? null })
    projects.value.unshift(project)
    currentProjectId.value = project.id
    return project
  }

  async function updateProject(id: number, name: string, description?: string, docsOutputDir?: string | null) {
    // docsOutputDir 未传时保留当前值（TopBar 重命名等场景不该清掉文档目录配置）
    const current = projects.value.find(p => p.id === id)
    const dir = docsOutputDir !== undefined ? docsOutputDir : (current?.docs_output_dir ?? null)
    const updated = await invoke<Project>('update_project', {
      id, name, description: description ?? null, docsOutputDir: dir,
    })
    const idx = projects.value.findIndex(p => p.id === id)
    if (idx !== -1) projects.value[idx] = updated
    return updated
  }

  async function deleteProject(id: number) {
    await invoke('delete_project', { id })
    projects.value = projects.value.filter(p => p.id !== id)
    if (currentProjectId.value === id) {
      // watch 会自动写入新值或 deleteSetting（newId === null 时）
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
    restoreLastProject,
    initProjects,
    createProject,
    updateProject,
    deleteProject,
  }
})
