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

### 侧栏树折叠（1.0.9）

- 三棵树（接口/字典/文档）统一受控展开，持久化真源是 `useTreeExpanded`（`src/composables/useTreeExpanded.ts`）：watch 实时写 localStorage `tree.expanded.{name}:{projectId}`，按「树×项目」记忆；恢复走 `restore()`（严格语义：过滤已删节点，新建默认收起）
- 接口树搜索期间 `setPaused(true)` 暂停写盘（搜索自动全展开是瞬时态），清空搜索恢复快照；切项目时恢复写盘
- ⊞/⊟（全部展开/折叠）按钮直接赋值 expandedKeys，与手动收展共用同一条持久化链路

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
- `GET /list_requests` 双参二选一（1.0.9）：`collection_id`（单目录）或 `project_id`（项目全量扁平）；都缺省 400。**400 文案与参数是 snake_case**——serde 无 camelCase 映射，照 `collectionId` 拼永远 400
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

### 默认请求头 / 响应解压

- **默认 UA**：两个 HTTP client（调试 `lib.rs` / 压测 `stress.rs`）统一 `.user_agent("ApiCat/<CARGO_PKG_VERSION>")`；用户手动 `User-Agent` 行优先覆盖。
- **响应解压默认关**：`app-settings.json` 的 `network.decompressEnabled`（bool）+ `network.decompressAlgorithms`（数组，默认 `gzip/deflate/br`，zstd 需手动勾）。开关变化走 IPC `apply_network_settings(enabled, algorithms)` 即时重建主 client——**直接传值，别让 Rust 回读文件**（plugin-store 落盘有防抖，回读会拿到旧值）。压测 client 每次运行读同一份设置，并把 `accept_encoding` 写进 `stress_runs.config_json`。
- **Headers 只读区**展示 App 真正注入的头：User-Agent / Content-Type / Authorization / Accept-Encoding，以及经 `get_cookies_for_domain` 拿到的命中 Cookie（真源仍是 `cookies::collect_cookies_for_url`）。只读、不进签名/落库；手动写 AE 且解压关闭时给乱码提示。
- **URL 域名来源**：URL 自带 `http(s)://` 时环境 `base_url` 完全不参与拼接；URL 栏用「URL 域名 / 环境域名」徽标常驻提示，两边 host 不一致时发送前再 `message.info` 一次。预览框只显示 base+path，**不含 Query 参数**（文案是「请求地址预览」），实际发送由 Rust 追加 `query_params`。
- **响应时间戳悬停**：`JsonTree` 对「字段名像时间（timestamp/_at/_ts/_time 等）+ 值 10/13 位且落在 2000–2100 年」的标量挂原生 `title`，显示本地时间 + UTC；只覆盖 JSON 美化视图，raw/文本/代码视图不处理。规则唯一实现在 `src/utils/unixTimestamp.ts`（带 `.check.ts` 自检）。

### 用例与断言

- 断言 JSON 形状：`{status_code, json_path}`，运行结果给出通过/失败判定。
- Bridge `create_test_case` 的 `case_type` **必填**（缺省返回 400，防止 7 种用例类型被静默全标成 happy_path）；UI 侧默认 happy_path。
- **用例执行默认继承接口认证**（`test_cases` 没有 auth 列，1.0.10 起从 `api_requests` 读 `auth_type/auth_config`），并注入存储 cookie；**唯一例外是 `case_type='unauthorized'`**——豁免认证与 cookie 注入，否则接口认证/登录态会被自动补上，401 断言恒挂。删用例 headers 挡不住注入（认证优先级高于手动 headers，cookie 是「手动; 存储」合并）。
- ~~收藏用例~~ 概念已随 1.0.5 移除（DB 迁移 DROP COLUMN starred）；用例无收藏/置顶，仅按最近使用排序。

### 历史列表契约

- 前端 `list_history` 只取元数据（body/snapshot/headers 三个大字段为 NULL），diff / 回填用 `get_history_record(id)` 单条补拉。
- **Bridge 的 `list_history` 仍返回全量字段**——外部技能脚本依赖此契约，勿「顺手统一」。
- `test_case_history` 表已合并进 `request_history`（用例历史同表，`test_case_id` 不为空）；由触发器 `trg_rh_keep_10_per_case` 按用例保留最近 10 条。
- **大响应落文件**：发送时超阈值则 body 存文件，`response_body` 为路径标记 + `is_truncated=1`；取回用 `open_response_file`，导出用 `export_response_file`，`cleanup_history` 可连带清理文件。
- **回填补元数据**：Rust `ParamItem` 已带 `description` 随快照落库；`handleRefill` 仍用 `mergeParamMeta` 以当前接口定义补旧快照的 type/description，并把 KV/JSON 模式重置为表格、清空文本缓冲（否则旧文本会覆盖回填值）。

