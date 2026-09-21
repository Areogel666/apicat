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

**直接读文档提取，不要找解析脚本。** 文档格式千变万化（不同人写的结构不同），写死的解析脚本只能吃一种格式、换个写法就错位；你读 Markdown 比它准得多，也能适应格式变化。

### 提取规范

逐条接口提取，字段对齐 Bridge：

| 字段 | 从哪来 | 注意 |
|---|---|---|
| `name` | 接口标题 | 中文名；标题里只有路径时用路径 |
| `method` | 请求方式行 | **以接口自身的请求方式为准**，别被参数描述里的 `GET`/`POST` 枚举示例带偏 |
| `url` | 标题或域名行里的路径 | 以 `/` 开头 |
| `params` | 参数表 | 每行 → `{key, value:"", enabled:true, type, description}`，`type`/`description` 取表格对应列 |
| 枚举候选 | 描述列 | 形如 `` `0`=待支付 `` 的成对写法 → 可建字典 |

**先把提取结果给用户过一眼**（字段名/路径常需人工修正），再落库。

### 落库流程

1. 按 `url` 路径匹配已有目录（`GET /list_collections?project_id=N`）；匹配不到问用户是否 `POST /create_collection` 新建
2. **查重按 `method + url`，不能只按 `name`**。库里接口名不统一 —— 有的存中文名（如 `获取用户信息`），有的存路径（如 `/api/user/info`）；而文档里提的通常是中文名。只比 `name` 会把同一个接口误判成新的，**导入出重复项**。
   取 `GET /list_requests?collection_id=$CID`，`method + url` 完全一致即视为同一接口，问用户：**更新它 / 跳过 / 新建**。
3. 逐条 `POST /create_request`（只传 `collectionId`/`name`/`method`/`url`）
4. 紧接 `POST /update_request` 补 `params`
5. 抽到枚举 → 问用户是否建字典并绑定（见第三章）
6. 汇报：新建/复用目录、导入接口数、跳过项

接口多（>10 条）时，用 `<skills>/apicat-lib/scripts/bridge_client.py` 循环写入更稳（可选，需 Python）；少量直接 curl。

## 二、新建 / 更新单条接口

### 1. 选目录
`GET /list_collections?project_id=$PID`，按用户描述或文档模块结构匹配同名目录；找不到就 `POST /create_collection` 新建。**建完显式报告**：「已在 {目录名} 下新建接口」。

### 2. 查重
目标 collection 下先按 **`method + url`** 比对（接口名不统一，同一接口可能一个存中文名、一个存路径），再按 `name` 比对（`UNIQUE(collection_id, name)` 约束）。任一命中都**必须问用户**：
> 已存在同 URL / 同名的接口「{name}」。**更新它** / **跳过** / **换个名字新建**？

### 3. 创建
`POST /create_request` —— **只支持 `collectionId`/`name`/`method`/`url` 四个字段，传 params/headers/body 会被静默忽略**，建完必须再调 `update_request` 补参数。

### 4. 更新
`POST /update_request`，字段：`id`/`name`/`method`/`url`/`params`/`headers`/`bodyType`/`body`/`authType`/`authConfig`/`description`。
`body_type` 取值：`raw_json`（@RequestBody）| `form_urlencoded`（getParameter）。参数格式见共享约束。

## 三、数据字典 CRUD

### 新建字典 + 字典项（推荐一步到位）
`POST /create_dictionary_with_items`，字段：`code`（全局唯一）/`name`/`projectId`（**必填**，缺省 400——归属创建时的项目，缺省不落全局）/`items: [{label, value, description}]`。
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
