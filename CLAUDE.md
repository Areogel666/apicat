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
- **Form Data 文件字段（1.0.7）**：字段 `type='file'`，`value` 存本地绝对路径，**不复制文件、不落库文件元数据**（UI 按 stat 实时派生，历史/用例同样只存路径）。发送时 Rust 读盘构建 multipart part（`client.rs` form_data 分支，MIME 按扩展名推断、失败报错指明字段）；form_data 下手动 Content-Type 被忽略（boundary 由 reqwest 生成，前端发送时有提示）。文件选择走 `tauri-plugin-dialog`，存在性查询走自定义 `file_meta` command（不走 fs 插件——任意路径 scope 难配）；**文件拖拽走 Tauri `onDragDropEvent`（`dragDropEnabled: true`）+ `elementFromPoint` 定位**。压测拒绝含 `{{var}}` 的文件路径。
- **布局常驻**：`AppLayout` 中接口树/字典树/文档侧栏与 `MainPanel`/`DictionaryJsonPanel`/`StatsPanel` 都用 `v-show` 常驻（切换保留浏览状态），不要改回 `v-if` 卸载重挂。activity bar 四按钮中 **统计面板无侧栏**：折叠把手与 `ResizableSplitter` 的 `v-show` 带 `leftPanel !== 'stats'`，别删。

### HTTP Bridge

- `127.0.0.1:17320`（占用顺延）起 REST 服务，token 在 `$APPDATA/com.apicat.app/bridge.json`（macOS 在 `~/Library/Application Support/com.apicat.app/`）；路径对齐 IPC command 名，GET=只读 / POST=写。AI 技能读写数据一律走 Bridge，**禁止直连 SQLite**。
- **新端点取值一律用 `get_str/get_i64`（camel+snake 双键）**：手写单键会把另一命名风格的参数静默丢弃（不报错，字段直接丢失）。
- 写操作后广播 `bridge-data-changed`，前端 store 自行 reload——**新增写端点漏广播 → 技能改完数据桌面端看不见**。

### 写操作真源

- 写操作业务逻辑放 `commands/*_impl`，IPC 与 Bridge 共用同一份实现，**禁止两侧各写 SQL/业务分叉**。
- `update_request` 的 impl 是全量覆写；Bridge 侧先 SELECT 旧值合并未传字段再调 impl（COALESCE 便利留在协议层，UI 的 IPC 签名不变）。

### Cookie

- **三级作用域**：`cookies.scope_type` = `global` / `project` / `env`，覆盖优先级 `env > project > global`；同 `name`+`path` 取最高层那条。
- **匹配与注入的唯一真源是 `src-tauri/src/cookies.rs`**：`collect_cookies_for_url`（取某 URL 命中的 cookie）/ `filter_cookies`（domain 子域 + path 前缀 + 过期 + Secure + 优先级去重）/ `inject_cookies`（合并进请求头，顺序固定 `手动; 存储`）。五条链路（调试 Send / 用例 / 压测 / 导出 cURL / Bridge）全部走它，**不要各自内联 SQL**。
- **`secure` 参与过滤，`http_only` 不参与**：Secure 的 cookie 只在 https / wss 发送（`filter_cookies` 的 `secure_ok`）；HttpOnly 约束的是「浏览器页面脚本能否读 cookie」，ApiCat 不是浏览器、没有可执行该约束的对象，该字段只做记录。
- **查重必须走 `cookies::find_cookie_id`，且用 `IS ?` 而非 `=`**：`=` 遇 NULL 恒为 unknown，global / project 层（`project_id` / `env_id` 为 NULL）永远查不中。**也不要依赖表的 UNIQUE 约束**——SQLite 的 UNIQUE 索引遇 NULL 列不判等，它实际只对 env 层生效，global / project 层可无限插入重复行（实测确认）。
- **新建拦截 vs 回存覆盖是两条路径**：`create_cookie_impl`（手工新建）遇重复键**返回错误**；Set-Cookie 回存走独立的 `cookies::upsert_cookie`（命中即覆盖）。二者共用 `find_cookie_id`，改一处别牵连另一处。
- `expires_at` 落库是 **UTC 串**（`YYYY-MM-DD HH:MM:SS`，无时区标记）：前端比对用 `nowUtcString()`，date-picker 往返用 `tsToUtcString` / `utcStringToTs`（显式按 UTC 解析，别用 `new Date(s)`——该串的时区行为跨引擎不一致）。

