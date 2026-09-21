# 合并 test_case_history 到 request_history 方案

## 背景

当前有两个历史记录表：
- `request_history`：调试发送的历史（History Tab）
- `test_case_history`：用例执行的历史（用例页右侧栏）

用户希望合并这两个表，简化架构。

## 当前架构分析

### request_history 表
```sql
CREATE TABLE request_history (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  request_id       INTEGER NOT NULL REFERENCES api_requests(id) ON DELETE CASCADE,
  test_case_id     INTEGER REFERENCES test_cases(id) ON DELETE SET NULL,  -- 已有！
  status_code      INTEGER,
  response_time_ms INTEGER,
  request_snapshot TEXT,
  response_body    TEXT,  -- 完整响应 或 @file:{id} 标记
  is_truncated     INTEGER DEFAULT 0,
  response_headers TEXT,
  created_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

**关键发现**：`test_case_id` 字段已存在！
- `test_case_id IS NULL` → 调试历史
- `test_case_id IS NOT NULL` → 用例执行历史

### test_case_history 表
```sql
CREATE TABLE test_case_history (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  test_case_id     INTEGER NOT NULL REFERENCES test_cases(id) ON DELETE CASCADE,
  status_code      INTEGER,
  duration_ms      INTEGER,
  response_preview TEXT,  -- 1KB 摘要
  error_message    TEXT,
  created_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 字段映射关系

| test_case_history | request_history | 说明 |
|-------------------|-----------------|------|
| test_case_id | test_case_id | 相同 |
| status_code | status_code | 相同 |
| duration_ms | response_time_ms | 改名 |
| response_preview | response_body | 改名 + 存完整响应 |
| error_message | （需新增） | 需要添加字段 |
| created_at | created_at | 相同 |

## 合并方案

### 1. 表结构变更

```sql
-- request_history 增加 error_message 字段
ALTER TABLE request_history ADD COLUMN error_message TEXT;
```

### 2. 写入逻辑变更

#### run_test_case（用例执行）
**当前**：写入 `test_case_history`
**改为**：写入 `request_history`

```rust
// 需要先获取 request_id
let request_id: i64 = sqlx::query_scalar(
    "SELECT request_id FROM test_cases WHERE id = ?"
)
.bind(test_case_id)
.fetch_one(pool)
.await?;

// 写入 request_history
sqlx::query(
    "INSERT INTO request_history \
     (request_id, test_case_id, status_code, response_time_ms, \
      response_body, is_truncated, error_message) \
     VALUES (?, ?, ?, ?, ?, ?, ?)"
)
.bind(request_id)
.bind(test_case_id)
.bind(status_code)
.bind(elapsed_ms)
.bind(&response_body)  // 完整响应，不再是 preview
.bind(is_truncated)
.bind(&error_message)
.execute(pool)
.await?;
```

#### add_test_case_history（前端记录）
**当前**：前端在 `handleSend` 后调用
**改为**：废弃此 IPC，前端不再单独记录

### 3. 读取逻辑变更

#### list_test_case_history（用例页右侧栏）
**当前**：查询 `test_case_history`
**改为**：查询 `request_history`

```rust
let sql = format!(
    "SELECT id, test_case_id, status_code, response_time_ms as duration_ms, \
            response_body as response_preview, error_message, created_at \
     FROM request_history \
     WHERE test_case_id = ? \
     ORDER BY created_at DESC, id DESC \
     LIMIT 10"
);
```

#### list_history（History Tab）
**当前**：查询所有 `request_history`
**改为**：只查询调试历史（`test_case_id IS NULL`）

```rust
let sql = format!(
    "SELECT ... FROM request_history \
     WHERE request_id = ? AND test_case_id IS NULL \
     ORDER BY created_at DESC LIMIT 20"
);
```

### 4. 保留策略

#### 触发器修改
```sql
-- 删除旧触发器
DROP TRIGGER IF EXISTS trg_tch_keep_10;

-- 创建新触发器：只对用例执行历史生效
CREATE TRIGGER trg_keep_10_per_case
AFTER INSERT ON request_history
WHEN NEW.test_case_id IS NOT NULL
BEGIN
  DELETE FROM request_history
  WHERE test_case_id = NEW.test_case_id
    AND id NOT IN (
      SELECT id FROM request_history
      WHERE test_case_id = NEW.test_case_id
      ORDER BY created_at DESC, id DESC
      LIMIT 10
    );
END;
```

### 5. 数据迁移

```sql
-- 迁移 test_case_history 到 request_history
INSERT INTO request_history 
  (request_id, test_case_id, status_code, response_time_ms, 
   response_body, error_message, created_at)
SELECT 
  tc.request_id,
  tch.test_case_id,
  tch.status_code,
  tch.duration_ms,
  tch.response_preview,  -- 作为 response_body
  tch.error_message,
  tch.created_at
FROM test_case_history tch
JOIN test_cases tc ON tch.test_case_id = tc.id
WHERE tc.request_id IS NOT NULL;

-- 删除旧表
DROP TABLE IF EXISTS test_case_history;
```

### 6. 清理逻辑简化

`cleanup_history` 不再需要单独清理 `test_case_history`：
- 删除 `cleanup_test_case_history` 参数
- 删除相关清理代码

### 7. 前端变更

#### testCase.ts
- `loadHistory()`：改用新的 `list_test_case_history`（已兼容）
- `recordHistory()`：废弃，不再调用
- `historyMap`：继续使用，但数据来源变为 `request_history`

#### MainPanel.vue
- `handleSend()`：删除 `testCaseStore.recordHistory()` 调用

#### HistoryTab.vue
- 无需变更（继续用 `list_history`）

## 实施步骤

1. **数据库迁移**
   - 添加 `error_message` 字段
   - 迁移数据
   - 删除旧表
   - 创建新触发器

2. **后端变更**
   - 修改 `run_test_case`：写入 `request_history`
   - 修改 `list_test_case_history`：查询 `request_history`
   - 修改 `list_history`：只查调试历史
   - 废弃 `add_test_case_history`
   - 简化 `cleanup_history`

3. **前端变更**
   - 删除 `recordHistory()` 调用
   - 更新类型定义（如有需要）

4. **测试验证**
   - 发送请求 → History Tab 显示
   - 执行用例 → 用例页显示
   - 清理历史 → 两处都清空

## 风险与注意事项

1. **数据兼容性**：迁移脚本需要处理 `request_id` 为 NULL 的情况
2. **性能影响**：`list_history` 增加 `test_case_id IS NULL` 条件
3. **前端兼容**：`TestCaseHistory` 类型字段名变化（duration_ms → response_time_ms）
4. **Bridge API**：`list_test_case_history` 返回结构变化

## 预期收益

- ✅ 减少表数量，简化架构
- ✅ 统一清理逻辑
- ✅ 避免数据不一致
- ✅ 用例历史可存储完整响应（不再限制 1KB）