### 压测

- **引擎不做 `{{var}}` 替换**（与正常发送的 Rust 侧 `replace_variables` 不同）；UI 从用例发起压测时预替换 `{{base_url}}`，最终 URL 仍含 `{{...}}` 或无激活环境 → `message.warning` 提示，不静默发压测。
- **报告真源是 Rust `build_stress_report_markdown`**（App 预览/导出与 Bridge 共用），改报告只改这一处。
- **压测 tab 完成自动弹报告的协调在 MainPanel**（`tabStressPending` 区分两个入口，modal 入口发起不叠加）；done 事件先 emit 后落库，前端延迟 300ms 再拉历史
- 压测历史时间 `formatTime` 必须经 `utcStringToTs` 显式 UTC 解析——`new Date('YYYY-MM-DD HH:MM:SS')` 在 WebKit 按本地时区解读，整体偏移时区差（实测踩坑）
- 曲线区高度可拖（`layout.stressChartHeight`，ResizableSplitter 120–600px），历史/对比内容在历史区内部滚动
- **参考线阈值**：项目级默认（`projects.p95/p99_threshold_ms`，500/1000）+ 接口级覆盖（`api_requests`，null=继承）。压测启动时生效值快照写入 `stress_runs.config_json`；报告只读 config_json → 常量默认，不回查项目/接口表（旧记录无字段 → 用默认值，改阈值不影响历史报告）。

### 新手指引（guide）

- 状态 = `app-settings.json` 的 `guide.*`（`src/stores/guide.ts` 唯一读写点）；文案集中 `src/config/guideContent.ts`——**加一条引导 = GuideId + 文案 + 挂点三处联动**，键不存在 = 未看过（老用户升级自然触发）
- **形态 = tour + GuideBar 两种**（1.0.9 起面板级 GuideTip 已移除——与首启 tour 重复）：`GuideBar`（弹窗内提示条）= **显示即记 seen**（本地 show 与 seen 解耦，弹窗关了也算了）；面板/全局导览统一由首启 tour 承担（`useAppTour`，driver.js highlight 单步模式自驱——每步先切面板、等 DOM 就绪再高亮；X/Esc/遮罩三条关闭路径统一走 `onDestroyStarted`，`finish()` 有防重入标志）
- **tour 一次性语义**：`guide.tour` seen = 应用生涯一次，不随版本重弹；启动串行 tour 先行（StartupChecks，等播种完成信号或 800ms 兜底）且 await 结束，技能引导/更新检查在其后——中断也不吞；⚙️ guideReset 会重开 tour
- 播种仅走 `loadProjects()` 空项目分支（`seedSampleData`，后台跑、失败仅 warn）；**`update_request` IPC 参数全必填**（缺一被 Tauri reject 且被 catch 吞），**`update_test_case` 的 method/url 会被覆写必须回传**——两个坑都踩过，seed 调用别精简参数
- 播种完成 `sidebarReloadTick.value++` 强刷接口树（同 ImportDialog 模式），否则首启树是空的
- 重放 = ⚙️ 菜单 `guideReset` → `resetAll()`；弹窗内 GuideBar 靠响应式自动补弹，无需额外代码

### 使用统计（stats）

