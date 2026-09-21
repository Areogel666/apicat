# ApiCat — Tauri 2 + Vue 3 桌面应用

接口调试桌面工具。前端 Vue 3 + TypeScript + NaiveUI + Pinia,后端 Rust(sqlx + sqlite)。

> 目录结构地图 / IPC 约定 / 命名与反模式清单见 `AGENTS.md`(根目录及 `src/stores/`、`src/components/layout/`、`src-tauri/src/` 等子目录各有分册);版本功能详述见 `docs/release/*.md`。本文档只放 Claude 动手时需要的东西。

## Commands

```bash
npm install              # 安装前端依赖
npm run tauri dev        # 开发模式(vite :1420 + cargo debug + 桌面窗口,HMR 热更新)
npx vue-tsc --noEmit     # 前端类型检查
npm run build            # 前端产物构建(vue-tsc + vite build)
npm run tauri build      # 打包 release 安装包(Windows 出 .msi)
```

## 核心语义（改动前先读）

### 数据字典 / 参数 / 草稿

- **数据字典 = 「字段名绑定」**：项目级 `field_dictionary_rules`（字段名↔字典，一对一）+ 接口级 `field_dictionary_overrides`（例外，优先）。描述列未绑定 → 直接 `n-input`；已绑定 → `FieldDictDesc.vue`（命中项 + Tooltip 全枚举 + 手写描述尾部）+ ✎ 手写编辑 popover + ✕ 解绑；绑定入口 📖 弹窗支持「项目规则 / 仅当前接口」双作用域（再点已绑定字典 = 取消）。Tooltip 枚举内容块自带 `--bg-elevated` 背景防对比问题。
- **参数元数据（type/description）随接口 `params` JSON 持久化**：`parseKvText/parseJsonToParams/parseTextToParams` 均带 `prev` 同 key 合并；用例快照缺元数据时由接口定义补全（`mergeParamMeta`）。**已作废旧 `descriptionDictRef` 字段，不再流动**（类型保留兼容旧数据）。
- **草稿隔离**：切换接口保存/恢复草稿时参数数组一律深拷贝；落库只发生在切走瞬间并锁定离开的接口 id（`flushPersist`），不在编辑期做防抖写库。
- **URL-Encoded 的 body 存储 = 结构化 JSON 数组**（含 type/description/enabled），旧 `k=v` 文本加载用 `parseUrlencodedBody` 兼容；发送时才 `syncUrlencodedData()` 编码为 k=v。保存路径统一用 `urlencodedStorageBody()`，勿直接存 bodyContent。
- **布局常驻**：`AppLayout` 中接口树/字典树侧栏与 `MainPanel`/`DictionaryJsonPanel` 都用 `v-show` 常驻（切换保留浏览状态），不要改回 `v-if` 卸载重挂。

### HTTP Bridge

- `127.0.0.1:17320`（占用顺延）起 REST 服务，token 在 `$APPDATA/com.apicat.app/bridge.json`（macOS 在 `~/Library/Application Support/com.apicat.app/`）；路径对齐 IPC command 名，GET=只读 / POST=写。AI 技能读写数据一律走 Bridge，**禁止直连 SQLite**。
- **新端点取值一律用 `get_str/get_i64`（camel+snake 双键）**：手写单键会把另一命名风格的参数静默丢弃（不报错，字段直接丢失）。
- 写操作后广播 `bridge-data-changed`，前端 store 自行 reload——**新增写端点漏广播 → 技能改完数据桌面端看不见**。

### 写操作真源

- 写操作业务逻辑放 `commands/*_impl`，IPC 与 Bridge 共用同一份实现，**禁止两侧各写 SQL/业务分叉**。
- `update_request` 的 impl 是全量覆写；Bridge 侧先 SELECT 旧值合并未传字段再调 impl（COALESCE 便利留在协议层，UI 的 IPC 签名不变）。

### 用例与断言

- 断言 JSON 形状：`{status_code, json_path}`，运行结果给出通过/失败判定。
- Bridge `create_test_case` 的 `case_type` **必填**（缺省返回 400，防止 7 种用例类型被静默全标成 happy_path）；UI 侧默认 happy_path。
- 不能删除最后一个收藏用例（IPC 与 Bridge 共用同一约束）。

### 历史列表契约

- 前端 `list_history` 只取元数据（body/snapshot/headers 三个大字段为 NULL），diff / 回填用 `get_history_record(id)` 单条补拉。
- **Bridge 的 `list_history` 仍返回全量字段**——外部技能脚本依赖此契约，勿「顺手统一」。

### 压测

