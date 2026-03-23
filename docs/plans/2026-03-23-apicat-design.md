# ApiCat — v1.0 设计文档

> 创建日期：2026-03-23  
> 状态：待实现

---

## 一、项目概述

ApiCat 是一个类 Postman 的桌面端 API 接口调试工具，面向个人开发者和小团队。  
第一版（v1.0）聚焦核心调试体验，涵盖接口管理、调试、环境变量、Cookie、压测、导入导出等基础功能。

---

## 二、技术栈

| 层 | 技术 |
|---|---|
| 桌面容器 | Tauri 2.x |
| 前端 | Vue 3 + TypeScript + Vite |
| UI 组件库 | Naive UI |
| 状态管理 | Pinia |
| 后端逻辑 | Rust（Tauri Commands） |
| 数据库 | SQLite（via `sqlx`） |
| HTTP 客户端 | Rust `reqwest`（异步，支持 HTTP/1.1 + HTTP/2） |
| 压测引擎 | Rust `tokio` 多任务并发 |

### 为什么 Rust 层发请求而非前端 fetch？

- 无浏览器 CORS 限制，可请求任意域名
- 压测性能：tokio 异步并发，能跑数百并发不卡 UI
- 可控制 HTTPS 证书校验（忽略自签证书）
- 完整 Cookie jar 控制

---

## 三、数据模型

### 3.1 实体关系

```
Project (项目)
  ├── Environment (环境：dev/test/prod)
  │     └── EnvVariable (环境变量 key-value)
  ├── Collection (接口集合/文件夹，可嵌套)
  │     └── ApiRequest (接口定义)
  │           └── RequestHistory (调试历史，自动保存)
  └── Cookie (项目级 Cookie，按域名)

GlobalCookie (全局 Cookie，跨项目，按域名)
```

### 3.2 SQLite 表结构

#### `projects` — 项目