### 用例与断言

- 断言 JSON 形状：`{status_code, json_path}`，运行结果给出通过/失败判定。
- Bridge `create_test_case` 的 `case_type` **必填**（缺省返回 400，防止 7 种用例类型被静默全标成 happy_path）；UI 侧默认 happy_path。
- ~~收藏用例~~ 概念已随 1.0.5 移除（DB 迁移 DROP COLUMN starred）；用例无收藏/置顶，仅按最近使用排序。

### 历史列表契约

- 前端 `list_history` 只取元数据（body/snapshot/headers 三个大字段为 NULL），diff / 回填用 `get_history_record(id)` 单条补拉。
- **Bridge 的 `list_history` 仍返回全量字段**——外部技能脚本依赖此契约，勿「顺手统一」。
- `test_case_history` 表已合并进 `request_history`（用例历史同表，`test_case_id` 不为空）；由触发器 `trg_rh_keep_10_per_case` 按用例保留最近 10 条。
- **大响应落文件**：发送时超阈值则 body 存文件，`response_body` 为路径标记 + `is_truncated=1`；取回用 `open_response_file`，导出用 `export_response_file`，`cleanup_history` 可连带清理文件。

### 压测

- **引擎不做 `{{var}}` 替换**（与正常发送的 Rust 侧 `replace_variables` 不同）；UI 从用例发起压测时预替换 `{{base_url}}`，最终 URL 仍含 `{{...}}` 或无激活环境 → `message.warning` 提示，不静默发压测。
- **报告真源是 Rust `build_stress_report_markdown`**（App 预览/导出与 Bridge 共用），改报告只改这一处。
- **参考线阈值**：项目级默认（`projects.p95/p99_threshold_ms`，500/1000）+ 接口级覆盖（`api_requests`，null=继承）。压测启动时生效值快照写入 `stress_runs.config_json`；报告只读 config_json → 常量默认，不回查项目/接口表（旧记录无字段 → 用默认值，改阈值不影响历史报告）。

### 新手指引（guide）

- 状态 = `app-settings.json` 的 `guide.*`（`src/stores/guide.ts` 唯一读写点）；文案集中 `src/config/guideContent.ts`——**加一条引导 = GuideId + 文案 + 挂点三处联动**，键不存在 = 未看过（老用户升级自然触发）
- **两种形态语义刻意不同，别混用**：`GuideBar`（弹窗内提示条）= **显示即记 seen**（本地 show 与 seen 解耦，弹窗关了也算了）；`GuideTip`（popover）= 点「知道了」才记 seen。弹窗是从菜单点开的、菜单项点击即消失，**锚点不存在，弹窗内不能用 popover**
- 播种仅走 `loadProjects()` 空项目分支（`seedSampleData`，后台跑、失败仅 warn）；**`update_request` IPC 参数全必填**（缺一被 Tauri reject 且被 catch 吞），**`update_test_case` 的 method/url 会被覆写必须回传**——两个坑都踩过，seed 调用别精简参数
- 播种完成 `sidebarReloadTick.value++` 强刷接口树（同 ImportDialog 模式），否则首启树是空的
- 重放 = ⚙️ 菜单 `guideReset` → `resetAll()`；popover 靠响应式自动补弹，无需额外代码

### 使用统计（stats）