- **引擎不做 `{{var}}` 替换**（与正常发送的 Rust 侧 `replace_variables` 不同）；UI 从用例发起压测时预替换 `{{base_url}}`，最终 URL 仍含 `{{...}}` 或无激活环境 → `message.warning` 提示，不静默发压测。
- **报告真源是 Rust `build_stress_report_markdown`**（App 预览/导出与 Bridge 共用），改报告只改这一处。
- **参考线阈值**：项目级默认（`projects.p95/p99_threshold_ms`，500/1000）+ 接口级覆盖（`api_requests`，null=继承）。压测启动时生效值快照写入 `stress_runs.config_json`；报告只读 config_json → 常量默认，不回查项目/接口表（旧记录无字段 → 用默认值，改阈值不影响历史报告）。

## Rust 侧加字段清单

给表加列必须一次改齐四处，漏一处 = sqlx 反序列化错位或运行时列不存在（本项目已踩过两次）：

1. migration（或 `db/mod.rs` 的 pragma 守卫式 ALTER）
2. `src-tauri/src/types.rs` 对应 struct——**字段顺序 = SELECT 列顺序**
3. `src-tauri/src/sql_cols.rs` 对应 `*_COLS`（commands 与 bridge 唯一事实源，勿在两侧再写列清单）
4. `src/types/index.ts` 镜像 interface（两侧文件头有同步约定注释，改任一侧必须同步另一侧）

## 主题系统(themeStore)

- `src/stores/theme.ts`:setup store。`customTokens` 增量覆盖 + `density`/`radiusScale`/`fontSize` → `resolvedTokens` computed → `applyTheme()` 写 `<html data-theme/data-density/data-font-size>` + inline style。
- `src/styles/tokens.css`:风格变量 `--row-height`/`--input-height`/`--spacing-*`/`--font-size-*`/`--radius-*`,按 `[data-density]`/`[data-font-size]` 选择器作用到 html。
- **组件样式必须用 CSS 变量而非硬编码 px**,否则不随主题风格档位变化。
- **Gotcha**:Pinia setup store 顶层解构 computed 是一次性快照非响应式,需用 `computed(() => store.xxx)`。
- 回退机制:打开主题工作室 `snapshotTheme()` 缓存 → 编辑实时改 DOM → 关闭弹窗 `applyCustomTheme()` 写盘;`revertTheme()` 恢复到打开前。**已去掉「应用主题」按钮**。
- 取色器为 `n-color-picker`（支持 rgba/alpha）；它渲染 fragment，需 `.picker-wrap` 容器 + `:deep` 剥 trigger 样式控制尺寸。

## AI 技能（skills/）

- 4 个内置技能：`apicat-lib`（Bridge 访问底座，其余技能的前置）、`apicat-edit`（增删改接口/字典/字段绑定、从 Markdown 文档批量导入）、`apicat-test-gen`（7 种类型用例生成 + 压测）、`apicat-doc-gen`（接口文档生成）。安装器名单见 `skill_installer.rs` 的 `SKILL_NAMES`。
- 技能经 HTTP Bridge 读写数据，**禁止直连 SQLite**；安装用 Windows junction（`mklink /J`，无需管理员权限）挂到各 Agent 的 skills 目录。
- App 启动时自动检测并修复悬空的 junction 链接（更新安装包后路径失效的场景）。
- 技能脚本里中文请求体必须走 heredoc/文件传入——内联参数会被 Git Bash 转码。

## 发布流程

- 版本号三处必须同步:package.json / src-tauri/tauri.conf.json / src-tauri/Cargo.toml
- git tag 格式 `v*.*.*`(如 v1.0.4),触发 `.github/workflows/release.yml` 构建;**推 tag 不会自动推分支**,`git push origin master v1.0.4` 分开推
- **构建完必须手动 Publish**:`releaseDraft: true` 建的是草稿,而 `/releases/latest` **不返回草稿**。不点 Publish → 客户端更新永久 404,且 `TopBar` 会静默降级,根本没人会发现
- **发版后务必验证**:浏览器开 `releases/latest/download/latest.json`,出 JSON 托算真的通了
- **改 runner 标签前先查是否已退役**:`macos-13` 于 2025-12-04 退役,该 job 永远拿不到 runner,排队满 24h 被取消会**连带整个矩阵运行 cancelled**、Release 建不出来(现用 `macos-15-intel`)
- **macOS 的 `bundles` 必须含 `app`**:更新载荷是 `.app.tar.gz` 而非 dmg,只传 dmg 会让 `latest.json` 缺失 `darwin-*` 条目,Mac 收不到更新

## Gotchas

- 代码/JSON 编辑区字号(monospace)不随主题缩放,属刻意排除
- `.url-highlight-layer` 需与 n-input medium 内边距同步,改动会错位
- dev 进程后台跑时 apicat.exe 存活,vite HMR 实时生效
