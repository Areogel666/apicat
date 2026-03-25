<template>
  <aside class="sidebar" :style="{ width: sidebarWidth + 'px' }">
    <!-- 搜索框 -->
    <div class="sidebar__search">
      <n-input v-model:value="searchText" placeholder="搜索接口..." size="small" clearable>
        <template #prefix>🔍</template>
      </n-input>
    </div>

    <!-- 树区域 -->
    <div class="sidebar__tree" @click.self="closeMenu">
      <n-empty v-if="!currentProjectId" description="请先选择或创建项目" size="small" style="margin-top: 40px" />
      <n-spin v-else-if="loading" size="small" style="margin-top: 40px; display:flex; justify-content:center" />
      <n-tree
        v-else
        :data="treeData"
        :filter-text="searchText"
        :node-props="nodeProps"
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
      <n-button size="small" block dashed @click="showNewCollectionDialog = true">
        + 新建文件夹
      </n-button>
      <n-button size="small" block dashed style="margin-top:4px" @click="showNewRequestDialog = true">
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

    <!-- 接口名重名冲突对话框 -->
    <n-modal v-model:show="showDuplicateNameDialog" preset="dialog" title="接口名已存在">
      <div style="font-size:14px; line-height:1.7; color:var(--n-text-color,#333)">
        当前文件夹中已存在同名接口：<br>
        <strong>「{{ pendingRequestName }}」</strong>
      </div>
      <template #action>
        <n-button @click="showDuplicateNameDialog = false">取消</n-button>
        <n-button @click="createWithAutoName" :loading="creating">重命名后创建</n-button>
        <n-button type="primary" @click="createForce" :loading="creating">仍然创建</n-button>
      </template>
    </n-modal>
  </aside>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { NInput, NEmpty, NButton, NTree, NSpin, NModal, NSpace, NSelect, useMessage } from 'naive-ui'
import type { TreeOption, TreeDropInfo } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { useUiStore } from '../../stores/ui'
import { useProjectStore } from '../../stores/project'
import { useCollectionStore } from '../../stores/collection'
import { useRequestStore } from '../../stores/request'
import { parseUrl } from '../../utils/urlParser'
import { buildCurl } from '../../utils/curlBuilder'
import type { Collection } from '../../types'

const uiStore = useUiStore()
const projectStore = useProjectStore()
const collectionStore = useCollectionStore()
const requestStore = useRequestStore()
const message = useMessage()

const sidebarWidth = uiStore.sidebarWidth
const searchText = ref('')
const loading = ref(false)
const creating = ref(false)
const expandedKeys = ref<string[]>([])

// 对话框状态
const showNewCollectionDialog = ref(false)
const newCollectionName = ref('')
const showNewRequestDialog = ref(false)
const newRequestUrl = ref('')
const newRequestName = ref('')
const newRequestMethod = ref('GET')

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
      await requestStore.renameRequest(id, name)
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

  const curl = buildCurl({
    method: req.method,
    url: req.url,
    queryParams: queryArr,
    headers: headersArr,
    bodyType: req.body_type,
    body: req.body,
    authType: req.auth_type,
    authConfig: req.auth_config,
  })

  try {
    await navigator.clipboard.writeText(curl)
    message.success('cURL 已复制到剪贴板')
  } catch {
    // Tauri 环境 clipboard API 可能需要权限，降级为 execCommand
    const ta = document.createElement('textarea')
    ta.value = curl
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
    message.success('cURL 已复制到剪贴板')
  }
}

// ── 删除节点 ─────────────────────────────────────────────
async function deleteItem() {
  closeMenu()
  const key = menuNodeKey.value
  const pid = currentProjectId.value
  if (!pid) return

  try {
    if (key.startsWith('req-')) {
      const id = parseInt(key.replace('req-', ''))
      const req = Object.values(requestStore.requestMap).flat().find(r => r.id === id)
      if (!req) return
      if (!window.confirm(`确定删除接口「${req.name}」吗？`)) return
      await requestStore.deleteRequest(id, req.collection_id)
      message.success('接口已删除')
    } else {
      const id = parseInt(key.replace('col-', ''))
      const col = collectionStore.getCollections(pid).find(c => c.id === id)
      if (!col) return
      if (!window.confirm(`确定删除文件夹「${col.name}」及其所有接口吗？`)) return
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
        delete requestStore.requestMap[cid]
      }
      message.success('文件夹已删除')
    }
  } catch (e) {
    message.error(String(e))
  }
}

