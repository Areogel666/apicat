---
name: apicat-doc-gen
description: 根据 ApiCat 接口定义 + 用例生成格式化接口文档（对齐 ias-api-doc 规范）。触发词：生成接口文档、接口文档、出文档、apicat doc、apicat-doc-gen
allowed-tools: Bash, Read, Write, Glob, AskUserQuestion
---

# ApiCat 接口文档生成

从 ApiCat 读接口定义和用例，生成对齐 `ias-api-doc/API_SPECIFICATION.md` 规范的 Markdown 文档。

## Step 1：定位 Bridge + 选项目

按 `apicat-lib` 定位。`AskUserQuestion` 选项目。

## Step 2：确定范围

- **单接口**（默认）：用户点名或问用户选一个接口
- **整项目**（用户说「全部」或 `--all`）：遍历项目下所有接口

## Step 3：读数据

```bash
# 接口详情
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/get_request?id=$RID"
# 用例（用于生成多场景 Response 示例）
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_test_cases?request_id=$RID"
# 字段绑定（已绑定字典的字段引用字典文件）
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_field_rules?project_id=$PID"
# 字典详情
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_dictionaries?project_id=$PID"
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_dictionary_items?dictionary_id=$DID"
```

## Step 4：确定输出目录

1. 读项目 `docs_output_dir`（从 `list_projects` 返回）
2. 有值 → 用它；无值 → 默认 `~/.apicat/apidoc/{project_name}/`
3. 用户说「写 ias-api-doc」→ `D:\ProgramFiles\mi\ias-api-doc\{service}-api\{module}\`

目录结构（对齐 ias-api-doc 约定）：
```
{output_dir}/
├── {MODULE}.md              # 接口文档（UPPER_SNAKE_CASE）
└── common/
    └── LIST_APP_COMMON.md   # 公共字段字典（如有）
```

## Step 5：生成文档格式

### 文件骨架

```markdown
# 【{service}-api】{接口中文名} {path}

> 引用块：登录要求 / 响应信封说明 / 集群限制等

---

### {接口中文名} {path}

> 匿名可访问 / 需登录 等

#### {操作名}

- 请求({service}-api)
    - 线上域名：https://...
    - 预发域名：https://...
    - 测试域名：https://...
- `GET` 使用 URL Query 参数，不使用请求体。

- URL 参数:

| 参数 | 类型 | 必需 | 说明 |
|---|---|---|---|
| gpId | string | 是 | 设备 GAID |

- Response（成功）

```json lines
{
  // 错误码：0=成功
  "code": 0,
  "list": []
}
```

- 顶层字段说明

| 字段名 | 类型 | 下发条件 | 默认值 | 取值范围 | 非范围处理 | 说明 |
|--------|------|---------|--------|---------|-----------|------|
| code | long | 总是 | — | `0`=成功 | — | 错误码 |

- 错误码

| code | 含义 | 触发场景 |
|---|---|---|
| 20000 | 参数校验失败 | gpId 缺失 |

- 备注
    - 要点逐条列出
```

### 关键格式约定

- **响应字段用「下发条件」**（`总是`/`条件下发`/`命中时才有`），不用「必填」
- **枚举写法**：反引号包值 + `=` 连含义：`` `0`=非广告、`1`=广告 ``
- **JSON 示例**统一 ` ```json lines ` 代码块（允许 `//` 行内注释）
- **类型**小写：`string`/`int`/`long`/`boolean`/`array[string]`
- 接口之间用 `---` 分隔线

### 用例 → 多场景 Response

用 ApiCat 里的用例生成多个 Response 示例块：
- Happy Path 用例 → `- Response（成功）`
- `missing_required` → `- Response（参数错误）`
- `empty_list` → `- Response（无数据）`
- `unauthorized` → `- Response（未授权）`

每个场景给完整 JSON 报文（可精简示例值，保留结构）。

**不生成「验收用例」表格**（现有 ias-api-doc 文档没人用这一节）。

### 字典字段引用

已绑定数据字典的字段，在字段说明表的「取值范围」列列出枚举值，并在文档末尾加：
> 字段 `{fieldName}` 的取值范围见数据字典 `{dictCode}`。

整项目模式下，公共字段字典写到 `common/` 子目录，接口文档用相对链接引用。

## Step 6：写文件

用 Write 工具写入目标路径。整项目模式下逐接口生成 + 一份公共字典文件。

## Step 7：输出摘要

报告生成的文件路径列表。