- 唯一真源 `collect_usage_stats`（`src-tauri/src/commands/stats.rs`，无 Bridge 端点）：请求类指标**恒带 `test_case_id IS NULL`**（用例行有每用例 10 条触发器裁剪，混入必失真）；成功率 = 2xx/3xx，NULL/≥400 失败；状态桶 CASE 顺序 `IS NULL→网络失败 / <400 / <500 / else`；方法分布 `json_extract(request_snapshot,'$.method')`（快照是 ApiRequest serde 串，method 单词键大小写免疫）
- 档位 `30d/180d/365d/all`：KPI（请求数/成功率/压测）与分布跟档；**接口总数/字典/文档恒存量全量**。趋势粒度 ≤90 天按日、更久按周；固定窗前端补零、`all` 不补零
- **周分组 SQL 必须 `date(created_at,'-6 days','weekday 1')`，修饰符顺序不可反**——`weekday 1` 遇当天已是周一会原地不动，反序会把周一错归上一周（实测踩坑；测试断言周桶 `%w=1` 锁住）
- 字段名两侧镜像：`UsageStats`（stats.rs ↔ src/types/index.ts），`requestsCount/successRate` 不带 30d 字样；空库哨兵 `successRate=-1` → 前端显示「—」
- 文档指标只扫已配 `docs_output_dir` 的项目（Rust 侧无默认路径回退，别臆造 `~/.apicat` 回退），区块有说明文案
- 图表色从 CSS var 取计算值 + MutationObserver 监听 `data-theme/density/font-size` 重算（canvas 读不到 CSS var）；`StatsChart` 是 echarts/core 按需薄封装，**勿整包 import echarts**

### 编辑撤销 / 草稿恢复

- **结构级 undo/redo**：`MainPanel.vue` 的 `pushUndo()` 在 add/remove/splice/排序/URL/method 生效前调用，按 requestId 分栈、栈深 20。文本输入不 pushUndo（走原生撤销）——新增结构操作必须前置 pushUndo，否则 ctrl+z 撤销会跳过它。执行 undo/redo 包 `isInitializing`，防 watcher 连锁。
- **回到上次保存**：Ctrl+S 成功后深拷贝存 `savedSnapshots[req.id]`；编辑区「⚠ 未保存」时点「回到上次保存版」应用快照，`isInitializing` 包住，不推入 undo 栈。

## Rust 侧加字段清单

给表加列必须一次改齐四处，漏一处 = sqlx 反序列化错位或运行时列不存在（本项目已踩过两次）：

1. migration（或 `db/mod.rs` 的 pragma 守卫式 ALTER）
2. `src-tauri/src/types.rs` 对应 struct——**字段顺序 = SELECT 列顺序**
3. `src-tauri/src/sql_cols.rs` 对应 `*_COLS`（commands 与 bridge 唯一事实源，勿在两侧再写列清单）
4. `src/types/index.ts` 镜像 interface（两侧文件头有同步约定注释，改任一侧必须同步另一侧）

## 主题系统(themeStore)

- `src/stores/theme.ts`:setup store。`customTokens` 增量覆盖 + `density`/`radiusScale`/`fontSize` → `resolvedTokens` computed → `applyTheme()` 写 `<html data-theme/data-density/data-font-size>` + inline style。
- `src/styles/tokens.css`:风格变量 `--row-height`/`--input-height`/`--spacing-*`/`--font-size-*`/`--radius-*`,按 `[data-density]`/`[data-font-size]` 选择器作用到 html。
- **组件样式一律走项目主题 token，禁止硬编码 px / 色值**。间距用 `--spacing-xs|sm|md|lg`、字号用 `--font-size-sm|base|lg`、行高用 `--row-height`/`--input-height`、圆角用 `--radius-*`、颜色边框用 `--bg-*`/`--text-*`/`--border-*`/`--color-*`。这些变量随密度/字号档位变，写死 px 的后果是用户改了风格档位那一块不跟着变。**唯一例外**：列宽、`min-width`、控件宽度这类布局尺寸可以写 px——它们不是密度语义。自定义组件参照 `src/components/cookie/` 三个文件的写法。
- **Gotcha**:Pinia setup store 顶层解构 computed 是一次性快照非响应式,需用 `computed(() => store.xxx)`。
- 回退机制:打开主题工作室 `snapshotTheme()` 缓存 → 编辑实时改 DOM → 关闭弹窗 `applyCustomTheme()` 写盘;`revertTheme()` 恢复到打开前。**已去掉「应用主题」按钮**。
- 取色器为 `n-color-picker`（支持 rgba/alpha）；它渲染 fragment，需 `.picker-wrap` 容器 + `:deep` 剥 trigger 样式控制尺寸。