// ── 项目切换时加载数据 ────────────────────────────────────
watch(currentProjectId, async (pid) => {
  if (!pid) return
  loading.value = true
  try {
    await collectionStore.loadCollections(pid)
    const cols = collectionStore.getCollections(pid)
    await Promise.all(cols.map(c => requestStore.loadRequests(c.id)))
    expandedKeys.value = cols.slice(0, 3).map(c => `col-${c.id}`)
  } finally {
    loading.value = false
  }
}, { immediate: true })

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

  function buildCollectionNode(col: Collection): TreeOption {
    const reqs = (requestStore.requestMap[col.id] ?? []).map(r => ({
      key: `req-${r.id}`,
      label: `${r.method} ${r.name}`,
      isLeaf: true,
      data: r,
    } as TreeOption))

    const subCols = (childMap[col.id] ?? []).map(buildCollectionNode)

    return {
      key: `col-${col.id}`,
      label: col.name,
      children: [...subCols, ...reqs],
      data: col,
    }
  }

  return rootCols.map(buildCollectionNode)
})

// ── 节点点击 → 激活接口 ───────────────────────────────────
function onSelectNode(keys: Array<string | number>) {
  const key = keys[0] as string
  if (!key?.startsWith('req-')) return
  const id = parseInt(key.replace('req-', ''))
  requestStore.activeRequestId = id
}

// ── 节点 props（右键菜单） ────────────────────────────────
function nodeProps({ option }: { option: TreeOption }) {
  return {
    onContextmenu(e: MouseEvent) {
      e.preventDefault()
      e.stopPropagation()
      const key = option.key as string
      if (key.startsWith('req-')) {
        openMenu(e, key, 'request')
      } else if (key.startsWith('col-')) {
        openMenu(e, key, 'collection')
      }
    },
  }
}

// ── 拖拽排序 ─────────────────────────────────────────────
/** 只允许同类同层拖拽（request → request，collection → collection），不允许放入文件夹内 */
function allowDrop({ dropPosition }: { dropPosition: string; node: TreeOption }) {
  // 不允许 inside 放置（放进文件夹内）
  if (dropPosition === 'inside') return false
  return true
}

async function onDrop(info: TreeDropInfo) {
  const { node, dragNode, dropPosition } = info
  const pid = currentProjectId.value
  if (!pid) return

  const dragKey = dragNode.key as string
  const targetKey = node.key as string

  if (dragKey.startsWith('req-') && targetKey.startsWith('req-')) {
    // 检查是否跨 collection 拖拽（不支持，给出提示）
    const dragId = parseInt(dragKey.replace('req-', ''))
    const targetId = parseInt(targetKey.replace('req-', ''))
    let dragCol: number | null = null
    let targetCol: number | null = null
    for (const [colId, reqs] of Object.entries(requestStore.requestMap)) {
      if (reqs.some(r => r.id === dragId)) dragCol = parseInt(colId)
      if (reqs.some(r => r.id === targetId)) targetCol = parseInt(colId)
    }
    if (dragCol !== targetCol) {
      message.warning('暂不支持跨文件夹拖拽，请在同一文件夹内排序')
      return
    }
    // 找到共同的 collection（通过 requestMap 定位）
    // 此时 dragCol === targetCol，直接用 dragCol
    const collectionId = dragCol!
    const items = [...(requestStore.requestMap[collectionId] ?? [])]
    if (!items.length) return

    // 重排数组
    const dragIdx = items.findIndex(r => `req-${r.id}` === dragKey)
    const targetIdx = items.findIndex(r => `req-${r.id}` === targetKey)
    const [moved] = items.splice(dragIdx, 1)
    const insertAt = dropPosition === 'before' ? targetIdx : targetIdx + 1
    items.splice(insertAt > dragIdx ? insertAt - 1 : insertAt, 0, moved)

    // 更新 sort_order 并持久化
    const sorted = items.map((r, i) => ({ ...r, sort_order: i }))
    requestStore.requestMap[collectionId] = sorted
    try {
      await invoke('update_request_sort', {
        items: sorted.map(r => [r.id, r.sort_order] as [number, number]),
      })
    } catch (e) {
      message.error('保存排序失败: ' + String(e))
    }

  } else if (dragKey.startsWith('col-') && targetKey.startsWith('col-')) {
    const cols = [...collectionStore.getCollections(pid)]
    const dragIdx = cols.findIndex(c => `col-${c.id}` === dragKey)
    const targetIdx = cols.findIndex(c => `col-${c.id}` === targetKey)
    if (dragIdx === -1 || targetIdx === -1) return

    const [moved] = cols.splice(dragIdx, 1)
    const insertAt = dropPosition === 'before' ? targetIdx : targetIdx + 1
    cols.splice(insertAt > dragIdx ? insertAt - 1 : insertAt, 0, moved)

    const sorted = cols.map((c, i) => ({ ...c, sort_order: i }))
    collectionStore.collectionMap[pid] = sorted
    try {
      await invoke('update_collection_sort', {
        items: sorted.map(c => [c.id, c.sort_order] as [number, number]),
      })
    } catch (e) {
      message.error('保存排序失败: ' + String(e))
    }
  }
}

