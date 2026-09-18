<template>
  <aside class="sidebar">
    <!-- 搜索框 + 刷新按钮 -->
    <div class="sidebar__search">
      <n-input v-model:value="searchText" placeholder="搜索接口..." size="small" clearable>
        <template #prefix>🔍</template>
      </n-input>
      <n-button
        size="small"
        quaternary
        :loading="loading"
        :disabled="!currentProjectId"
        title="刷新目录"
        style="flex-shrink: 0; margin-left: 4px;"
        @click="reloadCurrentProject"
      >↺</n-button>
    </div>

    <!-- 树区域 -->
    <div class="sidebar__tree" @click.self="closeMenu">
      <n-empty v-if="!currentProjectId" description="请先选择或创建项目" size="small" style="margin-top: 40px" />
      <n-spin v-else-if="loading" size="small" style="margin-top: 40px; display:flex; justify-content:center" />
      <n-tree
        v-else
        :data="treeData"
        :node-props="nodeProps"
        :render-suffix="renderSuffix"
        :render-label="renderLabel"
        :allow-drop="allowDrop"
        block-line
        draggable
        expand-on-click
        :default-expanded-keys="expandedKeys"
        @update:selected-keys="onSelectNode"
        @drop="onDrop"
      />
    </div>

    <!-- 底部操作 -->
    <div class="sidebar__footer">
      <n-button size="small" block dashed @click="openNewCollectionDialog()">
        + 新建文件夹
      </n-button>
      <n-button size="small" block dashed style="margin-top:4px" @click="openNewRequestDialog()">
        + 新建接口
      </n-button>
    </div>

    <!-- 右键浮层菜单 -->
    <Teleport to="body">
      <div
        v-if="menuVisible"
        class="ctx-menu"
        :style="{ top: menuY + 'px', left: menuX + 'px' }"
        @click.stop
      >
        <template v-if="menuNodeType === 'request'">
          <div class="ctx-item" @click="startRename">✏️ 重命名</div>
          <div class="ctx-item" @click="duplicateItem">📋 复制接口</div>
          <div class="ctx-item" @click="copyAsCurl">🔗 复制为 cURL</div>
          <div class="ctx-item ctx-item--danger" @click="deleteItem">🗑️ 删除</div>
        </template>
        <template v-else-if="menuNodeType === 'collection'">
          <div class="ctx-item" @click="startRename">✏️ 重命名</div>
          <div class="ctx-item ctx-item--danger" @click="deleteItem">🗑️ 删除文件夹</div>
        </template>
      </div>
    </Teleport>

    <!-- 新建文件夹对话框 -->
    <n-modal v-model:show="showNewCollectionDialog" preset="dialog" title="新建文件夹">
      <n-input v-model:value="newCollectionName" placeholder="文件夹名称" @keyup.enter="createCollection" />
      <template #action>
        <n-button @click="showNewCollectionDialog = false">取消</n-button>
        <n-button type="primary" :loading="creating" @click="createCollection">创建</n-button>
      </template>
    </n-modal>

    <!-- 重命名对话框 -->
    <n-modal v-model:show="showRenameDialog" preset="dialog" title="重命名">
      <n-input v-model:value="renameValue" placeholder="新名称" @keyup.enter="commitRename" />
      <template #action>
        <n-button @click="showRenameDialog = false">取消</n-button>
        <n-button type="primary" @click="commitRename">确定</n-button>
      </template>
    </n-modal>

    <!-- 新建接口对话框 -->
    <n-modal v-model:show="showNewRequestDialog" preset="dialog" title="新建接口">
      <n-space vertical>
        <n-select v-model:value="newRequestMethod" :options="methodOptions" size="small" style="width:100px" />
        <n-input v-model:value="newRequestUrl" placeholder="输入 URL，如 https://api.example.com/users/1" @keyup.enter="doCreateRequest" />
        <n-input v-model:value="newRequestName" placeholder="接口名（留空自动生成）" />
      </n-space>
      <template #action>
        <n-button @click="showNewRequestDialog = false">取消</n-button>
        <n-button type="primary" :loading="creating" @click="doCreateRequest">创建</n-button>
      </template>
    </n-modal>

    <!-- 删除确认对话框 -->
    <n-modal v-model:show="showDeleteConfirmDialog" preset="dialog" title="确认删除" :show-icon="false">
      <div style="font-size:14px; line-height:1.7; color:var(--text-primary)">
        确定要删除 <strong>「{{ deleteTargetName }}」</strong> 吗？<br>
        <span v-if="deleteTargetIsCollection" style="color:var(--color-error); font-size:12px">该文件夹及其包含的所有接口都将被删除，此操作不可撤销！</span>
        <span v-else style="color:var(--color-error); font-size:12px">此操作不可撤销！</span>
      </div>
      <template #action>
        <n-button @click="showDeleteConfirmDialog = false">取消</n-button>
        <n-button type="error" :loading="deleting" @click="confirmDelete">删除</n-button>
      </template>
    </n-modal>

    <!-- cURL 导入对话框 -->
    <n-modal v-model:show="showCurlImportDialog" preset="dialog" title="从 cURL 导入" style="width: 500px">
      <n-input
        v-model:value="curlImportText"
        type="textarea"
        :rows="8"
        placeholder="在此粘贴 cURL 文本..."
      />
      <template #action>
        <n-button @click="showCurlImportDialog = false">取消</n-button>
        <n-button type="primary" :loading="creating" @click="doImportCurl">导入</n-button>
      </template>
    </n-modal>
  </aside>
