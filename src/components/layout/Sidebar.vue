<template>
  <aside class="sidebar" :style="{ width: sidebarWidth + 'px' }">
    <!-- 搜索框 -->
    <div class="sidebar__search">
      <n-input v-model:value="searchText" placeholder="搜索接口..." size="small" clearable>
        <template #prefix>🔍</template>
      </n-input>
    </div>

    <!-- 树区域 -->
    <div class="sidebar__tree">
      <n-empty v-if="!currentProjectId" description="请先选择或创建项目" size="small" style="margin-top: 40px" />
      <n-spin v-else-if="loading" size="small" style="margin-top: 40px; display:flex; justify-content:center" />
      <n-tree
        v-else
        :data="treeData"
        :filter-text="searchText"
        :node-props="nodeProps"
        block-line
        expand-on-click
        :default-expanded-keys="expandedKeys"
        @update:selected-keys="onSelectNode"
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

    <!-- 新建文件夹对话框 -->
    <n-modal v-model:show="showNewCollectionDialog" preset="dialog" title="新建文件夹">
      <n-input v-model:value="newCollectionName" placeholder="文件夹名称" @keyup.enter="createCollection" />
      <template #action>
        <n-button @click="showNewCollectionDialog = false">取消</n-button>
        <n-button type="primary" :loading="creating" @click="createCollection">创建</n-button>
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
  </aside>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NInput, NEmpty, NButton, NTree, NSpin, NModal, NSpace, NSelect, useMessage } from 'naive-ui'
import type { TreeOption } from 'naive-ui'
import { useUiStore } from '../../stores/ui'
import { useProjectStore } from '../../stores/project'
import { useCollectionStore } from '../../stores/collection'
import { useRequestStore } from '../../stores/request'
import { parseUrl } from '../../utils/urlParser'
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

// 项目切换时，加载该项目的所有 collections 和 requests（一次性，避免 N+1）
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

// 构建 NTree 数据（内存组装树，设计文档 10.4 N+1 防护）
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

// 节点点击 → 激活接口
function onSelectNode(keys: Array<string | number>) {
  const key = keys[0] as string
  if (!key?.startsWith('req-')) return
  const id = parseInt(key.replace('req-', ''))
  requestStore.activeRequestId = id
}

// 节点右键菜单（M3 完善）
function nodeProps(_: { option: TreeOption }) {
  return {
    onContextmenu(e: MouseEvent) {
      e.preventDefault()
      // TODO: M3 右键菜单
    }
  }
}

// 创建文件夹
async function createCollection() {
  const pid = currentProjectId.value
  if (!pid || !newCollectionName.value.trim()) return
  creating.value = true
  try {
    const col = await collectionStore.createCollection(pid, newCollectionName.value.trim())
    showNewCollectionDialog.value = false
    newCollectionName.value = ''
    // 新建的 collection 无 requests，初始化空数组
    requestStore.requestMap[col.id] = []
    message.success('文件夹创建成功')
  } catch (e) {
    message.error(String(e))
  } finally {
    creating.value = false
  }
}

// 创建接口（自动解析 URL 生成名称和 Path Params）
async function doCreateRequest() {
  const pid = currentProjectId.value
  if (!pid) { message.error('请先选择项目'); return }
  const cols = collectionStore.getCollections(pid)
  if (cols.length === 0) { message.error('请先创建文件夹'); return }

  const url = newRequestUrl.value.trim()
  if (!url) { message.error('请输入 URL'); return }

  const parsed = parseUrl(url, newRequestMethod.value)
  const name = newRequestName.value.trim() || parsed.displayName

  // 默认放入第一个 collection
  const targetCollectionId = cols[0].id

  creating.value = true
  try {
    await requestStore.createRequest(targetCollectionId, name, newRequestMethod.value, url)
    showNewRequestDialog.value = false
    newRequestUrl.value = ''
    newRequestName.value = ''
    newRequestMethod.value = 'GET'
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
</style>