// ── 创建文件夹 ────────────────────────────────────────────
async function createCollection() {
  const pid = currentProjectId.value
  if (!pid || !newCollectionName.value.trim()) return
  creating.value = true
  try {
    const col = await collectionStore.createCollection(pid, newCollectionName.value.trim())
    showNewCollectionDialog.value = false
    newCollectionName.value = ''
    requestStore.requestMap[col.id] = []
    message.success('文件夹创建成功')
  } catch (e) {
    message.error(String(e))
  } finally {
    creating.value = false
  }
}

// ── 创建接口 ──────────────────────────────────────────────
// 重名冲突对话框状态
const showDuplicateNameDialog = ref(false)
const pendingRequestName = ref('')
// 待创建接口的暂存参数（用于冲突处理后继续创建）
const pendingCreateParams = ref<{
  collectionId: number
  name: string
  method: string
  url: string
} | null>(null)

async function doCreateRequest() {
  const pid = currentProjectId.value
  if (!pid) { message.error('请先选择项目'); return }
  const cols = collectionStore.getCollections(pid)
  if (cols.length === 0) { message.error('请先创建文件夹'); return }

  const url = newRequestUrl.value.trim()
  if (!url) { message.error('请输入 URL'); return }

  const parsed = parseUrl(url, newRequestMethod.value)
  const name = newRequestName.value.trim() || parsed.displayName
  const targetCollectionId = cols[0].id

  // 检查 Collection 内是否已有同名接口
  const existingReqs = requestStore.requestMap[targetCollectionId] ?? []
  const hasDuplicate = existingReqs.some(r => r.name === name)

  if (hasDuplicate) {
    // 暂存参数，弹出冲突对话框
    pendingRequestName.value = name
    pendingCreateParams.value = { collectionId: targetCollectionId, name, method: newRequestMethod.value, url }
    showDuplicateNameDialog.value = true
    return
  }

  await executeCreateRequest(targetCollectionId, name, newRequestMethod.value, url)
}

/** 「重命名后创建」— 自动追加序号找最小可用名 */
async function createWithAutoName() {
  if (!pendingCreateParams.value) return
  const { collectionId, name, method, url } = pendingCreateParams.value
  const existingReqs = requestStore.requestMap[collectionId] ?? []
  const existingNames = new Set(existingReqs.map(r => r.name))

  // 找最小可用序号：「name (2)」「name (3)」...
  let finalName = name
  let i = 2
  while (existingNames.has(finalName)) {
    finalName = `${name} (${i++})`
  }

  showDuplicateNameDialog.value = false
  await executeCreateRequest(collectionId, finalName, method, url)
}

/** 「仍然创建」— 强制用原名创建（DB 层允许同名） */
async function createForce() {
  if (!pendingCreateParams.value) return
  const { collectionId, name, method, url } = pendingCreateParams.value
  showDuplicateNameDialog.value = false
  await executeCreateRequest(collectionId, name, method, url)
}

/** 实际执行创建接口并重置表单 */
async function executeCreateRequest(collectionId: number, name: string, method: string, url: string) {
  creating.value = true
  try {
    await requestStore.createRequest(collectionId, name, method, url)
    showNewRequestDialog.value = false
    newRequestUrl.value = ''
    newRequestName.value = ''
    newRequestMethod.value = 'GET'
    pendingCreateParams.value = null
    message.success('接口创建成功')
  } catch (e) {
    message.error(String(e))
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
  border-right: 1px solid var(--n-border-color, #e0e0e6);
  background: var(--n-color-embedded, #f9f9f9);
  flex-shrink: 0;
  overflow: hidden;
}
.sidebar__search { padding: 10px 10px 6px; flex-shrink: 0; }
.sidebar__tree { flex: 1; overflow-y: auto; padding: 4px 0; }
.sidebar__footer { padding: 8px 10px; border-top: 1px solid var(--n-border-color, #e0e0e6); flex-shrink: 0; }

.tree-node { width: 100%; }

/* 右键菜单 */
.ctx-menu {
  position: fixed;
  z-index: 9999;
  background: var(--n-color, #fff);
  border: 1px solid var(--n-border-color, #e0e0e6);
  border-radius: 6px;
  box-shadow: 0 4px 16px rgba(0,0,0,0.12);
  padding: 4px 0;
  min-width: 148px;
  font-size: 13px;
}
.ctx-item {
  padding: 7px 14px;
  cursor: pointer;
  color: var(--n-text-color, #333);
  user-select: none;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: background 0.1s;
}
.ctx-item:hover { background: var(--n-item-color-hover, rgba(0,0,0,0.05)); }
.ctx-item--danger { color: var(--n-error-color, #d03050); }
.ctx-item--danger:hover { background: rgba(208,48,80,0.07); }
</style>
