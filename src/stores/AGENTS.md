# STORES — Pinia 状态层

## OVERVIEW
12 个 setup store，覆盖全部业务域。所有 `invoke()` 调用在 store 内部完成，组件不直接调用 Tauri IPC。

## STORES 清单
| Store | 状态 | 职责 |
|-------|------|------|
| `project.ts` | `projects[]`, `currentProjectId`, `sidebarReloadTick` | 项目 CRUD，切换项目，触发侧边栏重载信号 |
| `collection.ts` | `collectionMap: Record<projectId, Collection[]>` | 文件夹树（一次性拉全，内存组树） |
| `request.ts` | `requestMap: Record<collectionId, ApiRequest[]>` | 接口列表，按 collection 分组 |
| `tab.ts` | `openTabs[]`, `activeRequestId` | 多标签管理 + plugin-store 持久化 |
| `response.ts` | 当前响应数据 | 发请求结果缓存 |
| `history.ts` | `historyMap` | 请求历史记录 |
| `environment.ts` | `environments[]`, `envVariables[]` | 环境 + 变量，`activeEnvId` |
| `cookie.ts` | `cookies[]` | Cookie 管理 |
| `headerTemplate.ts` | `templates[]` | 请求头模板 |
| `testCase.ts` | `testCases[]` | 测试用例 |
| `stress.ts` | `config`, `stats`, `chartPoints[]` | 压测配置与实时数据 |
| `ui.ts` | UI 状态（弹窗开关等） | 跨组件 UI 状态 |

## KEY PATTERNS

### sidebarReloadTick（重要）
```ts
// 导入到当前项目时 currentProjectId 不变，watch 不触发
// 通过递增 tick 强制 Sidebar 重载
projectStore.sidebarReloadTick++
```
仅在 `ImportDialog.vue` 完成导入后调用，**不要**在其他地方随意递增。

### collectionMap / requestMap 结构
```ts
// 按 projectId / collectionId 分组的扁平数组
// 树形结构在 Sidebar.vue 的 treeData computed 里组装，store 不存树
collectionMap[projectId]   // Collection[] 扁平列表
requestMap[collectionId]   // ApiRequest[] 扁平列表
```

### tab 持久化
`tab.ts` 在 `saveState()` / `restoreState()` 中使用 `@tauri-apps/plugin-store`。
**必须**在 `capabilities/default.json` 声明 `"store:default"` 权限，否则 IPC 被静默拒绝。

## ANTI-PATTERNS
- **不要**在 store 外部调用 `invoke()`——统一在 store action 内封装
- **不要**直接修改 `collectionMap[pid]` 数组元素——用 `splice` 替换触发响应式（见 `collection.ts:36`）
- **不要**依赖 `currentProjectId` watch 来检测"导入到当前项目"的情况——值不变 watch 不触发，用 `sidebarReloadTick`
