---
name: apicat-doc-gen
description: 根据 ApiCat 接口定义 + 用例生成格式化接口文档（Markdown，含富字段表 + 多场景 Response）。触发词：生成接口文档、接口文档、出文档、apicat doc、apicat-doc-gen
allowed-tools: Bash, Read, Write, Glob, AskUserQuestion
---

# ApiCat 接口文档生成

从 ApiCat 读接口定义和用例，生成标准 Markdown 接口文档。
先按 `apicat-lib` 的 Step 1 定位 Bridge。**选项目方式等公共约定见 `apicat-lib` 的「共享约束」。**

**格式规范已内嵌在本技能 Step 5，不需要读任何外部模板项目。**

## Step 1：选项目

用 `AskUserQuestion` 选项卡让用户选项目。

## Step 2：确定范围

- **单接口**（默认）：用户点名或问用户选一个接口
- **整项目**（用户说「全部」或 `--all`）：遍历项目下所有接口

## Step 3：读数据

要连读 5 个端点。**curl 逐个读也行**（零依赖）；装了 Python 的话用底座客户端批量读更省事（**可选**，`<skills>` = 技能安装目录）：

```python
import sys; sys.path.insert(0, "<skills>/apicat-lib/scripts")
from bridge_client import ApiCatBridge
b = ApiCatBridge()
req    = b.get("/get_request", {"id": rid})                      # 接口详情
cases  = b.get("/list_test_cases", {"request_id": rid})          # 用例 → 多场景 Response
rules  = b.get("/list_field_rules", {"project_id": pid})         # 字段绑定
dicts  = b.get("/list_dictionaries", {"project_id": pid})        # 字典列表
items  = b.get("/list_dictionary_items", {"dictionary_id": did}) # 字典项
```

`get` 的第二个参数是 query 参数字典，返回的已是 `data` 字段（外壳已剥）。

## Step 4：确定输出目录

**用户显式指定的路径优先于项目配置**：

1. 用户指定了路径 → 用它
2. 否则读项目 `docs_output_dir`（`list_projects` 返回的 snake_case 字段）→ 有值就用它
3. 都没有 → 默认 `~/.apicat/apidoc/{project_name}/`

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
| userId | string | 是 | 用户 ID |

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
| 20000 | 参数校验失败 | userId 缺失 |

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

**不生成「验收用例」表格**（实际文档中没人用这一节，用例只用于生成 Response 示例）。

### 字典字段引用

已绑定数据字典的字段，在字段说明表的「取值范围」列列出枚举值，并在文档末尾加：
> 字段 `{fieldName}` 的取值范围见数据字典 `{dictCode}`。

整项目模式下，公共字段字典写到 `common/` 子目录，接口文档用相对链接引用。

## Step 6：写文件

用 Write 工具写入目标路径。整项目模式下逐接口生成 + 一份公共字典文件。

## Step 7：输出摘要

报告生成的文件路径列表。
