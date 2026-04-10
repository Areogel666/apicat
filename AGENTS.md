# PROJECT KNOWLEDGE BASE — ApiCat

**Generated:** 2026-04-10
**Commit:** aad4476
**Branch:** feature/multi-tab

## OVERVIEW
ApiCat 是一款桌面 API 调试工具（类 Postman），使用 Tauri 2.x + Vue 3 + TypeScript 构建前端，Rust + SQLite（sqlx 0.8）构建后端，单文件 `apicat.db` 存储所有数据。

## STRUCTURE
```
apicat/
├── src/                        # Vue 3 前端
│   ├── App.vue                 # 根组件，初始化 Pinia stores
│   ├── main.ts                 # 应用入口，挂载 Naive UI
│   ├── types/index.ts          # 前后端共享类型（必须与 types.rs 保持一致）
│   ├── stores/                 # 12 个 Pinia stores（见 stores/AGENTS.md）
│   ├── components/
│   │   ├── layout/             # 核心布局组件（见 layout/AGENTS.md）
│   │   ├── io/                 # 导入/导出对话框
│   │   ├── cookie/             # Cookie 管理
│   │   ├── env/                # 环境变量管理
│   │   ├── response/           # 响应面板、历史、JSON Viewer
│   │   ├── stress/             # 压测配置与结果
│   │   └── testcase/           # 测试用例
│   └── utils/                  # curlBuilder / paramParser / urlParser
├── src-tauri/                  # Rust 后端（见 src-tauri/src/AGENTS.md）
│   ├── src/
│   │   ├── lib.rs              # 插件注册 + AppDb 注入 + command 注册
│   │   ├── types.rs            # 数据模型（与 src/types/index.ts 镜像）
│   │   ├── error.rs            # AppError / CmdResult<T>
│   │   ├── db/mod.rs           # SQLite pool 初始化，WAL 模式
│   │   ├── commands/           # Tauri command（见 commands/AGENTS.md）
│   │   └── http/               # reqwest 客户端封装
│   ├── migrations/0001_init.sql # 全部建表 SQL（幂等，手动分号分割执行）
│   └── capabilities/default.json # IPC 权限声明（必须显式声明每个插件）
├── docs/
│   ├── plans/                  # 功能设计方案（1.0.0/, 1.0.1/）
│   ├── fix/                    # Bug 修复记录（已加入 .gitignore，不提交）
│   └── release/                # 发布说明（提交到 git）
└── scripts/apicat-test-gen/    # AI 测试用例生成脚本
```

## WHERE TO LOOK
| 任务 | 位置 |
|------|------|
| 添加新 Tauri command | `src-tauri/src/commands/` 新增函数 → `lib.rs` invoke_handler 注册 |
| 添加新数据库表 | `migrations/0001_init.sql` + `src-tauri/src/types.rs` + `src/types/index.ts` |
| 添加新前端 store | `src/stores/` 新建文件，参考现有 setup store 风格 |
| 修改 IPC 权限 | `src-tauri/capabilities/default.json` |
| 导入/导出逻辑 | `src-tauri/src/commands/io.rs` + `src/components/io/` |
| 侧边栏树逻辑 | `src/components/layout/Sidebar.vue`（1300+ 行，含拖拽） |
| 压测功能 | `src-tauri/src/commands/stress.rs` + `src/components/stress/` |

## KEY PATTERNS

### Tauri IPC
```ts
// 前端调用：参数名自动 camelCase → snake_case 转换
invoke('list_collections', { projectId: 1 })
// Rust 侧接收：project_id: i64
```

### 全局状态
- Pinia stores 通过 `useXxxStore()` 在任意 `.vue` 文件中调用
- 无 provide/inject，无 Vuex，纯 Pinia
- `invoke()` 调用**直接在 store 内部**，不做额外抽象层

### SQLite 连接
- `AppDb(SqlitePool)` 通过 `app.manage()` 注入全局状态
- 每个 command 通过 `State<'_, AppDb>` 获取连接池（max 5 connections）
- WAL 模式 + foreign keys ON，初始化在 `db/mod.rs`

### 错误处理
- 所有 command 返回 `CmdResult<T> = Result<T, AppError>`
- `AppError` 实现 `Serialize`，错误字符串直接传到前端
- 前端 `invoke()` 抛出的 error 是字符串，用 `String(e)` 处理

## CONVENTIONS
- **命名**：Vue 组件 PascalCase，store 函数 camelCase，Rust command snake_case
- **Tree node key 格式**：`"col-{id}"` 目录，`"req-{id}"` 接口（Sidebar.vue 全局约定）
- **数值布尔**：SQLite 无 BOOL 类型，前端类型定义用 `number`（0|1），不用 `boolean`
- **Tauri 2.x camelCase 转换**：IPC 参数名自动转换，Rust 侧写 `snake_case`，前端传 `camelCase`
- **store 风格**：全部使用 setup store（`defineStore('id', () => { ... })`），禁止 options API store

## ANTI-PATTERNS (THIS PROJECT)
- **不要**修改 `migrations/0001_init.sql` 现有表结构（会破坏已有 DB）；新表加在末尾
- **不要**在 capabilities 里省略新插件权限——Tauri 2.x IPC 层静默拒绝，异常会吞掉后续逻辑
- **不要**在 `allowDrop` 回调参数里解构 `dragNode`——Naive UI NTree `AllowDrop` 类型不含此字段；用 `currentDragNode` ref 替代
- **不要**在 Sidebar.vue 的 `saveState`/`restoreState` 抛出异常——必须用独立 `try/catch` 包裹，否则阻断 `loadCollections()`
- **不要**使用 `@ts-ignore` 或 `as any`（vite.config.ts 唯一的 `@ts-expect-error` 是官方模板遗留，勿模仿）
- **不要**提交 `docs/fix/` 目录（已加入 .gitignore）；只提交 `docs/release/`

## COMMANDS
```bash
npm run dev          # 启动 Vite 前端（端口 1420，strictPort）
npm run tauri dev    # 启动完整 Tauri 应用（含 Rust 热重载）
npm run build        # vue-tsc 类型检查 + Vite 打包
npm run tauri build  # 生产打包（生成安装包）
```

## NOTES
- DB 文件路径：平台 AppData 目录 + `com.apicat.app/apicat.db`（macOS: `~/Library/Application Support/com.apicat.app/`）
- `plugin-store` 用于 Tab 状态持久化（跨会话保留已打开的标签页）
- 检查更新 endpoint：`github.com/Areogel666/apicat/releases/latest`；当前无 `latest.json` 时正常降级提示"已是最新"
- `fix_migrations.rs` 在根目录是临时脚本，非正式代码
- 自动清理：启动时 fire-and-forget 删除 30 天前未收藏的测试用例（`lib.rs: cleanup_old_test_cases`）
