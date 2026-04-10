# COMMANDS — Tauri IPC 命令层

## OVERVIEW
10 个 `.rs` 文件，每个对应一个业务域。所有函数通过 `lib.rs` 的 `invoke_handler!` 宏注册。

## 文件清单与职责
| 文件 | Commands | 说明 |
|------|----------|------|
| `project.rs` | `list/create/update/delete_project` | 项目 CRUD |
| `collection.rs` | `list/create/rename/delete_collection`, `update_collection_sort`, `move_collection` | 文件夹 CRUD + 拖拽移动 |
| `request.rs` | `list/create/update/delete/duplicate_request`, `update_request_sort`, `move_request` | 接口 CRUD + 拖拽移动 |
| `send_request.rs` | `send_request`, `list_history` | 发送 HTTP 请求，写历史记录 |
| `environment.rs` | `list/create/update/delete_environment`, `activate/deactivate_environment`, `list/create/update/delete_env_variable` | 环境变量管理 |
| `cookie.rs` | `list/create/update/delete_cookie`, `get_cookies_for_domain` | Cookie 管理 |
| `test_case.rs` | `list/create/update/delete_test_case` | 测试用例 CRUD |
| `stress.rs` | `start_stress` | 压测（Tauri event 推送进度） |
| `io.rs` | `export_apicat`, `export_postman`, `import_apicat`, `import_postman`, `import_openapi` | 导入/导出（699 行，最复杂） |
| `mod.rs` | — | `pub mod` 声明各子模块 |

## 关键 Command 说明

### move_collection / move_request（拖拽后端）
```rust
// move_collection：含递归 CTE 循环引用防护
move_collection(id, new_parent_id: Option<i64>, sort_order)
// new_parent_id=None → 移到根层

move_request(id, new_collection_id, sort_order)
```
前端在 `onDrop` 之后调用，**配合** `update_collection_sort` / `update_request_sort` 批量更新排序。

### io.rs（导入导出）
- `export_apicat`：递归树形导出，返回 JSON 字符串，前端写文件
- `import_postman`：递归处理 items 树，用 `async-recursion` crate
- `import_openapi`：解析 YAML/JSON，`serde_yaml` 处理 .yaml 格式
- 导出文件写入由前端 `tauri-plugin-fs` 完成（后端只返回字符串）

### update_*_sort（批量排序）
```rust
// 接收 Vec<(id, sort_order)> 逐条 UPDATE
// 前端乐观更新后调用，失败时回滚本地 store
update_collection_sort(items: Vec<(i64, i64)>)
update_request_sort(items: Vec<(i64, i64)>)
```

## 查询模式
```rust
// 标准模式：query_as + RETURNING（INSERT/UPDATE）
sqlx::query_as::<_, T>("... RETURNING *").bind(x).fetch_one(&db.0).await?

// 批量操作：循环 + 逐条执行（无事务，乐观更新由前端处理回滚）
for (id, sort) in items { sqlx::query("UPDATE ...").bind(sort).bind(id).execute(&db.0).await? }
```

## ANTI-PATTERNS
- **不要**在 command 中直接使用 `sqlx::query!` 宏——项目未配置 `DATABASE_URL` 编译时变量，用 `sqlx::query` / `sqlx::query_as` 替代
- **不要**跳过 `mod.rs` 的 `pub mod` 声明——Rust 模块系统要求显式声明
- **不要**在 `io.rs` 的导入函数里跳过 project_id 绑定——会导致数据插入到错误项目
