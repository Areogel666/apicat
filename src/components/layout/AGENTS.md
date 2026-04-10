# LAYOUT COMPONENTS — 核心 UI 层

## OVERVIEW
6 个组件构成应用骨架：`AppLayout`（壳）→ `TopBar` + `Sidebar` + `MainPanel`（含 `TabBar`）。

## 组件职责
| 组件 | 行数 | 职责 |
|------|------|------|
| `AppLayout.vue` | 32 | 顶栏 + 侧边栏 + 主面板的 flex 容器 |
| `TopBar.vue` | 288 | 项目切换、环境选择、Cookie/环境管理入口、设置菜单（检查更新） |
| `Sidebar.vue` | 1300+ | 接口树（NTree）、拖拽移动、右键菜单、搜索、新建/重命名/删除 |
| `MainPanel.vue` | — | 请求编辑区（参数/Header/Body/Auth）+ 响应面板 |
| `TabBar.vue` | — | 多标签栏，tab 开关/激活，与 tabStore 联动 |
| `HeaderTemplateModal.vue` | — | 请求头模板管理弹窗 |

## SIDEBAR 关键设计（Sidebar.vue）

### Tree node key 格式（全局约定，不可改）
```
"col-{id}"   → Collection（文件夹）
"req-{id}"   → ApiRequest（接口）
```

### 拖拽实现要点
```ts
// Naive UI NTree AllowDrop 类型不含 dragNode！
// 正确做法：用 @dragstart 记录被拖拽节点
const currentDragNode = ref<TreeOption | null>(null)
// n-tree 绑定：@dragstart="onDragStart" @dragend="onDragEnd"

// allowDrop 从 currentDragNode.value 读取，而非参数解构
function allowDrop({ dropPosition, node }) { 
  const dragNode = currentDragNode.value  // ← 唯一正确取法
}
```

### 侧边栏加载防竞态
```ts
// loadSeq 序列号机制：防止快速切换项目时多个 async watch 并发
let loadSeq = 0
watch([currentProjectId, sidebarReloadTick], async () => {
  const seq = ++loadSeq
  // ... 异步加载 ...
  if (seq !== loadSeq) return  // 已被新的加载覆盖，丢弃
})
```

### saveState / restoreState 必须独立 try/catch
plugin-store IPC 失败时不能阻断 `loadCollections()`，必须：
```ts
try { await tabStore.saveState() } catch {}
await loadCollections(pid)  // 必须在 try 外执行
```

## ANTI-PATTERNS
- **不要**在 `allowDrop` 参数里解构 `dragNode`（NTree 不传此字段，会得到 undefined）
- **不要**在 `saveState`/`restoreState` 异常时中断后续加载逻辑
- **不要**在 `loadCollections` 之外手动操作 collectionStore/requestStore（用 `sidebarReloadTick` 信号触发）
- Naive UI `NTree` 的 `expand-on-click` 与 `@update:selected-keys` 同时存在时，点击展开不触发 selected 事件