</template>

<script setup lang="ts">
import { h, ref, computed, watch, onMounted, onUnmounted, defineComponent } from 'vue'
import { NInput, NEmpty, NButton, NTree, NSpin, NModal, NSpace, NSelect, NTag, NDropdown, useMessage } from 'naive-ui'
import type { TreeOption } from 'naive-ui'
import { useProjectStore } from '../../stores/project'
import { useCollectionStore } from '../../stores/collection'
import { useRequestStore } from '../../stores/request'
import { useTabStore } from '../../stores/tab'
import { useTestCaseStore } from '../../stores/testCase'
import { useEnvironmentStore } from '../../stores/environment'
import { parseUrl, resolveEffectiveUrl, hasUnresolvedPlaceholder } from '../../utils/urlParser'
import { buildCurl } from '../../utils/curlBuilder'
import { copyText } from '../../utils/clipboard'
import { useCollectionTreeDnd } from '../../composables/useCollectionTreeDnd'
import type { ApiRequest, Collection, ParamItem } from '../../types'

const projectStore = useProjectStore()
const collectionStore = useCollectionStore()
const requestStore = useRequestStore()
const tabStore = useTabStore()
const testCaseStore = useTestCaseStore()
const envStore = useEnvironmentStore()
const message = useMessage()

/**
 * NodeStatusDot — 一个真正的 Vue 组件，有独立的 setup 响应式上下文。
 * 这是解决圆点不更新的根本方案：
 * - renderLabel/renderSuffix 是普通 prop 函数，在 Naive UI 内部调用时不在 Vue 响应式 Effect 里
 * - 只有 defineComponent + setup 才有自己的 effect scope，能正确追踪 Pinia store 变化
 * 使用 inline style 避免 scoped CSS 哈希匹配问题（NodeStatusDot 的 DOM 没有 Sidebar 的 scope 哈希）
 */
const NodeStatusDot = defineComponent({
  props: {
    reqId: { type: Number, required: true },
  },
  setup(props) {
    // 这里的 computed 有独立的响应式上下文，能正确追踪 dirtyRequestIds/savedRequestIds 变化
    const status = computed(() => {
      const dirty = requestStore.dirtyRequestIds
      const saved = requestStore.savedRequestIds
      if (dirty.has(props.reqId)) return 'dirty'
      if (saved.has(props.reqId)) return 'saved'
      return null
    })
    return () => {
      const s = status.value
      if (!s) return null
      // 用 inline style，不依赖 scoped CSS 哈希
      const baseStyle = 'display:inline-block;width:7px;height:7px;border-radius:50%;flex-shrink:0;vertical-align:middle;position:relative;top:-1px;'
      // dirty: 警告色（橙）；saved: 主色（绿）
      const colorStyle = s === 'dirty'
        ? 'background:var(--color-warning);box-shadow:0 0 0 2px rgba(240,160,32,0.25);'
        : 'background:var(--color-primary);box-shadow:0 0 0 2px var(--color-primary-soft);'
      return h('span', {
        style: baseStyle + colorStyle,
        title: s === 'dirty' ? '有未保存的修改' : '已保存',
      })
    }
  },
})

const searchText = ref('')
const loading = ref(false)
const creating = ref(false)
const expandedKeys = ref<string[]>([])

// 搜索时自动展开所有文件夹，以便显示匹配的接口
watch(searchText, (keyword) => {
  if (!keyword.trim()) return
  const pid = currentProjectId.value
  if (!pid) return
  const cols = collectionStore.getCollections(pid)
  expandedKeys.value = cols.map(c => `col-${c.id}`)
})

// 对话框状态
const showNewCollectionDialog = ref(false)
const newCollectionName = ref('')
const parentCollectionId = ref<number | null>(null)
const showNewRequestDialog = ref(false)
const newRequestUrl = ref('')
const newRequestName = ref('')
const newRequestMethod = ref('GET')
const targetCollectionId = ref<number | null>(null)

const showCurlImportDialog = ref(false)
const curlImportText = ref('')

const methodOptions = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'].map(m => ({ label: m, value: m }))

const currentProjectId = computed(() => projectStore.currentProjectId)