- 唯一真源 `collect_usage_stats`（`src-tauri/src/commands/stats.rs`，无 Bridge 端点）：请求类指标**恒带 `test_case_id IS NULL`**（用例行有每用例 10 条触发器裁剪，混入必失真）；成功率 = 2xx/3xx，NULL/≥400 失败；状态桶 CASE 顺序 `IS NULL→网络失败 / <400 / <500 / else`；方法分布 `json_extract(request_snapshot,'$.method')`（快照是 ApiRequest serde 串，method 单词键大小写免疫）
- **级联筛选（1.0.9）**：签名 `get_usage_stats(range, projectId?, requestId?)`；范围过滤统一抽在 `scope_filter_sql`（接口级 `request_id` 有索引直查，项目级经 api_requests→collections 两跳）；**请求类指标全跟筛选走**（含压测次数，stress_runs 同 scope 片段）；接口总数项目级=该项目存量、接口级=1；字典/文档存量跟随项目口径（项目自有+全局共享，与字典树一致），接口级时直接零值并跳过磁盘扫描（前端该 scope 隐藏区块，别做无效功）
- 接口下拉数据走 IPC `list_project_requests`（项目扁平一次查），seq 守卫防快速连切竞态
- 档位 `30d/180d/365d/all`：KPI（请求数/成功率/压测）与分布跟档；**接口总数/字典/文档恒存量全量**（筛选态除外，见上）；趋势粒度 ≤90 天按日、更久按周；固定窗前端补零、`all` 不补零
- **周分组 SQL 必须 `date(created_at,'-6 days','weekday 1')`，修饰符顺序不可反**——`weekday 1` 遇当天已是周一会原地不动，反序会把周一错归上一周（实测踩坑；测试断言周桶 `%w=1` 锁住）
- 字段名两侧镜像：`UsageStats`（stats.rs ↔ src/types/index.ts），`requestsCount/successRate` 不带 30d 字样；空库哨兵 `successRate=-1` → 前端显示「—」
- 文档指标只扫已配 `docs_output_dir` 的项目（Rust 侧无默认路径回退，别臆造 `~/.apicat` 回退），区块有说明文案
- 图表色从 CSS var 取计算值 + MutationObserver 监听 `data-theme/density/font-size` 重算（canvas 读不到 CSS var）；`StatsChart` 是 echarts/core 按需薄封装，**勿整包 import echarts**

### 编辑撤销 / 草稿恢复

- **结构级 undo/redo**：`MainPanel.vue` 的 `pushUndo()` 在 add/remove/splice/排序/URL/method 生效前调用，按 requestId 分栈、栈深 20。文本输入不 pushUndo（走原生撤销）——新增结构操作必须前置 pushUndo，否则 ctrl+z 撤销会跳过它。执行 undo/redo 包 `isInitializing`，防 watcher 连锁。
- **回到上次保存**：Ctrl+S 成功后深拷贝存 `savedSnapshots[req.id]`；编辑区「⚠ 未保存」时点「回到上次保存版」应用快照，`isInitializing` 包住，不推入 undo 栈。
- **黄点即时性**：`formDataParams` / `urlencodedParams` 有 deep watcher；撤销/重做/↺/切换接口/退出用例/历史回填后在 `nextTick` 补一次 `markRequestDirty()`。切走静默落库后 `persistParams` 同步更新 store 基准，仅 method/url 改动仍保留黄点；Auth 保存后也重算而非无条件清脏。
- **黄点签名归一化**：URL 只比较 base（query 由 `queryParams` 单独管，实际发送时 Rust 也剥掉 URL query 再拼 params）；ParamItem 固定字段顺序并统一 `null/undefined/""`；body 按类型归一（表单数组走 `paramSignature`、raw_json 忽略格式空白、form_urlencoded 兼容旧 k=v）。
- **切接口 watcher 必须同 id 短路**：`updateRequest` 会替换 `requestMap` 元素，`persistParams` 必须**原地改字段**而不是替换元素；否则 `watch(activeRequest)` 把同 id 替换误当切接口，触发「存草稿 → 落库 → 再替换」循环，并清掉用例上下文。

## Rust 侧加字段清单

给表加列必须一次改齐四处，漏一处 = sqlx 反序列化错位或运行时列不存在（本项目已踩过两次）：

1. migration（或 `db/mod.rs` 的 pragma 守卫式 ALTER）
2. `src-tauri/src/types.rs` 对应 struct——**字段顺序 = SELECT 列顺序**
3. `src-tauri/src/sql_cols.rs` 对应 `*_COLS`（commands 与 bridge 唯一事实源，勿在两侧再写列清单）
4. `src/types/index.ts` 镜像 interface（两侧文件头有同步约定注释，改任一侧必须同步另一侧）

## 主题系统(themeStore)

