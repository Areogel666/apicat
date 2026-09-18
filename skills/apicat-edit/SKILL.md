---
name: apicat-edit
description: 增删改 ApiCat 本地库的数据字典、接口、字段绑定，以及从 Markdown 接口文档批量导入接口。触发词：加接口、新建接口、改接口、删接口、加字典、新建字典、字段绑定、关联字典、导入接口、从文档导入接口、把文档导进 apicat、apicat edit
allowed-tools: Bash, Read, Glob, AskUserQuestion
---

# ApiCat 数据编辑

增删改 ApiCat 本地库的接口、数据字典、字段绑定，支持从接口文档批量导入。
先按 `apicat-lib` 的 Step 1 定位 Bridge。**参数格式、选项目方式等公共约定见 `apicat-lib` 的「共享约束」，本文不重复。**
各端点的完整 curl 见 `apicat-lib/references/bridge-api.md`；下文只给关键参数签名。

## 一、从文档批量导入接口

手上有 ias-api-doc 风格的 Markdown 接口文档、要导入**多条**接口时，不要一条条手搓 curl，先跑解析脚本：

```bash
python "<skills>/apicat-edit/scripts/parse_markdown_docs.py" <doc.md> --project-id $PID
```

**⚠️ Windows(Git Bash) 传路径用相对路径或 `D:/...` 形式**，别用 `/tmp/x.md` 这类 POSIX 路径——MSYS 会把它改写成 Git 安装目录下的路径，导致「文件不存在」。

输出每条接口的 `name`/`method`/`url`/`params`（含 type/description/required），外加 `enum_candidates` —— 从 `` `0`=非广告 `` 这类字段描述里抽出的枚举候选，可直接拿去建字典。

**导入流程**：
1. 跑脚本拿 JSON，**先把解析结果给用户过一眼**（字段名/路径常需人工修正）
2. 按 `url` 路径匹配已有目录（`GET /list_collections?project_id=N`）；匹配不到问用户是否 `POST /create_collection` 新建
3. 逐条 `POST /create_request`（只传 `collectionId`/`name`/`method`/`url`）
4. 紧接 `POST /update_request` 补 `params` —— 脚本输出的 `{key,value,enabled,type,description}` 形状可直接用
5. `enum_candidates` 非空 → 问用户是否建字典并绑定（见第三章）
6. 汇报：新建/复用目录、导入接口数、跳过项

脚本只解析不写库，落库动作全部由你按上述流程执行。

## 二、新建 / 更新单条接口

### 1. 选目录
`GET /list_collections?project_id=$PID`，按用户描述或文档模块结构匹配同名目录；找不到就 `POST /create_collection` 新建。**建完显式报告**：「已在 {目录名} 下新建接口」。

### 2. 同名检查
目标 collection 下已有同名接口（`UNIQUE(collection_id, name)` 约束）时，**必须问用户**：
> 同名接口「{name}」已存在。**更新它** / **跳过** / **换个名字新建**？

### 3. 创建
`POST /create_request` —— **只支持 `collectionId`/`name`/`method`/`url` 四个字段，传 params/headers/body 会被静默忽略**，建完必须再调 `update_request` 补参数。

### 4. 更新
`POST /update_request`，字段：`id`/`name`/`method`/`url`/`params`/`headers`/`bodyType`/`body`/`authType`/`authConfig`/`description`。
`body_type` 取值：`raw_json`（@RequestBody）| `form_urlencoded`（getParameter）。参数格式见共享约束。

## 三、数据字典 CRUD

### 新建字典 + 字典项（推荐一步到位）
`POST /create_dictionary_with_items`，字段：`code`（全局唯一）/`name`/`projectId`/`items: [{label, value, description}]`。
`code` 已存在时问用户：更新字典项 / 跳过。

### 全量替换字典项
`POST /replace_dictionary_items`，字段：`dictionaryId` + `items: [...]`（全量覆盖，不是增量）。

### 查字典
`GET /list_dictionaries?project_id=$PID` → `GET /list_dictionary_items?dictionary_id=$DID`

## 四、字段绑定

### 默认作用域：**必须先问用户**
> 「{fieldName}」绑定到字典「{dictName}」——**项目级规则**（影响所有接口）还是**仅当前接口**？

- 项目级 → `POST /set_field_rule`，字段：`projectId`/`fieldName`/`dictionaryId`
- 仅当前接口 → `POST /set_field_override`，字段：`projectId`/`requestId`/`fieldName`/`dictionaryId`

**`dictionaryId: null` 表示解绑。**

### 查当前绑定
```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_field_rules?project_id=$PID"
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_field_overrides?project_id=$PID&request_id=$RID"
```

## 完成后

报告变更摘要：新建/更新了哪些接口、字典、绑定。App 内 UI 已自动刷新。