// ── 右键菜单状态 ─────────────────────────────────────────
const menuVisible = ref(false)
const menuX = ref(0)
const menuY = ref(0)
const menuNodeType = ref<'request' | 'collection' | null>(null)
// 当前菜单目标节点的 key（"req-{id}" 或 "col-{id}"）
const menuNodeKey = ref<string>('')

function openMenu(e: MouseEvent, key: string, type: 'request' | 'collection') {
  menuNodeKey.value = key
  menuNodeType.value = type
  menuX.value = e.clientX
  menuY.value = e.clientY
  menuVisible.value = true
}

function closeMenu() {
  menuVisible.value = false
}

// 点击页面其他地方关闭菜单
function onDocClick() {
  if (menuVisible.value) closeMenu()
}
onMounted(() => document.addEventListener('click', onDocClick))
onUnmounted(() => document.removeEventListener('click', onDocClick))

// ── 重命名对话框状态 ──────────────────────────────────────
const showRenameDialog = ref(false)
const renameValue = ref('')
// renamingKey 存储当前重命名目标的节点 key
const renamingKey = ref<string>('')

function startRename() {
  closeMenu()
  const key = menuNodeKey.value
  renamingKey.value = key
  // 获取当前名称填入
  if (key.startsWith('req-')) {
    const id = parseInt(key.replace('req-', ''))
    const req = Object.values(requestStore.requestMap).flat().find(r => r.id === id)
    renameValue.value = req?.name ?? ''
  } else {
    const id = parseInt(key.replace('col-', ''))
    const pid = currentProjectId.value
    if (pid) {
      const col = collectionStore.getCollections(pid).find(c => c.id === id)
      renameValue.value = col?.name ?? ''
    }
  }
  showRenameDialog.value = true
}

async function commitRename() {
  showRenameDialog.value = false
  const key = renamingKey.value
  const name = renameValue.value.trim()
  if (!name || !key) return

  try {
    if (key.startsWith('req-')) {
      const id = parseInt(key.replace('req-', ''))
      const updated = await requestStore.renameRequest(id, name)
      // 同步 Tab 标题（若该接口已打开）
      tabStore.updateTabTitle(id, updated.method, name)
      message.success('重命名成功')
    } else {
      const id = parseInt(key.replace('col-', ''))
      const pid = currentProjectId.value
      if (pid) {
        await collectionStore.renameCollection(id, name, pid)
        message.success('重命名成功')
      }
    }
  } catch (e) {
    message.error(String(e))
  }
}

// ── 复制接口 ─────────────────────────────────────────────
async function duplicateItem() {
  closeMenu()
  const key = menuNodeKey.value
  if (!key.startsWith('req-')) return
  const id = parseInt(key.replace('req-', ''))
  try {
    await requestStore.duplicateRequest(id)
    message.success('接口已复制')
  } catch (e) {
    message.error(String(e))
  }
}

// ── 复制为 cURL ──────────────────────────────────────────
async function copyAsCurl() {
  closeMenu()
  const key = menuNodeKey.value
  if (!key.startsWith('req-')) return
  const id = parseInt(key.replace('req-', ''))
  const req = Object.values(requestStore.requestMap).flat().find(r => r.id === id)
  if (!req) return

  const headersArr = (() => { try { return JSON.parse(req.headers) } catch { return [] } })()
  const queryArr = (() => { try { return JSON.parse(req.params) } catch { return [] } })()

  // 与 MainPanel.effectiveUrl 共享同一套拼接规则：
  // 相对路径 + 激活环境有 base_url → 拼接为完整 URL；否则原样
  const fullUrl = resolveEffectiveUrl(req.url, envStore.activeEnv?.base_url)

  const curl = buildCurl({
    method: req.method,
    url: fullUrl,
    queryParams: queryArr,
    headers: headersArr,
    bodyType: req.body_type,
    body: req.body,
    authType: req.auth_type,
    authConfig: req.auth_config,
  })

  await copyText(curl)

  // 侧边栏走 DB URL —— 若 DB 存的 URL 含未替换占位符（:id / {id}），
  // 粘贴到终端会请求到错误地址。提示用户可改用编辑区的 "📋 cURL" 填值后复制。
  if (hasUnresolvedPlaceholder(fullUrl)) {
    message.warning('cURL 含未替换的占位符（:xxx / {xxx}）；在编辑区填值后点 "📋 cURL" 可获得替换后的命令')
  } else {
    message.success('cURL 已复制到剪贴板')
  }
}

// ── 删除节点 ─────────────────────────────────────────────
// 删除确认对话框状态
const showDeleteConfirmDialog = ref(false)
const deleteTargetName = ref('')
const deleteTargetIsCollection = ref(false)
const deleteTargetKey = ref('')
const deleting = ref(false)

async function deleteItem() {
  closeMenu()
  await promptDeleteByKey(menuNodeKey.value)
}