## AI 技能（skills/）

- 4 个内置技能：`apicat-lib`（Bridge 访问底座，其余技能的前置）、`apicat-edit`（增删改接口/字典/字段绑定、从 Markdown 文档批量导入）、`apicat-test-gen`（7 种类型用例生成 + 压测）、`apicat-doc-gen`（接口文档生成）。安装器名单见 `skill_installer.rs` 的 `SKILL_NAMES`。
- 技能经 HTTP Bridge 读写数据，**禁止直连 SQLite**；安装用 Windows junction（`mklink /J`，无需管理员权限）挂到各 Agent 的 skills 目录。
- App 启动时自动检测并修复失效的技能安装（链接悬空 / 链接指向旧版 App 残留目录 / 副本版本标记过期）。
- **技能版本 ≡ App 版本**（技能随 App 打包，无独立版本号）；判定唯一真源是 `skill_installer::skill_state`（UI 展示与启动静默修复共用，别写成两套）。链接的判据是「**指向当前 App 的内置技能目录**」而非「没悬空」——旧 App 残留目录还在磁盘时链接并不悬空，只判悬空会漏掉「技能内容已过期」。
- 技能脚本里中文请求体必须走 heredoc/文件传入——内联参数会被 Git Bash 转码。

## 发布流程

- 版本号三处必须同步:package.json / src-tauri/tauri.conf.json / src-tauri/Cargo.toml
- git tag 格式 `v*.*.*`(如 v1.0.4),触发 `.github/workflows/release.yml` 构建;**推 tag 不会自动推分支**,`git push origin master v1.0.4` 分开推
- **构建完必须手动 Publish**:`releaseDraft: true` 建的是草稿,而 `/releases/latest` **不返回草稿**。不点 Publish → 客户端更新永久 404,且 `TopBar` 会静默降级,根本没人会发现
- **发版后务必验证**:浏览器开 `releases/latest/download/latest.json`,出 JSON 托算真的通了
- **改 runner 标签前先查是否已退役**:`macos-13` 于 2025-12-04 退役,该 job 永远拿不到 runner,排队满 24h 被取消会**连带整个矩阵运行 cancelled**、Release 建不出来(现用 `macos-15-intel`)
- **macOS 的 `bundles` 必须含 `app`**:更新载荷是 `.app.tar.gz` 而非 dmg,只传 dmg 会让 `latest.json` 缺失 `darwin-*` 条目,Mac 收不到更新

## Gotchas

- **dragDropEnabled 必须 true（1.0.7）**：实测 Linux webkitGTK 在 false 时不把外部文件拖拽派发为 DOM 事件（dragenter/over/drop 全不触发），且会把文件内容按 Latin-1 读出插进输入框（乱码根源）；true 后 drop 在 GTK 层被 Tauri 接管。代价是 NTree 原生 draggable 全平台失效——侧边栏排序已换成 pointer 自研引擎 `useTreePointerDnd`（复刻 NTree 三段落点语义，产出等价 drop 信息调 `useCollectionTreeDnd.onDrop`，业务逻辑零改动）。**别把 NTree 的 draggable 加回去**，也别把 conf 改回 false
- **调试前端事件别用 `document.title` 取证**：Tauri 窗口上 webview 的 title 变化**不会同步到 GTK/系统窗口标题**，探针永远读到旧值（1.0.7 拖拽调试因此绕了多轮）。可靠通道：自定义 Rust command 写文件（如追加写 /tmp 日志，无 capability 限制）
- 代码/JSON 编辑区字号(monospace)不随主题缩放,属刻意排除
- `.url-highlight-layer` 需与 n-input medium 内边距同步,改动会错位
- dev 进程后台跑时 apicat.exe 存活,vite HMR 实时生效
