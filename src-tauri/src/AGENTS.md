# RUST BACKEND — src-tauri/src

## OVERVIEW
Tauri 2.x Rust 后端。`lib.rs` 为入口，注册插件 + SQLite pool + 所有 command；`types.rs` 定义数据模型；`error.rs` 统一错误类型。

## 模块结构
```
src/
├── lib.rs          # 插件注册、AppDb 注入、invoke_handler 注册、定时清理
├── main.rs         # 仅调用 lib::run()
├── types.rs        # 所有数据模型（derive sqlx::FromRow + Serialize/Deserialize）
├── error.rs        # AppError / CmdResult<T>
├── db/mod.rs       # SqlitePool 初始化（WAL + FK + migration）
├── commands/       # 业务 command（见 commands/AGENTS.md）
└── http/           # reqwest 客户端封装（client.rs / variable.rs）
```

## 核心约定

### 新增 command 流程（必须三步）
```rust
// 1. commands/xxx.rs 中定义
#[tauri::command]
pub async fn my_command(db: State<'_, AppDb>, ...) -> CmdResult<T> { ... }

// 2. commands/mod.rs 中 pub use
pub mod xxx;

// 3. lib.rs invoke_handler 中注册
tauri::generate_handler![..., my_command]
```
缺任何一步都会导致前端 invoke 报 "command not found"。

### 错误处理
```rust
// 统一返回类型
pub type CmdResult<T> = Result<T, AppError>;

// sqlx 错误自动转换（From trait）
// 自定义错误：AppError::Custom("message".to_string())
// 前端收到：字符串，用 String(e) 处理
```

### SQLite 查询风格
```rust
// 优先 query_as + RETURNING（单次往返）
sqlx::query_as::<_, Collection>(
    "INSERT INTO ... RETURNING id, project_id, ..."
)
.bind(value)
.fetch_one(&db.0)
.await?
```

### 插件权限（必须在 capabilities/default.json 声明）
当前已声明：`core:default`, `opener:default`, `dialog:default`, `fs:default`, `fs:allow-write-text-file`, `store:default`, `updater:default`, `process:default`

**Tauri 2.x 陷阱**：插件在 lib.rs 注册但不在 capabilities 里声明 → IPC 被静默拒绝，前端收到异常，且异常会吞掉后续逻辑。

## DB Schema 概览（migrations/0001_init.sql）
```
projects → environments → env_variables
         → collections (self-ref parent_id) → api_requests → request_history
         → cookies
collections → test_cases
```
全部表用 `CREATE TABLE IF NOT EXISTS`（幂等）。**不要修改现有列**，新增内容追加到文件末尾。

## HTTP 客户端
- 全局单例 `HttpClient(reqwest::Client)`，通过 `app.manage()` 注入
- `danger_accept_invalid_certs(true)`（设计决策：调试工具，不验证证书）
- 超时 30s，`cookies` feature 启用

## ANTI-PATTERNS
- **不要**每个 command 新建 reqwest Client——复用全局 `HttpClient` state
- **不要**用 `include_str!` 之外的方式读 migration——已有机制保证编译时内嵌
- **不要**修改 `0001_init.sql` 现有建表语句——会破坏已有用户的数据库
- **不要**在 `#[tauri::command]` 函数里 `panic!`——会崩溃整个应用；用 `CmdResult` 返回错误