/** 展示删除确认弹窗 */
async function promptDeleteByKey(key: string) {
  const pid = currentProjectId.value
  if (!pid || !key) return

  if (key.startsWith('req-')) {
    const id = parseInt(key.replace('req-', ''))
    const req = Object.values(requestStore.requestMap).flat().find(r => r.id === id)
    if (!req) return
    deleteTargetName.value = req.name
    deleteTargetIsCollection.value = false
  } else {
    const id = parseInt(key.replace('col-', ''))
    const col = collectionStore.getCollections(pid).find(c => c.id === id)
    if (!col) return
    deleteTargetName.value = col.name
    deleteTargetIsCollection.value = true
  }

  deleteTargetKey.value = key
  showDeleteConfirmDialog.value = true
}

/** 确认删除后执行 */
async function confirmDelete() {
  deleting.value = true
  try {
    await performDeleteByKey(deleteTargetKey.value)
    showDeleteConfirmDialog.value = false
  } finally {
    deleting.value = false
  }
}

async function performDeleteByKey(key: string) {
  const pid = currentProjectId.value
  if (!pid || !key) return

  try {
    if (key.startsWith('req-')) {
      const id = parseInt(key.replace('req-', ''))
      const req = Object.values(requestStore.requestMap).flat().find(r => r.id === id)
      if (!req) return
      await requestStore.deleteRequest(id, req.collection_id)
      // 接口删除后同步关闭对应 Tab（若已打开）
      tabStore.closeTab(id)
      message.success('接口已删除')
    } else {
      const id = parseInt(key.replace('col-', ''))
      const col = collectionStore.getCollections(pid).find(c => c.id === id)
      if (!col) return
      await collectionStore.deleteCollection(id, pid)
      // 同步清理 requestStore 中该 collection 及其所有子 collection 的缓存
      // DB 层 ON DELETE CASCADE 已处理数据，这里只清理内存
      const allCols = collectionStore.getCollections(pid)
      const toDelete = new Set<number>([id])
      // BFS 找出所有子 collection
      let changed = true
      while (changed) {
        changed = false
        for (const c of allCols) {
          if (c.parent_id !== null && toDelete.has(c.parent_id) && !toDelete.has(c.id)) {
            toDelete.add(c.id)
            changed = true
          }
        }
      }
      for (const cid of toDelete) {
        // 关闭该 collection 下所有已打开的 Tab
        for (const req of requestStore.requestMap[cid] ?? []) {
          tabStore.closeTab(req.id)
        }
        delete requestStore.requestMap[cid]
      }
      message.success('文件夹已删除')
    }
  } catch (e) {
    message.error(String(e))
  }
}

const requestActionOptions = [
  { label: '重命名', key: 'rename' },
  { label: '复制接口', key: 'duplicate' },
  { label: '复制为 cURL', key: 'curl' },
  { label: '删除', key: 'delete' },
]

const collectionAddOptions = [
  { label: '添加接口到此文件夹', key: 'add-request' },
  { label: '添加子文件夹', key: 'add-sub-collection' },
  { label: '📋 从 cURL 导入', key: 'import-curl' },
]

const collectionActionOptions = [
  { label: '重命名', key: 'rename' },
  { label: '删除文件夹', key: 'delete' },
]

function openNewCollectionDialog(parentId: number | null = null) {
  parentCollectionId.value = parentId
  showNewCollectionDialog.value = true
}

function openNewRequestDialog(collectionId: number | null = null) {
  targetCollectionId.value = collectionId
  showNewRequestDialog.value = true
}

function renderLabel(info: { option: TreeOption }) {
  const label = String(info.option.label ?? '')
  // 1.0.4：request 节点 label 形如 "GET 登录" —— 拆出方法渲染为彩色 NTag
  const m = label.match(/^(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)\s+(.+)$/)
  if (m && String(info.option.key ?? '').startsWith('req-')) {
    return h('span', { class: 'node-label' }, [
      h(NTag, {
        size: 'small',
        bordered: false,
        style: {
          marginRight: '6px',
          color: `var(--method-${m[1].toLowerCase()})`,
        },
      }, { default: () => m[1] }),
      h('span', null, m[2]),
    ])
  }
  // 圆点已移至 renderSuffix（按钮左侧），此处只渲染 label
  return h('span', { class: 'node-label' }, label)
}