- `src/stores/theme.ts`:setup store。`customTokens` 增量覆盖 + `density`/`radiusScale`/`fontSize` → `resolvedTokens` computed → `applyTheme()` 写 `<html data-theme/data-density/data-font-size>` + inline style。
- **圆角联动（1.0.9）**：`naiveOverrides` 必须带 `borderRadius`/`borderRadiusSmall`（= 6/4 × radiusScale）——漏了的话 NaiveUI 控件永远内置 3px 直角，圆角预设形同虚设（1.0.9 之前一直如此）
- **玻璃质感（1.0.9）**：`glass`/`glassBlur`/`glassAlpha` 三状态；半透明表面色由 `glassSurfaceTokens(mode, alphaLevel)` 按「深浅模式 + 档位」动态注入 inline style，**绝不放 customTokens**（跨主题持久，深色 rgba 切浅色会不可读）；blur 走 `--glass-blur` 变量由 glass.css 消费；alpha 档位换整串 rgba（CSS Color 4 的 `rgb(/a)` 旧 WebKit 不支持）；冰晶预设只负责「打开」glass，非玻璃预设不强制关（手动开关独立，深蓝+玻璃也行）
- `src/styles/tokens.css`:风格变量 `--row-height`/`--input-height`/`--spacing-*`/`--font-size-*`/`--radius-*`,按 `[data-density]`/`[data-font-size]` 选择器作用到 html。
- **组件样式一律走项目主题 token，禁止硬编码 px / 色值**。间距用 `--spacing-xs|sm|md|lg`、字号用 `--font-size-sm|base|lg`、行高用 `--row-height`/`--input-height`、圆角用 `--radius-*`、颜色边框用 `--bg-*`/`--text-*`/`--border-*`/`--color-*`。这些变量随密度/字号档位变，写死 px 的后果是用户改了风格档位那一块不跟着变。**唯一例外**：列宽、`min-width`、控件宽度这类布局尺寸可以写 px——它们不是密度语义。自定义组件参照 `src/components/cookie/` 三个文件的写法。
- **Gotcha**:Pinia setup store 顶层解构 computed 是一次性快照非响应式,需用 `computed(() => store.xxx)`。
- 回退机制:打开主题工作室 `snapshotTheme()` 缓存 → 编辑实时改 DOM → 关闭弹窗 `applyCustomTheme()` 写盘;`revertTheme()` 恢复到打开前。**已去掉「应用主题」按钮**。
- 取色器为 `n-color-picker`（支持 rgba/alpha）；它渲染 fragment，需 `.picker-wrap` 容器 + `:deep` 剥 trigger 样式控制尺寸。

## AI 技能（skills/）

- 5 个内置技能：`apicat-lib`（Bridge 访问底座，其余技能的前置）、`apicat-edit`（增删改接口/字典/字段绑定、从 Markdown 文档批量导入）、`apicat-test-gen`（7 种类型用例生成 + 压测）、`apicat-doc-gen`（接口文档生成）、`apicat-doctor`（应用问诊：只读扫描出分级发现清单与整理计划，问答定稿，**绝不实施**——SKILL 写死仅允许 GET）。安装器名单见 `skill_installer.rs` 的 `SKILL_NAMES`。
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

- **历史时间显示必须显式 UTC 解析**：DB 落库 UTC 串（无时区标记），`new Date(串)` 在 WebKit 按本地时区解读整体偏移（压测历史实测踩坑）——用 `utcStringToTs`（utils/time.ts）或 `Date.parse(s+'Z')`（HistoryTab 同款）
- **dragDropEnabled 必须 true（1.0.7）**：实测 Linux webkitGTK 在 false 时不把外部文件拖拽派发为 DOM 事件（dragenter/over/drop 全不触发），且会把文件内容按 Latin-1 读出插进输入框（乱码根源）；true 后 drop 在 GTK 层被 Tauri 接管。代价是 NTree 原生 draggable 全平台失效——侧边栏排序已换成 pointer 自研引擎 `useTreePointerDnd`（复刻 NTree 三段落点语义，产出等价 drop 信息调 `useCollectionTreeDnd.onDrop`，业务逻辑零改动）。**别把 NTree 的 draggable 加回去**，也别把 conf 改回 false
- **调试前端事件别用 `document.title` 取证**：Tauri 窗口上 webview 的 title 变化**不会同步到 GTK/系统窗口标题**，探针永远读到旧值（1.0.7 拖拽调试因此绕了多轮）。可靠通道：自定义 Rust command 写文件（如追加写 /tmp 日志，无 capability 限制）
- 代码/JSON 编辑区字号(monospace)不随主题缩放,属刻意排除
- `.url-highlight-layer` 需与 n-input medium 内边距同步,改动会错位
- **URL 框双击 `{{var}}` 打开环境管理（1.0.9）**：判定「双击选中词范围中点落在任一 `{{...}}` 区间内」（高亮层 pointer-events:none，事件来自原生 input 的 selection）；打开走 envStore `requestOpenManager()` tick 信号 → TopBar watch——弹窗 show 留在 TopBar 本地，跨组件只传信号（同 sidebarReloadTick 模式）
- dev 进程后台跑时 apicat.exe 存活,vite HMR 实时生效
