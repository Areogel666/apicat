---
name: apicat-edit
description: 增删改 ApiCat 本地库的数据字典、接口、字段绑定。触发词：加接口、新建接口、改接口、删接口、加字典、新建字典、字段绑定、关联字典、apicat edit
allowed-tools: Bash, Read, AskUserQuestion
---

# ApiCat 数据编辑

增删改 ApiCat 本地库的接口、数据字典、字段绑定。先按 `apicat-lib` 的 Step 1 定位 Bridge。

## 通用前置：选项目

**必须用 `AskUserQuestion` 选项卡让用户选项目**（禁止打印列表让用户回复数字）：

```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_projects"
```

## 一、新建 / 更新接口

### 1. 选目录
```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_collections?project_id=$PID"
```
按用户描述或文档模块结构匹配同名目录；找不到就 `POST /create_collection` 新建。**建完显式报告**：「已在 {目录名} 下新建接口」。

### 2. 同名检查
目标 collection 下已有同名接口（`UNIQUE(collection_id, name)` 约束）时，**必须问用户**：
> 同名接口「{name}」已存在。**更新它** / **跳过** / **换个名字新建**？

### 3. 创建
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "{\"collectionId\": $CID, \"name\": \"$NAME\", \"method\": \"$METHOD\", \"url\": \"$URL\"}" \
  "$BASE/create_request"
```

### 4. 更新（含参数/Body/描述）
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "{\"id\": $RID, \"name\": \"$NAME\", \"method\": \"$METHOD\", \"url\": \"$URL\", \"params\": $PARAMS_JSON, \"headers\": $HEADERS_JSON, \"bodyType\": \"$BODY_TYPE\", \"body\": \"$BODY\", \"authType\": \"none\", \"authConfig\": \"{}\", \"description\": \"$DESC\"}" \
  "$BASE/update_request"
```

**参数格式坑**：
- `params`/`headers` 是 **JSON 字符串**（不是数组对象），内含 `[{key, value, enabled}]`
- GET 参数进 `params`（拼 URL query）；POST/PUT 必须写 `body_type` + `body`
- `body_type`: `raw_json`（@RequestBody）| `form_urlencoded`（getParameter）

## 二、数据字典 CRUD

### 新建字典 + 字典项（推荐一步到位）
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "{\"code\": \"$CODE\", \"name\": \"$NAME\", \"projectId\": $PID, \"items\": [{\"label\": \"标签\", \"value\": \"1\", \"description\": \"说明\"}]}" \
  "$BASE/create_dictionary_with_items"
```
同名 `code` 已存在时问用户：更新字典项 / 跳过。

### 全量替换字典项
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "{\"dictionaryId\": $DID, \"items\": [...]}" "$BASE/replace_dictionary_items"
```

## 三、字段绑定（需求 6）

### 默认作用域：**必须先问用户**
> 「{fieldName}」绑定到字典「{dictName}」——**项目级规则**（影响所有接口）还是**仅当前接口**？

- 项目级 → `POST /set_field_rule`
- 仅当前接口 → `POST /set_field_override`

```bash
# 项目级
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "{\"projectId\": $PID, \"fieldName\": \"$FIELD\", \"dictionaryId\": $DID}" "$BASE/set_field_rule"

# 接口级（dictionaryId=null 表示解绑）
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "{\"projectId\": $PID, \"requestId\": $RID, \"fieldName\": \"$FIELD\", \"dictionaryId\": $DID}" "$BASE/set_field_override"
```

### 查当前绑定
```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_field_rules?project_id=$PID"
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_field_overrides?project_id=$PID&request_id=$RID"
```

## 完成后

报告变更摘要：新建/更新了哪些接口、字典、绑定。App 内 UI 已自动刷新。