function renderSuffix(info: { option: TreeOption; checked: boolean; selected: boolean }) {
  const key = String(info.option.key ?? '')

  if (key.startsWith('req-')) {
    const reqId = parseInt(key.replace('req-', ''))
    // suffix 容器：圆点（始终可见） + 操作按钮（hover 才显示）
    return h('div', { class: 'node-suffix', onClick: (e: MouseEvent) => e.stopPropagation() }, [
      // 圆点：NodeStatusDot 有独立响应式上下文，始终渲染（不受 node-actions opacity 影响）
      h(NodeStatusDot, { reqId }),
      // 操作按钮组（hover 时由 DOM style 控制 opacity）
      h(
        'div',
        { class: 'node-actions' },
        [
          h(
            NDropdown,
            {
              trigger: 'click',
              options: requestActionOptions,
              onSelect: async (actionKey: string | number) => {
                menuNodeKey.value = key
                menuNodeType.value = 'request'
                if (actionKey === 'rename') {
                  startRename()
                } else if (actionKey === 'duplicate') {
                  await duplicateItem()
                } else if (actionKey === 'curl') {
                  await copyAsCurl()
                } else if (actionKey === 'delete') {
                  await promptDeleteByKey(key)
                }
              },
            },
            {
              default: () => h('button', { class: 'node-action-btn', type: 'button', title: '更多操作' }, '•••'),
            },
          ),
        ],
      ),
    ])
  }

  if (key.startsWith('col-')) {
    const colId = parseInt(key.replace('col-', ''))
    return h('div', { class: 'node-suffix', onClick: (e: MouseEvent) => e.stopPropagation() }, [
      // 操作按钮组（hover 时显示）
      h(
        'div',
        { class: 'node-actions' },
        [
          // + 新建子项
          h(
            NDropdown,
            {
              trigger: 'click',
              options: collectionAddOptions,
              onSelect: (actionKey: string | number) => {
                if (actionKey === 'add-sub-collection') {
                  openNewCollectionDialog(colId)
                } else if (actionKey === 'add-request') {
                  openNewRequestDialog(colId)
                } else if (actionKey === 'import-curl') {
                  targetCollectionId.value = colId
                  curlImportText.value = ''
                  showCurlImportDialog.value = true
                }
              },
            },
            {
              default: () => h('button', { class: 'node-action-btn node-action-btn--add', type: 'button', title: '添加' }, '+'),
            },
          ),
          // ••• 文件夹操作下拉
          h(
            NDropdown,
            {
              trigger: 'click',
              options: collectionActionOptions,
              onSelect: async (actionKey: string | number) => {
                menuNodeKey.value = key
                menuNodeType.value = 'collection'
                if (actionKey === 'rename') {
                  startRename()
                } else if (actionKey === 'delete') {
                  await promptDeleteByKey(key)
                }
              },
            },
            {
              default: () => h('button', { class: 'node-action-btn', type: 'button', title: '更多操作' }, '•••'),
            },
          ),
        ],
      ),
    ])
  }

  return null
}

// ── 项目切换时加载数据，并保存/恢复 Tab 状态 ────────────────
// loadSeq：每次触发 watch 时递增，异步回调结束时对比，过期回调不更新 loading
let loadSeq = 0

watch(currentProjectId, async (pid, oldPid) => {
  if (!pid) return
  const seq = ++loadSeq
  loading.value = true

  try {
    // 切换前：保存旧项目的 Tab 状态（immediate 首次触发时 oldPid 为 undefined，跳过）
    // saveState 失败（如 store 权限问题）不阻断后续数据加载，单独 catch
    if (oldPid) {
      try {
        await tabStore.saveState(oldPid)
      } catch (e) {
        console.warn('[Sidebar] saveState failed, continuing:', e)
      }
    }
    // 如果在 saveState 期间又触发了新的 watch，当前回调已过期，直接退出
    if (seq !== loadSeq) return
    tabStore.clearTabs()

    // 加载新项目数据
    await collectionStore.loadCollections(pid)
    await Promise.all(collectionStore.getCollections(pid).map(c => requestStore.loadRequests(c.id)))

    // 再次检查，只有最新的回调才更新 UI 状态
    if (seq !== loadSeq) return
    const cols = collectionStore.getCollections(pid)
    expandedKeys.value = cols.slice(0, 3).map(c => `col-${c.id}`)

    // 恢复新项目的 Tab 状态，失败不阻断 UI 渲染
    try {
      await tabStore.restoreState(pid, requestStore.requestMap)
    } catch (e) {
      console.warn('[Sidebar] restoreState failed:', e)
    }
  } finally {
    // 只有最新的回调才能把 loading 置回 false
    if (seq === loadSeq) {
      loading.value = false
    }
  }
}, { immediate: true })

// ── 导入后强制刷新当前项目数据 ───────────────────────────────
// 场景：导入到"当前项目"时 currentProjectId 值不变，watch 不会触发，
// 需要额外监听一个专用信号来强制重载侧边栏数据
async function reloadCurrentProject() {
  const pid = currentProjectId.value
  if (!pid) return
  const seq = ++loadSeq
  loading.value = true
  try {
    await collectionStore.loadCollections(pid)
    await Promise.all(collectionStore.getCollections(pid).map(c => requestStore.loadRequests(c.id)))
    if (seq !== loadSeq) return
    const cols = collectionStore.getCollections(pid)
    expandedKeys.value = cols.slice(0, 3).map(c => `col-${c.id}`)
  } finally {
    if (seq === loadSeq) {
      loading.value = false
    }
  }
}

