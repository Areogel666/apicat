<template>
  <div class="docs-sidebar">
    <div class="sidebar-header">
      <n-input
        v-model:value="searchText"
        size="small"
        placeholder="搜索文档..."
        clearable
        class="search-input"
      />
      <n-button size="small" quaternary title="刷新" @click="refresh">↻</n-button>
    </div>

    <div v-if="loading" class="empty">扫描中...</div>
    <div v-else-if="!docsRoot" class="empty">
      未配置文档目录。<br>
      <span class="hint">在「项目设置」里配置文档输出目录，或用 apicat-doc-gen 技能生成文档。</span>
    </div>
    <div v-else-if="filteredFiles.length === 0" class="empty">
      {{ searchText ? '没有匹配的文档' : '目录下暂无 .md 文档' }}<br>
      <span class="hint">用 apicat-doc-gen 技能生成文档后在此查看。</span>
    </div>
    <n-tree
      v-else
      :data="treeData"
      :selected-keys="selectedKey ? [selectedKey] : []"
      :render-label="renderLabel"
      :node-props="nodeProps"
      block-line
      class="docs-tree"
      @update:selected-keys="onSelect"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, h } from 'vue'
import { NInput, NButton, NTree, useMessage } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '../../stores/project'

interface DocFile {
  relative_path: string
  name: string
  dir: string
  size: number
  modified_at: string
  absolute_path: string
}

const emit = defineEmits<{
  select: [file: DocFile | null]
}>()

const projectStore = useProjectStore()
const message = useMessage()

const searchText = ref('')
const loading = ref(false)
const files = ref<DocFile[]>([])
const selectedKey = ref<string | null>(null)
const selectedFile = ref<DocFile | null>(null)

/** 文档根目录：优先 docs_output_dir，空则回落默认路径 */
const docsRoot = computed(() => {
  const pid = projectStore.currentProjectId
  if (pid == null) return null
  const proj = projectStore.projects.find(p => p.id === pid)
  return proj?.docs_output_dir || null
})

/** 默认回落目录（由后端解析 ~，前端只传 null 让后端处理） */
async function resolveDefaultDir(): Promise<string | null> {
  const pid = projectStore.currentProjectId
  if (pid == null) return null
  const proj = projectStore.projects.find(p => p.id === pid)
  if (proj?.docs_output_dir) return proj.docs_output_dir
  // 回落：~/.apicat/apidoc/{project_name}
  const home = await invoke<string>('get_home_dir').catch(() => null)
  if (!home) return null
  return `${home}/.apicat/apidoc/${proj?.name ?? 'unknown'}`
}

const filteredFiles = computed(() => {
  if (!searchText.value.trim()) return files.value
  const kw = searchText.value.trim().toLowerCase()
  return files.value.filter(f =>
    f.relative_path.toLowerCase().includes(kw) || f.name.toLowerCase().includes(kw),
  )
})

interface TreeNode {
  key: string
  label: string
  children?: TreeNode[]
  isLeaf?: boolean
  file?: DocFile
  [key: string]: unknown
}

/** 把扁平文件列表转成目录树 */
const treeData = computed<TreeNode[]>(() => {
  const topNodes: TreeNode[] = []
  // dir path → 该目录节点的 children 数组引用
  const dirChildren: Record<string, TreeNode[]> = {}
  // dir path → 该目录节点本身（去重用）
  const dirNodes: Record<string, TreeNode> = {}

  function getOrCreateDir(dirPath: string, label: string): TreeNode {
    if (dirNodes[dirPath]) return dirNodes[dirPath]
    const node: TreeNode = { key: `dir:${dirPath}`, label, children: [] }
    dirNodes[dirPath] = node
    dirChildren[dirPath] = node.children!
    // 挂到父目录或顶层
    const parentSlash = dirPath.lastIndexOf('/')
    if (parentSlash === -1) {
      topNodes.push(node)
    } else {
      const parentPath = dirPath.slice(0, parentSlash)
      const parentArr = dirChildren[parentPath]
      if (parentArr) parentArr.push(node)
      else topNodes.push(node) // 父目录还没创建（乱序兜底）
    }
    return node
  }

  for (const f of filteredFiles.value) {
    const parts = f.relative_path.split('/')
    const fileName = parts.pop()!
    // 逐级创建目录
    let currentPath = ''
    for (const part of parts) {
      currentPath = currentPath ? `${currentPath}/${part}` : part
      getOrCreateDir(currentPath, part)
    }
    // 叶子节点
    const fileNode: TreeNode = {
      key: `file:${f.relative_path}`,
      label: fileName,
      isLeaf: true,
      file: f,
    }
    if (currentPath) {
      dirChildren[currentPath]?.push(fileNode)
    } else {
      topNodes.push(fileNode)
    }
  }

  return topNodes
})

/** 双击文件节点 → 用系统默认程序打开 */
function nodeProps(info: { option: Record<string, unknown> }) {
  const opt = info.option as unknown as TreeNode
  if (!opt.isLeaf || !opt.file) return {}
  const file = opt.file
  return {
    onDblclick: (e: MouseEvent) => {
      e.stopPropagation()
      invoke('open_file_with_default', { path: file.absolute_path }).catch(err => {
        message.error(`打开文件失败：${err}`)
      })
    },
  }
}

function renderLabel({ option }: { option: { key?: unknown; label?: unknown; isLeaf?: unknown; [k: string]: unknown } }) {
  if (option.isLeaf) {
    return h('span', { class: 'doc-file-label' }, String(option.label ?? ''))
  }
  return h('span', { class: 'doc-dir-label' }, `📁 ${String(option.label ?? '')}`)
}

async function refresh() {
  const dir = await resolveDefaultDir()
  if (!dir) {
    files.value = []
    return
  }
  loading.value = true
  try {
    files.value = await invoke<DocFile[]>('scan_docs_dir', { dir })
  } catch (e) {
    message.error(`扫描文档目录失败：${e}`)
    files.value = []
  } finally {
    loading.value = false
  }
}

function onSelect(keys: string[]) {
  const key = keys[0] ?? null
  selectedKey.value = key
  if (key && key.startsWith('file:')) {
    const relPath = key.slice(5)
    const file = files.value.find(f => f.relative_path === relPath) ?? null
    selectedFile.value = file
    emit('select', file)
  } else {
    selectedFile.value = null
    emit('select', null)
  }
}

/** 双击打开文件 */
function openFile(file: DocFile) {
  invoke('open_file_with_default', { path: file.absolute_path }).catch(e => {
    message.error(`打开文件失败：${e}`)
  })
}

defineExpose({ refresh, openFile, selectedFile })

onMounted(refresh)
watch(() => projectStore.currentProjectId, refresh)
</script>

<style scoped>
.docs-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.sidebar-header {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px;
  border-bottom: 1px solid var(--border-base);
  flex-shrink: 0;
}

.search-input {
  flex: 1;
}

.docs-tree {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
}

.empty {
  padding: 24px 16px;
  text-align: center;
  color: var(--text-tertiary);
  font-size: 13px;
  line-height: 1.8;
}

.empty .hint {
  font-size: 12px;
  color: var(--text-disabled);
}
</style>

<style>
.docs-sidebar .doc-file-label {
  font-size: 13px;
}
.docs-sidebar .doc-dir-label {
  font-size: 13px;
  color: var(--text-secondary);
}
</style>