```sql
CREATE TABLE projects (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  name        TEXT    NOT NULL,
  description TEXT,
  created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### `environments` — 环境

```sql
CREATE TABLE environments (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name       TEXT    NOT NULL,       -- e.g. "开发", "测试", "生产"
  base_url   TEXT,                   -- 该环境的 base URL
  is_active  INTEGER DEFAULT 0,      -- 当前激活的环境（每个项目只有一个）
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### `env_variables` — 环境变量

```sql
CREATE TABLE env_variables (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  env_id      INTEGER NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
  key         TEXT    NOT NULL,
  value       TEXT    NOT NULL DEFAULT '',
  description TEXT,
  enabled     INTEGER DEFAULT 1      -- 0=禁用，1=启用
);
```

> 变量替换规则：URL / Headers / Body 中的 `{{variable_name}}` 在 Rust 层发请求前替换。  
> 优先级：项目级环境变量 > 全局变量。

#### `collections` — 接口分组（支持无限嵌套）

```sql
CREATE TABLE collections (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  parent_id  INTEGER REFERENCES collections(id) ON DELETE CASCADE,  -- null = 根节点
  name       TEXT    NOT NULL,
  sort_order INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### `api_requests` — 接口定义

```sql
CREATE TABLE api_requests (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  name          TEXT    NOT NULL,
  method        TEXT    NOT NULL DEFAULT 'GET',  -- GET/POST/PUT/DELETE/PATCH/HEAD/OPTIONS
  url           TEXT    NOT NULL DEFAULT '',
  -- Params、Headers、Body 均存为 JSON 数组，元素结构：{key, value, enabled, description}
  params        TEXT    DEFAULT '[]',            -- query params
  headers       TEXT    DEFAULT '[]',
  body_type     TEXT    DEFAULT 'none',          -- none | raw_json | raw_text | form_data | form_urlencoded
  body          TEXT    DEFAULT '',
  -- Auth
  auth_type     TEXT    DEFAULT 'none',          -- none | bearer | basic | api_key
  auth_config   TEXT    DEFAULT '{}',            -- JSON：{ token } / { username, password } / { key, value, in }
  sort_order    INTEGER DEFAULT 0,
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### `request_history` — 调试历史

```sql
CREATE TABLE request_history (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id          INTEGER NOT NULL REFERENCES api_requests(id) ON DELETE CASCADE,
  status_code         INTEGER,
  response_time_ms    INTEGER,
  request_snapshot    TEXT    NOT NULL,   -- JSON：完整请求快照（method/url/headers/body/...）
  response_body       TEXT,
  is_truncated        INTEGER DEFAULT 0,  -- 响应体超出 2MB 截断阈值时为 1
  response_headers    TEXT    DEFAULT '{}',
  created_at          DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### `cookies` — Cookie（两层：全局 + 项目级）

```sql
CREATE TABLE cookies (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  -- scope_type = 'global' 时 project_id 为 null
  scope_type TEXT    NOT NULL DEFAULT 'global',   -- global | project
  project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
  domain     TEXT    NOT NULL,
  name       TEXT    NOT NULL,
  value      TEXT    NOT NULL DEFAULT '',
  path       TEXT    DEFAULT '/',
  expires_at DATETIME,                            -- null = session cookie
  http_only  INTEGER DEFAULT 0,
  secure     INTEGER DEFAULT 0,
  enabled    INTEGER DEFAULT 1,
  -- 防止同名 Cookie 重复积累：同一作用域 + 项目 + 域名 + 路径 + 名称唯一
  UNIQUE(scope_type, project_id, domain, path, name)
);
```

> Cookie 匹配规则：请求域名与 `domain` 字段做后缀匹配。  
> 发请求时，项目级 Cookie 优先于全局 Cookie（同名同域名时项目级覆盖）。

#### `test_cases` — 自动化测试用例

```sql
CREATE TABLE test_cases (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  -- 关联到接口，同时关联到 collection 便于分组展示
  request_id    INTEGER REFERENCES api_requests(id) ON DELETE SET NULL,
  collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
  -- 用例基本信息
  name          TEXT    NOT NULL,                 -- 用例名称，如 "GET /users - 正常请求"
  description   TEXT,                             -- 用例描述
  source        TEXT    DEFAULT 'manual',         -- 来源：manual | ai_generated
  -- 请求覆盖（可覆盖关联接口的默认值，为空则继承 api_requests 的值）
  method        TEXT,                             -- 覆盖 HTTP Method（null = 继承）
  url           TEXT,                             -- 覆盖 URL（null = 继承，支持 {{var}}）
  headers       TEXT    DEFAULT '[]',             -- 覆盖请求头 JSON 数组
  params        TEXT    DEFAULT '[]',             -- 覆盖 Query Params JSON 数组
  body_type     TEXT,                             -- 覆盖 Body 类型（null = 继承）
  body          TEXT,                             -- 覆盖请求体（null = 继承）
  -- 断言（期望的响应结果）
  assertions    TEXT    NOT NULL DEFAULT '[]',    -- JSON 数组，见断言格式说明
  -- 用例执行结果（最近一次）
  last_run_at   DATETIME,
  last_status   TEXT    DEFAULT 'pending',        -- pending | passed | failed | error
  last_duration_ms INTEGER,
  last_response TEXT,                             -- 最近一次响应体（用于 diff）
  -- 收藏与清理
  starred       INTEGER DEFAULT 0,               -- 1=已收藏（不会被定时清理）
  -- 元数据
  enabled       INTEGER DEFAULT 1,               -- 0=禁用
  sort_order    INTEGER DEFAULT 0,
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

**断言格式（`assertions` 字段 JSON 数组）：**

每条断言是一个对象，格式如下：

```json
[
  { "type": "status_code",  "operator": "eq",       "expected": 200 },
  { "type": "response_time","operator": "lt",        "expected": 500 },
  { "type": "body_json",    "operator": "json_path", "path": "$.code",    "expected": 0 },
  { "type": "body_json",    "operator": "json_path", "path": "$.data.id", "expected": "not_null" },
  { "type": "header",       "operator": "contains",  "key": "Content-Type", "expected": "application/json" }
]
```

| `type` | 断言对象 |
|--------|---------|
| `status_code` | HTTP 状态码 |
| `response_time` | 响应耗时（ms） |
| `body_json` | JSON 响应体（支持 JSONPath） |
| `body_text` | 响应体原始文本 |
| `header` | 响应头 |

| `operator` | 含义 |
|------------|------|
| `eq` | 等于 |
| `ne` | 不等于 |
| `lt` / `gt` | 小于 / 大于 |
| `contains` | 包含字符串 |
| `json_path` | JSONPath 取值后比较 |
| `not_null` | 值不为 null/空 |

> `test_cases` 表通过 `collection_id` 归属到某个专属 Collection（如项目下自动创建的「🤖 AI 测试用例」分组），在 ApiCat 侧边栏中与普通接口并列展示，可单条运行或批量运行。

### 3.3 索引定义

外键列必须建索引，否则级联删除（如删除 Project）会触发全表扫描和长写锁：

```sql
-- 外键列索引（级联删除性能保障）
CREATE INDEX IF NOT EXISTS idx_env_proj    ON environments(project_id);
CREATE INDEX IF NOT EXISTS idx_coll_proj   ON collections(project_id);
CREATE INDEX IF NOT EXISTS idx_coll_parent ON collections(parent_id);
CREATE INDEX IF NOT EXISTS idx_req_coll    ON api_requests(collection_id);
CREATE INDEX IF NOT EXISTS idx_hist_req    ON request_history(request_id);
CREATE INDEX IF NOT EXISTS idx_tc_req      ON test_cases(request_id);
CREATE INDEX IF NOT EXISTS idx_tc_coll     ON test_cases(collection_id);
```

> 以上索引与建表 SQL 一并写入 migration 文件，确保数据库初始化时同步创建。

---

## 四、功能模块

### 4.1 项目 & 接口管理

- 左侧树形结构，支持拖拽排序、文件夹无限嵌套
- 右键菜单：新建接口、新建文件夹、重命名、删除、复制
- 接口支持一键复制为 cURL 命令

#### 4.1.1 接口命名默认规则

创建接口时，根据输入的 URL 自动生成默认名称，规则如下：

**路径参数识别（自动剥离 + 提取）：**

满足以下任一条件的路径段，识别为路径参数，从名称中剥离，并**自动填入 Path Params 表格**：
- 纯数字段，如 `1676657` → `{id}: 1676657`
- 以 `:` 开头的段，如 `:id` → `{id}: `（值留空）
- 符合 UUID 格式（`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`）→ `{uuid}: <值>`

**命名格式：** `[METHOD] /剥离参数后的路径`

```
输入 URL：http://localhost:8088/apm/intl/download/1676657?androidVersion=14
Method：POST
→ 默认名称：POST /apm/intl/download
→ Path Params 表格自动填入：{id} = 1676657
→ Query Params 表格自动填入：androidVersion = 14

输入 URL：https://api.example.com/users/123/orders/456
Method：GET
→ 默认名称：GET /users/orders
→ Path Params 表格自动填入：{id} = 123, {id2} = 456

输入 URL：https://api.example.com/users/:id/profile
Method：PUT
→ 默认名称：PUT /users/profile
→ Path Params 表格自动填入：{id} =（值留空）
```

> 用户下次调试同一接口，只需修改 Path Params 表格中的值，URL 自动更新，无需手动编辑 URL 字符串。  
> 默认名称自动填入输入框，用户可在保存前手动修改。

#### 4.1.2 接口名唯一性校验

**规则：同一 Collection 内，接口名称不可重复；不同 Collection 间允许同名。**

> 理由：实际项目中，`POST /api/users` 在「用户模块」和「管理模块」各出现一次是正常场景，项目级唯一性约束会导致频繁冲突弹窗，体验极差。

**创建/重命名时的校验流程：**

```
用户输入名称
    ↓
查询当前 Collection 内是否存在同名接口
    ↓
不存在 → 直接保存
    ↓
存在 → 弹出提示弹窗：

┌────────────────────────────────────────────┐
│  当前文件夹中已存在同名接口：                 │
│  「POST /apm/intl/download」                │
│                                            │
│  [重命名后创建]  [仍然创建]  [取消]           │
└────────────────────────────────────────────┘
```

| 选项 | 行为 |
|------|------|
| **重命名后创建** | 在名称后自动追加序号，如 `POST /apm/intl/download (2)`，用户可进一步编辑 |
| **仍然创建** | 强制创建，系统自动追加序号保证唯一性，用户事后可重命名 |
| **取消** | 关闭弹窗，回到编辑状态 |

> 自动追加序号规则：检测当前 Collection 内已有 `{name}`、`{name} (2)`、`{name} (3)` ...，取最小可用序号。

### 4.2 接口调试

- Method 下拉（GET / POST / PUT / DELETE / PATCH / HEAD / OPTIONS）
- 请求配置 Tab：Params（含 Path Params）/ Headers / Body / Auth
- Body 类型：raw JSON、raw Text、form-data、x-www-form-urlencoded
- Auth 类型：Bearer Token、Basic Auth、API Key
- 响应区：
  - JSON 自动美化 + 语法高亮 + 折叠
  - HTML / XML 语法高亮
  - 一键复制美化后内容
  - 响应头、耗时、状态码、响应体大小显示
- 历史记录：每次调试自动保存到 `request_history`，响应区「History」Tab 可查看和 Diff

#### 4.2.0 Path Params 编辑器

URL 中识别到的路径参数自动出现在 Params Tab 的 **Path Params** 分区：

```
Params Tab 布局：
  ┌─ Path Params ──────────────────────┐
  │  {id}      [  1676657  ]  ✓        │
  │  {version} [           ]  ✓        │
  └────────────────────────────────────┘
  ┌─ Query Params ─────────────────────┐
  │  androidVersion  [  14  ]  ✓       │
  │  [+ 添加]                          │
  └────────────────────────────────────┘
```

- Path Params 的 key 不可删除（与 URL 模板绑定），只能修改 value
- 修改 value 后，URL 输入框实时同步（`/apm/intl/download/{id}` → `/apm/intl/download/9999`）
- URL 输入框中的 `{id}` 占位符高亮显示

#### 4.2.0b 响应历史 Tab + JSON Diff

响应区新增 **[History]** Tab，展示当前接口最近 20 条调试历史（来自 `request_history` 表）：

```
响应区 Tab：[Body(美化)] [Headers] [History]

History Tab 布局：
  ┌───────────────────────────────────────────────────┐
  │ ☑ 2026-03-23 14:32  200  120ms                    │
  │ ☑ 2026-03-23 14:28  200  145ms                    │
  │   2026-03-23 14:20  500  89ms                     │
  │   2026-03-23 10:05  200  201ms                    │
  │                                    [Diff 选中两条] │
  └───────────────────────────────────────────────────┘
```

- 勾选任意两条记录 → 触发 JSON Diff，左右对比展示差异（新增/删除/修改高亮）
- 点击单条记录 → 展开查看完整响应体
- 「↩ 回填」按钮：将该历史的请求参数（URL/Headers/Body）一键回填到编辑区

> `request_history` 表的 `request_snapshot` 和 `response_body` 字段已设计好，直接支持此功能。

**设计原则：低干扰自动决策**——用例在后台静默关联，不打断调试心流。

```
用户点击「Send / 发送」
        ↓
当前接口是否已有测试用例？
        ↓                         ↓
       否                        是
        ↓                         ↓
  后台自动创建「用例 1」       沿用上次激活的用例（静默）
  标记收藏 ⭐，静默关联          直接执行请求
  执行本次请求
        ↓
  请求完成后，底部出现用例选择栏（非阻塞，不影响响应展示）
  ┌──────────────────────────────────┐
  │ 📋 用例 1 ⭐  │ 用例 2  │ [+ 新建] │
  └──────────────────────────────────┘
```

**参数变化检测（Send 时）：**

当前编辑区内容与激活用例的保存值有差异时，Send 完成后（不是之前）底部静默出现提示条：

```
「当前参数与用例 1 不同」  [保存到用例 1]  [另存为新用例]  [忽略]
```

> 关键：提示在响应返回后出现，不阻塞请求发送本身。

**用例选择栏交互说明：**

| 操作 | 行为 |
|------|------|
| 点击已有用例 | 切换激活用例，编辑区加载该用例的参数（method/url/headers/params/body） |
| 点击「+ 新建」 | 以当前编辑区内容为初始值，创建新用例（弹出命名输入框） |
| 长按/右键用例 | 弹出菜单：重命名 / 收藏⭐ / 删除 |

**用例命名规则：**
- 自动命名：`用例 1`、`用例 2`……（按创建顺序递增）
- 用户可随时重命名，名称在同一接口内唯一

#### 4.2.2 测试用例收藏机制与定时清理

**收藏规则：**

| 规则 | 说明 |
|------|------|
| 每个接口创建的**第一个用例自动标记收藏** ⭐ | 避免新建接口就没有保留用例 |
| 每个接口**至少保留一个收藏用例** | 删除时若为最后一个收藏用例，阻止删除并提示 |
| 用户可手动收藏/取消收藏任意用例 | 取消收藏时，若当前接口只剩一个收藏用例，阻止取消并提示 |
| **AI 生成的用例（`source = ai_generated`）默认不收藏** | AI 批量生成的用例不占用保留配额 |

**定时清理规则：**

- 清理对象：`starred = 0`（未收藏）且 `created_at < 当前时间 - 30天` 的用例
- 清理时机：应用启动时后台静默执行，不打断用户操作
- 清理前检查：确保每个接口清理后仍至少保留一个收藏用例（即使收藏用例超过 30 天也不清理）

**对应 Rust 清理逻辑（Tauri Command，启动时调用）：**

```rust
// 两步清理，逻辑清晰且无 SQL 自引用问题：
// Step 1: 找出「所有用例均未收藏」的接口 ID（需要保护，不能全删）
// Step 2: 删除 30 天前未收藏用例，但排除上述接口中最新的一条
```

```sql
-- Step 1: 找出有收藏用例的接口（这些接口可以安全删除未收藏的旧用例）
-- Step 2: 对这些接口，删除 30 天前的未收藏用例

DELETE FROM test_cases
WHERE starred = 0
  AND created_at < datetime('now', '-30 days')
  AND request_id IN (
    -- 只处理「至少有一个收藏用例」的接口，确保删后仍有收藏用例保留
    SELECT DISTINCT request_id FROM test_cases WHERE starred = 1
  );

-- 对「完全没有收藏用例」的接口，额外保护：保留最新一条（兜底）
-- 此类接口的旧用例不参与上面的删除，不会被误删
```

> 两步设计要点：先用子查询过滤出「有收藏用例」的接口集合，再在该集合内删除未收藏旧用例；「没有任何收藏用例」的接口完全不受影响，其最新用例天然保留。

### 4.3 环境 & 变量管理

- 右上角环境切换下拉框
- 环境管理弹窗：增删改环境、管理变量
- URL / Headers / Body 中 `{{variable}}` 自动高亮提示
- 全局变量 + 环境变量两层，环境变量优先级更高

### 4.4 参数编辑器

三种编辑模式，随时切换互转：

| 模式 | 说明 |
|------|------|
| **表格模式** | Key-Value 行编辑，支持勾选启用/禁用单行 |
| **KV 文本模式** | 粘贴 `key: value` 或 `key=value` 文本，自动解析为表格行 |
| **JSON 模式** | 粘贴 JSON 对象，自动展开为表格行；表格也可导出为 JSON |

- 支持复制当前参数为 KV 文本格式 或 JSON 格式
- 公共 Headers 模板（如统一的 `Authorization` 前缀）

### 4.5 全局 & 项目级 Cookie

- **全局 Cookie**：顶栏入口，跨项目共享，按域名管理
- **项目级 Cookie**：项目设置内，仅对当前项目生效
- 发请求时自动携带匹配域名的 Cookie，项目级优先于全局级
- 支持手动增删改 Cookie，支持启用/禁用

### 4.6 压测

- 入口：接口详情页右上角「压测」按钮
- 配置参数：
  - 并发数（1 ~ 500）
  - 模式：总请求数 或 持续时间（秒）
- 执行：Rust tokio 异步并发，通过 Tauri Event 实时推送进度到前端
- 结果展示：
  - 总请求数、成功数、失败数、成功率
  - 平均 / P50 / P95 / P99 响应时间
  - TPS（每秒事务数）
  - 实时折线图（响应时间 + TPS）

### 4.7 导入 & 导出

| 方向 | 格式 |
|------|------|
| 导入 | Postman Collection v2.1 JSON |
| 导入 | **OpenAPI 3.x（swagger.json / openapi.yaml）** |
| 导入 | ApiCat 自定义格式（JSON / YAML） |
| 导出 | ApiCat 自定义格式（完整项目数据，含环境变量） |
| 导出 | Postman Collection v2.1 兼容格式 |

- 入口：项目右键菜单 或 顶栏设置

---

## 五、UI 布局

```
┌─────────────────────────────────────────────────────────────┐
│  顶栏：[项目切换▼]  [环境切换▼]  [🍪 全局Cookie]  [⚙ 设置]  │
├──────────────┬──────────────────────────────────────────────┤
│              │  ┌─ Method ─┬──────── URL ──────────┬[Send]─┐│
│  左侧边栏    │  │  POST  ▼ │ {{base_url}}/api/login │  发送 ││
│              │  └──────────┴───────────────────────┴───────┘│
│  🗂 项目树   │  Tab: [Params] [Headers] [Body] [Auth]        │
│  ├ 📁 用户   │  ┌──────────────────────────────────────────┐ │
│  │  ├ 登录  │  │  参数表格 / KV文本 / JSON  三种模式切换   │ │
│  │  └ 注册  │  └──────────────────────────────────────────┘ │
│  └ 📁 订单  │                                               │
│     └ 创建  │  ────── 可拖拽分隔线 ──────────────────────── │
│             │                                               │
│  [+ 新建]   │  Status: 200 OK | 120ms | 1.2KB  [📋复制]    │
│             │  Tab: [Body(美化)] [Headers] [History]        │
│             │  ┌──────────────────────────────────────────┐ │
│             │  │  {                                        │ │
│             │  │    "code": 0,                             │ │
│             │  │    "data": { ... }   ▶ 可折叠             │ │
│             │  │  }                                        │ │
│             │  └──────────────────────────────────────────┘ │
└──────────────┴──────────────────────────────────────────────┘
```

---

## 六、项目目录结构

```
apicat/
├── src-tauri/                      # Rust 后端
│   ├── src/
│   │   ├── main.rs
│   │   ├── db/
│   │   │   ├── mod.rs              # DB 连接池初始化（sqlx）
│   │   │   ├── migrations/         # SQLite 迁移 SQL 文件
│   │   │   └── models/             # 各表的 CRUD 操作
│   │   │       ├── project.rs
│   │   │       ├── environment.rs
│   │   │       ├── collection.rs
│   │   │       ├── request.rs
│   │   │       ├── history.rs
│   │   │       └── cookie.rs
│   │   ├── commands/               # Tauri Commands（前端调用入口）
│   │   │   ├── project.rs
│   │   │   ├── environment.rs
│   │   │   ├── collection.rs
│   │   │   ├── request.rs          # 发送请求 + 变量替换
│   │   │   ├── stress.rs           # 压测逻辑
│   │   │   ├── cookie.rs
│   │   │   └── io.rs               # 导入导出
│   │   └── http/
│   │       ├── client.rs           # reqwest 客户端封装
│   │       └── variable.rs         # {{var}} 替换引擎
│   └── Cargo.toml
│
├── src/                            # Vue 3 前端
│   ├── main.ts
│   ├── App.vue
│   ├── router/
│   ├── stores/                     # Pinia
│   │   ├── project.ts
│   │   ├── environment.ts
│   │   ├── cookie.ts
│   │   └── ui.ts                   # 布局状态（面板宽度等）
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.vue         # 左侧项目树
│   │   │   ├── TopBar.vue
│   │   │   └── MainPanel.vue
│   │   ├── request/
│   │   │   ├── RequestEditor.vue
│   │   │   ├── ParamsEditor.vue    # 表格 / KV文本 / JSON 三模式
│   │   │   ├── HeadersEditor.vue
│   │   │   ├── BodyEditor.vue
│   │   │   └── AuthEditor.vue
│   │   ├── response/
│   │   │   ├── ResponsePanel.vue
│   │   │   └── JsonViewer.vue      # 美化 + 折叠 + 复制
│   │   ├── environment/
│   │   │   ├── EnvSelector.vue
│   │   │   └── EnvManager.vue
│   │   ├── cookie/
│   │   │   ├── GlobalCookieManager.vue
│   │   │   └── ProjectCookieManager.vue
│   │   ├── stress/
│   │   │   ├── StressConfig.vue
│   │   │   └── StressResult.vue    # 实时图表
│   │   └── io/
│   │       ├── ImportDialog.vue
│   │       └── ExportDialog.vue
│   ├── utils/
│   │   ├── paramParser.ts          # KV ↔ JSON 互转工具
│   │   └── highlight.ts            # 代码高亮工具封装
│   └── types/                      # 全局 TypeScript 类型定义
│       ├── project.ts
│       ├── request.ts
│       └── response.ts
│
├── docs/
│   └── plans/
│       └── 2026-03-23-apicat-design.md   # 本文档
├── package.json
├── vite.config.ts
└── tauri.conf.json
```

---

## 七、开发计划（6 个迭代）

| 迭代 | 内容 | 关键产出 |
|------|------|---------| 
| **M1** | 项目脚手架 + DB 初始化 + 基础布局 | Tauri + Vue + SQLite 跑通，三栏布局可见，迁移脚本执行成功 |
| **M2** | 项目 / 集合 / 接口 CRUD + 左侧树 + Path Params | 能创建项目、建文件夹、新建接口；URL 路径参数自动提取为 Path Params 表格行 |
| **M3** | 接口调试核心：发请求 + 响应美化 + History Diff | 能发 GET/POST，JSON 高亮，状态码/耗时显示；响应历史 Tab 可查看和 JSON Diff |
| **M4** | 环境变量 + Cookie 管理 + 变量替换 | `{{base_url}}` 自动替换，Cookie 按域名自动携带 |
| **M5** | 参数编辑器增强（KV/JSON互转）+ 测试用例低干扰流程 | 粘贴 JSON 自动展开为表格；Send 静默关联用例，参数变更后非阻塞提示保存 |
| **M6** | 压测 + 导入导出（Postman + OpenAPI 3.x）| 完整 v1.0 功能闭环；支持导入 Postman Collection 和 OpenAPI 3.x 文档 |

### v1.1 规划（v1.0 之后）

| 功能 | 说明 |
|------|------|
| **本地 Mock Server** | 基于接口配置一键启动本地 HTTP Mock 服务 |
| **批量运行测试用例 + 报告** | Collection 级一键运行所有 test_cases，输出健康度报告 |
| **test_cases 完整 UI** | 左侧树展示、单条运行、结果持久化（目前为 AI Skill 写入 + 基础展示） |

---

## 八、关键技术决策记录

| 决策 | 选择 | 原因 |
|------|------|------|
| HTTP 发送层 | Rust reqwest | 无 CORS 限制，压测性能，证书控制 |
| 压测实现 | Rust tokio 并发 | 不依赖外部工具，性能足够，可实时推送事件 |
| 数据库 | SQLite + sqlx | 本地存储，零部署，类型安全查询 |
| Cookie 分层 | 全局 + 项目级 | 兼顾跨项目共享和项目隔离的不同需求 |
| 参数存储格式 | JSON 文本列 | 灵活扩展，避免多余关联表 |
| 导入导出 | 自定义格式 + Postman 兼容 | 迁移成本低，保留 Postman 用户转换路径 |
| 测试用例存储 | 独立 test_cases 表 | 与调试历史分离，支持断言、AI 生成来源标记、批量运行 |

---

## 九、AI 测试用例生成 Skill（`apicat-test-gen`）

> **定位**：一个 Claude Code / OpenCode Skill，读取接口定义或源代码，自动生成 API 测试用例，直接写入 ApiCat 本地 SQLite 数据库。  
> **开发时机**：M2（数据库 CRUD 跑通）完成后即可验证写入；M6 完成后作为配套工具正式发布。

---

### 9.1 Skill 目录结构

```
.claude/skills/apicat-test-gen/
├── SKILL.md                        ← 触发词 + 工作流指令（主文件）
├── scripts/
│   ├── find_db.py                  ← 自动定位 ApiCat 数据库路径
│   ├── write_test_cases.py         ← 测试用例 JSON → SQLite 写入脚本
│   └── run_test_cases.py           ← （可选）批量运行已写入的测试用例
└── references/
    └── apicat_schema.md            ← ApiCat 表结构速查（从本文档同步）
```

---

### 9.2 SKILL.md 框架

```markdown
---
name: apicat-test-gen
description: |
  分析接口定义（OpenAPI / 路由文件 / Controller 代码），自动生成 API 测试用例，
  写入 ApiCat 本地 SQLite 数据库的 test_cases 表。
  触发词：「生成测试用例」「为接口生成测试」「写入 ApiCat」「apicat test」
allowed-tools: Bash, Read, Glob, Grep, Write
---

## 工作流

### Step 1：定位 ApiCat 数据库

执行 `python3 scripts/find_db.py` 获取 DB 路径。

数据库位置规则：
- macOS：`~/Library/Application Support/ApiCat/apicat.db`
- Windows：`%APPDATA%\ApiCat\apicat.db`
- Linux：`~/.config/ApiCat/apicat.db`
- 开发模式：`./apicat/apicat.db`（优先检测）

### Step 2：确认目标项目和 Collection

查询数据库获取项目列表，询问用户要写入哪个项目。
若目标项目下不存在名为「🤖 AI 测试用例」的 Collection，自动创建。

### Step 3：分析接口定义

读取用户指定的文件，识别格式：
- **OpenAPI 3.x / Swagger JSON/YAML** → 解析 paths
- **Postman Collection JSON** → 解析 item
- **源代码路由文件** → 识别 @GetMapping / router.get / app.get 等模式
- **ApiCat 数据库中的已有接口** → 直接从 api_requests 表读取

### Step 4：生成测试用例

为每个接口生成以下用例：

| 用例类型 | 描述 | 典型断言 |
|---------|------|---------|
| Happy Path | 正常参数，期望成功响应 | status=200, body.code=0 |
| 缺少必填参数 | 省略 required 字段 | status=400 |
| 未授权请求 | 不携带 Token | status=401 |
| 参数边界值 | 字符串最大长度、数字边界 | status=200 或 400 |
| 空列表场景 | 查询无数据时的响应 | status=200, body.data=[] |

### Step 5：写入数据库

```bash
python3 scripts/write_test_cases.py <db_path> /tmp/apicat_test_cases_{timestamp}.json
```

### Step 6：输出摘要

告知用户共生成 N 条测试用例、写入项目、覆盖接口数。
```

---

### 9.3 `write_test_cases.py` 核心逻辑

```python
#!/usr/bin/env python3
import sqlite3, json, sys
from datetime import datetime

def write(db_path: str, json_file: str):
    with open(json_file) as f:
        cases = json.load(f)

    with sqlite3.connect(db_path, timeout=10) as conn:
        conn.execute("PRAGMA journal_mode = WAL")
        conn.execute("PRAGMA busy_timeout = 5000")

        sql = """
        INSERT INTO test_cases
            (collection_id, request_id, name, description, source,
             method, url, headers, params, body_type, body,
             assertions, enabled, sort_order, created_at, updated_at)
        VALUES
            (?, ?, ?, ?, ?,
             ?, ?, ?, ?, ?, ?,
             ?, 1, ?, datetime('now'), datetime('now'))
        """
        rows = [
            (c["collection_id"], c.get("request_id"),
             c["name"], c.get("description", ""), c.get("source", "ai_generated"),
             c.get("method"), c.get("url"),
             json.dumps(c.get("headers", []), ensure_ascii=False),
             json.dumps(c.get("params",  []), ensure_ascii=False),
             c.get("body_type"), c.get("body", ""),
             json.dumps(c.get("assertions", []), ensure_ascii=False),
             i)
            for i, c in enumerate(cases)
        ]
        conn.executemany(sql, rows)
        conn.commit()
        print(f"✅ 成功写入 {len(cases)} 条测试用例")

if __name__ == "__main__":
    write(sys.argv[1], sys.argv[2])
```

---

### 9.4 ApiCat 侧需同步实现（M6 之后）

| 功能 | 说明 |
|------|------|
| 左侧树显示测试用例 | `test_cases` 记录以「⚡」图标区分普通接口 |
| 单条测试用例运行 | 执行断言，显示 passed / failed / error |
| 批量运行 | 选中 Collection 下所有测试用例一键运行，输出报告 |
| 测试用例编辑 | 支持手动修改断言、请求参数 |
| 运行结果持久化 | 更新 `last_status`、`last_duration_ms`、`last_response` 字段 |

这部分功能可作为 **v1.1** 迭代目标，不影响 v1.0 主体开发。

---

## 十、工程风险与边界约束

> 本章记录 Eng Review 发现的关键风险点及对应约束方案，在实现时必须遵守。

### 10.1 压测 IPC 事件风暴（CRITICAL）

**风险**：500 并发压测时，每个请求完成都触发一次 `tauri::emit`，每秒产生数千事件，堵塞 JS 事件循环，导致 UI 卡死。

**约束方案**：压测进度事件必须聚合后批量推送，禁止逐请求发送事件。

```rust
// ✅ 正确：每 200ms 聚合一次，推送一批统计数据
// ❌ 禁止：每个请求完成立即 emit 一次
```

具体实现：维护一个共享的 `Arc<Mutex<StressStats>>`，由每个 tokio task 写入；另起一个定时任务每 200ms 读取一次快照并 emit 到前端。

---

### 10.2 文件描述符耗尽（HIGH）

**风险**：macOS / Linux 默认 fd 上限（macOS: 256, 某些 Linux: 1024），500 并发压测在建立 TCP 连接时触发 `EMFILE: too many open files`，导致 Tauri 进程崩溃。

**约束方案**：

1. 压测启动前检测系统 fd 上限，若不足则警告用户并拒绝启动（或自动调低并发数）
2. reqwest 连接池大小不超过 `max_concurrent`，避免连接无限扩张

---

### 10.3 变量 JSON 注入（CRITICAL）

**风险**：变量值中含双引号（如 `{{user}}` = `foo"bar`）时，直接字符串替换会破坏 raw JSON body。

**约束方案**：`body_type = raw_json` 时，变量替换必须对值进行 JSON 字符串转义：

```rust
let escaped_value = serde_json::to_string(&value)
    .unwrap()
    .trim_matches('"')
    .to_string();
```

---

### 10.4 树结构 N+1 查询（MEDIUM）

**风险**：左侧接口树按层递归查询 `collections` 表，深度 5 层时产生 5+ 次查询，项目规模大时加载卡顿。

**约束方案**：一次查出全部节点，在内存中组装树：

```rust
// ✅ 正确：单次查询 + 内存组装
let all_nodes = sqlx::query!("SELECT * FROM collections WHERE project_id = ?", project_id)
    .fetch_all(&pool).await?;
let tree = build_tree(all_nodes);

// ❌ 禁止：递归按 parent_id 逐层查询
```

---

### 10.5 响应体积上限（HIGH）

**风险**：请求返回 20MB 大型响应体时，全部写入 `request_history.response_body`，导致 SQLite 文件急剧膨胀，且前端 JSON 渲染 OOM。

**约束方案**：

- 响应体超过 **2MB** 时截断，只保存前 2MB，同时设置 `is_truncated = 1`
- 前端展示时，若 `is_truncated = 1`，在响应区顶部显示提示条：`⚠️ 响应体过大，仅显示前 2MB`

```rust
const MAX_BODY_SIZE: usize = 2 * 1024 * 1024; // 2MB
let (body_to_store, is_truncated) = if body.len() > MAX_BODY_SIZE {
    (body[..MAX_BODY_SIZE].to_vec(), 1)
} else {
    (body, 0)
};
```