watch(() => projectStore.sidebarReloadTick, (tick) => {
  if (tick > 0) reloadCurrentProject()
})

// ── 构建 NTree 数据 ───────────────────────────────────────
const treeData = computed<TreeOption[]>(() => {
  const pid = currentProjectId.value
  if (!pid) return []
  const cols = collectionStore.getCollections(pid)

  const rootCols = cols.filter(c => c.parent_id === null)
  const childMap: Record<number, Collection[]> = {}
  for (const c of cols) {
    if (c.parent_id !== null) {
      childMap[c.parent_id] = childMap[c.parent_id] ?? []
      childMap[c.parent_id].push(c)
    }
  }

  const keyword = searchText.value.trim().toLowerCase()

  function buildCollectionNode(col: Collection): TreeOption | null {
    const reqs = (requestStore.requestMap[col.id] ?? [])
      .filter(r => {
        if (!keyword) return true
        const label = `${r.method} ${r.name}`.toLowerCase()
        return label.includes(keyword)
      })
      .map(r => ({
        key: `req-${r.id}`,
        label: `${r.method} ${r.name}`,
        isLeaf: true,
        data: r,
      } as TreeOption))

    const subCols = (childMap[col.id] ?? [])
      .map(buildCollectionNode)
      .filter((n): n is TreeOption => n !== null)

    // 搜索时：若该文件夹无匹配的接口和子文件夹，则隐藏该文件夹
    if (keyword && reqs.length === 0 && subCols.length === 0) return null

    return {
      key: `col-${col.id}`,
      label: col.name,
      children: [...subCols, ...reqs],
      data: col,
    }
  }

  return rootCols
    .map(buildCollectionNode)
    .filter((n): n is TreeOption => n !== null)
})

// ── 节点点击 → 通过 tabStore 打开接口 Tab ────────────────────
function onSelectNode(keys: Array<string | number>) {
  const key = keys[0] as string
  if (!key?.startsWith('req-')) return
  const id = parseInt(key.replace('req-', ''))
  const req = Object.values(requestStore.requestMap).flat().find(r => r.id === id)
  if (!req) return
  tabStore.openTab(req)
}

// 组件 mount 完成后才激活 hover 事件，防止 WebView 初始化期间误触发
const hoverEnabled = ref(false)
onMounted(() => {
  // 延迟 300ms 确保 WebView 完全初始化
  setTimeout(() => { hoverEnabled.value = true }, 300)
})

// ── 节点 props（右键菜单 + hover 状态追踪） ───────────────
function nodeProps({ option }: { option: TreeOption }) {
  const key = option.key as string
  return {
    onContextmenu(e: MouseEvent) {
      e.preventDefault()
      e.stopPropagation()
      if (key.startsWith('req-')) {
        openMenu(e, key, 'request')
      } else if (key.startsWith('col-')) {
        openMenu(e, key, 'collection')
      }
    },
    onMouseenter(e: MouseEvent) {
      if (!hoverEnabled.value) return
      const nodeEl = e.currentTarget as HTMLElement
      nodeEl.querySelectorAll<HTMLElement>('.node-actions').forEach(el => {
        el.style.opacity = '1'
        el.style.pointerEvents = 'auto'
      })
    },
    onMouseleave(e: MouseEvent) {
      if (!hoverEnabled.value) return
      const nodeEl = e.currentTarget as HTMLElement
      nodeEl.querySelectorAll<HTMLElement>('.node-actions').forEach(el => {
        el.style.opacity = '0'
        el.style.pointerEvents = 'none'
      })
    },
  }
}

// ── 拖拽排序 / 移动 ───────────────────────────────────────
// 逻辑在 useCollectionTreeDnd（只依赖 store + IPC，与模板零耦合）
const { allowDrop, onDrop } = useCollectionTreeDnd(currentProjectId)

// ── 创建文件夹 ────────────────────────────────────────────
async function createCollection() {
  const pid = currentProjectId.value
  if (!pid || !newCollectionName.value.trim()) return
  creating.value = true
  try {
    const col = await collectionStore.createCollection(
      pid,
      newCollectionName.value.trim(),
      parentCollectionId.value ?? undefined,
    )
    showNewCollectionDialog.value = false
    newCollectionName.value = ''
    parentCollectionId.value = null
    requestStore.requestMap[col.id] = []
    message.success('文件夹创建成功')
  } catch (e) {
    message.error(String(e))
  } finally {
    creating.value = false
  }
}

// ── 创建接口 ──────────────────────────────────────────────

async function doCreateRequest() {
  const pid = currentProjectId.value
  if (!pid) { message.error('请先选择项目'); return }
  const cols = collectionStore.getCollections(pid)
  if (cols.length === 0) { message.error('请先创建文件夹'); return }

  const url = newRequestUrl.value.trim()
  if (!url) { message.error('请输入 URL'); return }

  const parsed = parseUrl(url, newRequestMethod.value)
  const name = newRequestName.value.trim() || parsed.displayName
  const targetCid = targetCollectionId.value ?? cols[0].id

  // 检查 Collection 内是否已有同名接口
  const existingReqs = requestStore.requestMap[targetCid] ?? []
  const hasDuplicate = existingReqs.some(r => r.name === name)

  if (hasDuplicate) {
    // 1.0.4：不建第二个接口 —— 在已有接口下新增用例（实例 = 用例）
    const existing = existingReqs.find(r => r.name === name)
    if (existing) {
      await createCaseUnderExisting(existing, newRequestMethod.value, url)
    }
    return
  }

  await executeCreateRequest(targetCid, name, newRequestMethod.value, url)
}

/**
 * 1.0.4：同名时在已有接口下新增用例（实例 = 用例，不新建接口）
 * create_test_case 后端对空 name 自动命名「用例 N」，故传空串
 */
async function createCaseUnderExisting(existing: ApiRequest, method: string, url: string) {
  creating.value = true
  try {
    const tc = await testCaseStore.createTestCase({
      requestId: existing.id,
      collectionId: existing.collection_id,
      name: '',
      method,
      url,
      headers: '[]',
      params_: '[]',
      bodyType: null,
      body: null,
    })
    showNewRequestDialog.value = false
    newRequestUrl.value = ''
    newRequestName.value = ''
    newRequestMethod.value = 'GET'
    targetCollectionId.value = null
    message.success(`已将参数新增为「${existing.name}」的用例「${tc.name}」`)
    // 打开已有接口，方便查看新用例
    tabStore.openTab(existing)
  } catch (e) {
    message.error(String(e))
  } finally {
    creating.value = false
  }
}

/** 实际执行创建接口并重置表单 */
async function executeCreateRequest(collectionId: number, name: string, method: string, url: string) {
  creating.value = true
  try {
    const req = await requestStore.createRequest(collectionId, name, method, url)
    // 新建接口后自动在 TabBar 打开
    tabStore.openTab(req)
    showNewRequestDialog.value = false
    newRequestUrl.value = ''
    newRequestName.value = ''
    newRequestMethod.value = 'GET'
    targetCollectionId.value = null
    message.success('接口创建成功')
  } catch (e) {
    message.error(String(e))
  } finally {
    creating.value = false
  }
}

// cURL 解析与导入
function parseCurl(curlText: string): { method: string; url: string; headers: ParamItem[]; bodyType: string; body: string } {
  const text = curlText.trim().replace(/\s*\\\s*\n\s*/g, ' ') // join line continuations
  const urlMatch = text.match(/https?:\/\/[^\s'"]+/)
  const url = urlMatch ? urlMatch[0].replace(/['"]/g, '') : ''
  const methodMatch = text.match(/-X\s+([A-Z]+)/i)
  const method = methodMatch ? methodMatch[1].toUpperCase() : (text.includes('-d ') || text.includes('--data') || text.includes('--data-urlencode') ? 'POST' : 'GET')
  const headers: ParamItem[] = []
  const headerRe = /-H\s+['"]([^'"]+)['"]/g
  let hm: RegExpExecArray | null
  while ((hm = headerRe.exec(text)) !== null) {
    const colonIdx = hm[1].indexOf(':')
    if (colonIdx > 0) {
      headers.push({ key: hm[1].substring(0, colonIdx).trim(), value: hm[1].substring(colonIdx + 1).trim(), enabled: true })
    }
  }
  // 匹配 --data / -d / --data-binary / --data-raw / --data-urlencode（curl 的 data 参数全家族）
  // 用反向引用 \1 配对同一引号，body 内含双引号也不截断（如 --data-raw '{"a":1}' 的 JSON）
  const dataMatch =
    text.match(/(?:--data(?:-binary|-raw|-urlencode)?|-d)\s+(["'])([\s\S]*?)\1/s) ||
    text.match(/(?:--data(?:-binary|-raw|-urlencode)?|-d)\s+\$(["'])([\s\S]*?)\1/s)

  let body = ''
  if (dataMatch) {
    const rawValue = dataMatch[2]  // [1] 是引号字符，[2] 才是 body 内容（反向引用正则）
    // --data-urlencode：值已经 URL 编码，解码后作为 body
    if (text.includes('--data-urlencode')) {
      try {
        body = decodeURIComponent(rawValue)
      } catch {
        body = rawValue
      }
    } else {
      body = rawValue
    }
  }
  let bodyType = 'none'
  if (body) {
    const ct = headers.find(h => h.key.toLowerCase() === 'content-type')?.value ?? ''
    if (ct.includes('json')) bodyType = 'raw_json'
    else if (ct.includes('form') && ct.includes('urlencoded')) bodyType = 'form_urlencoded'
    else bodyType = 'raw_text'
  }
  return { method, url, headers, bodyType, body }
}

async function doImportCurl() {
  const pid = currentProjectId.value
  if (!pid || targetCollectionId.value === null) return
  if (!curlImportText.value.trim()) {
    message.warning('请输入 cURL 内容')
    return
  }
  creating.value = true
  try {
    const parsed = parseCurl(curlImportText.value)
    if (!parsed.url) {
      message.error('无法解析 URL，请检查 cURL 格式')
      return
    }
    const parsedName = parseUrl(parsed.url, parsed.method).displayName || 'Imported Request'
    const existingReqs = requestStore.requestMap[targetCollectionId.value] ?? []
    const existing = existingReqs.find(r => r.name === parsedName)

    if (existing) {
      // 1.0.4：同名 → 不建新接口，在该接口下新增用例并快照 headers/body
      const tc = await testCaseStore.createTestCase({
        requestId: existing.id,
        collectionId: existing.collection_id,
        name: '',
        method: parsed.method,
        url: parsed.url,
        headers: JSON.stringify(parsed.headers),
        params_: '[]',
        bodyType: parsed.bodyType,
        body: parsed.body,
      })
      tabStore.openTab(existing)
      message.success(`cURL 已导入为「${existing.name}」的用例「${tc.name}」`)
    } else {
      const req = await requestStore.createRequest(targetCollectionId.value, parsedName, parsed.method, parsed.url)

      // Update headers and body
      await requestStore.updateRequest(req.id, {
        headers: JSON.stringify(parsed.headers),
        body_type: parsed.bodyType,
        body: parsed.body
      })
      // cURL 导入成功后自动在 TabBar 打开
      tabStore.openTab(req)

      message.success('cURL 导入成功')
    }
    showCurlImportDialog.value = false
    curlImportText.value = ''
    targetCollectionId.value = null
  } catch (e) {
    message.error('导入失败: ' + String(e))
  } finally {
    creating.value = false
  }
}
</script>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  border-right: 1px solid var(--border-base);
  background: var(--bg-surface);
  flex-shrink: 0;
  overflow: hidden;
}
.sidebar__search { padding: var(--spacing-sm) var(--spacing-sm) var(--spacing-xs); flex-shrink: 0; display: flex; align-items: center; gap: var(--spacing-xs); }
.sidebar__tree { flex: 1; overflow-y: auto; padding: var(--spacing-xs) 0; }
.sidebar__footer { padding: var(--spacing-sm) var(--spacing-sm); border-top: 1px solid var(--border-base); flex-shrink: 0; }

/* ── label 区域 ───────────────────────────────────────────── */
.node-label {
  display: inline-flex;
  align-items: center;
  font-size: var(--font-size-base);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 右键菜单（在 Teleport body 下，由 Sidebar 组件控制，需要 scoped）*/
.ctx-menu {
  position: fixed;
  z-index: 9999;
  background: var(--bg-elevated);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  padding: var(--spacing-xs) 0;
  min-width: 148px;
  font-size: var(--font-size-base);
}
.ctx-item {
  padding: var(--spacing-sm) var(--spacing-md);
  cursor: pointer;
  color: var(--text-primary);
  user-select: none;
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  transition: background 0.1s;
}
.ctx-item:hover { background: var(--bg-hover); }
.ctx-item--danger { color: var(--color-error); }
.ctx-item--danger:hover { background: rgba(208, 48, 80, 0.07); }
</style>

<!-- 非 scoped：renderSuffix 返回的 VNode 由 Naive UI TreeNode 渲染，DOM 上不会有 Sidebar 的 scope 哈希 -->
<!-- 这些类必须是全局 CSS 才能匹配到 n-tree 内部渲染的元素 -->
<style>
/* ── suffix 外层容器：圆点（始终可见）+ 按钮（hover 才显示）排成一行 */
.node-suffix {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
}

/* ── 操作按钮组：默认隐藏，nodeProps mouseenter/mouseleave 通过 DOM style 控制 */
.node-actions {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.15s ease;
}

/* ── 操作按钮基础样式 */
.node-action-btn {
  all: unset;
  box-sizing: border-box;
  width: calc(var(--row-height) - 10px);
  height: calc(var(--row-height) - 12px);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: var(--font-size-sm);
  letter-spacing: 0.5px;
  line-height: 1;
  user-select: none;
  border: none !important;
  outline: none !important;
  box-shadow: none !important;
  -webkit-appearance: none !important;
  transition: background 0.12s ease, color 0.12s ease;
}

.node-action-btn:hover {
  background: var(--bg-active);
  color: var(--color-primary);
}

/* + 按钮较大字号 */
.node-action-btn--add {
  font-size: var(--font-size-lg);
  letter-spacing: 0;
}

.node-action-btn--add:hover {
  background: var(--color-primary-soft);
  color: var(--color-primary);
}
</style>
