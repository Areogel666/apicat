---
name: apicat-test-gen
description: 为 ApiCat 本地库接口生成测试用例（7 种类型）并可选压测。支持从接口定义/文档/源码推断参数。触发词：生成测试用例、生成用例、测试用例、压测、apicat test、apicat-test-gen
allowed-tools: Bash, Read, Glob, Grep, AskUserQuestion
---

# ApiCat 测试用例生成

为 ApiCat 本地库接口生成 7 种类型用例，写入后可跑断言、可压测。先按 `apicat-lib` 定位 Bridge。

## Step 1：选项目

`AskUserQuestion` 选项卡让用户选（禁止打印列表让用户回复数字）：
```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_projects"
```

## Step 2：选接口

- 用户 prompt 里点名了接口（URL 或名称）→ `method + url` 匹配，不问
- 没点名 → 列接口让用户多选

```bash
# 遍历项目下所有接口
for cid in $(curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_collections?project_id=$PID" | jq -r '.data[].id'); do
  curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_requests?collection_id=$cid"
done
```

匹配不到 → 先问「要不要新建接口」再建。

## Step 3：选用例类型

先检查 prompt 是否已指定类型（指定则跳过）。未指定则 `AskUserQuestion` 多选：

| case_type | 中文 | 参数策略 | 断言 |
|-----------|------|---------|------|
| `happy_path` | 正常 | GET 填全量 params；POST 填全量 body | status 200 + 业务码（能推断才写） |
| `missing_required` | 缺必填 | 移除必填项，其余保留 | status 400 |
| `unauthorized` | 未授权 | 全量参数，headers 去 Authorization | status 401 |
| `boundary` | 边界值 | 全量保留，改被测参数值 | 200 或 400 视边界方向 |
| `empty_list` | 空列表 | 关键查询参数改为不存在的值 | 200 + `$.data` not_null |
| `type_error` | 类型错误 | string 传 int 等 | 400 |
| `invalid_chars` | 特殊字符 | 注入超长串/HTML/SQL 特殊字符 | 400 |

`auth_type='none'` 的接口自动跳过 `unauthorized`。

**依赖链用例不支持**（用例间无编排概念），不要生成。

## Step 4：读接口已有参数

用例参数以接口定义为基础。读接口详情：
```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/get_request?id=$RID"
```

**参数格式**（沿用接口的 `params` JSON 字符串）：
```jsonc
// GET params —— 拼 URL query
[{"key": "gpId", "value": "test-gaid", "enabled": true}]
// POST body_type="raw_json", body 是 JSON 字符串
"{\"channel\": \"home\", \"count\": 20}"
```

**核心原则「参数要全」**：Happy Path 须含全部 optional 业务参数（设备信息 lo/la/sdk 等），不只 required。

## Step 5：生成用例 JSON

每接口 × 每类型一条。组装 `create_test_case` 请求体：

```jsonc
{
  "requestId": 5,
  "collectionId": 2,
  "name": "[缺必填] 缺少 gpId",
  "method": "GET",
  "url": "{{base_url}}/apm/intl/recommend/migration",
  "headers": "[{\"key\":\"Authorization\",\"value\":\"Bearer x\",\"enabled\":true}]",
  "params": "[{\"key\":\"page\",\"value\":\"1\",\"enabled\":true}]",
  "bodyType": null,
  "body": null,
  "caseType": "missing_required",
  "assertions": "[{\"type\":\"status_code\",\"operator\":\"eq\",\"expected\":\"400\"}]"
}
```

## Step 6：写入（幂等）

同名检查：先查已有用例，同 `name` 则跳过。
```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_test_cases?request_id=$RID"
# 写入
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d @case.json "$BASE/create_test_case"
```

## Step 7：断言规则

- **status_code 必写**
- 业务码从接口描述/文档/样例推断；推断不出则**不写业务码断言**（避免模板假定的 `$.code eq 0` 落库误导）
- 断言格式见 `apicat-lib/references/apicat_schema.md`

## Step 8：压测（需求 5）

用户要求压测时：

1. 选 N 个用例（`AskUserQuestion` 多选，或用户指定）
2. **逐个压**：对每个用例，取其参数快照组装 `SendRequestParams`，调 `POST /start_stress`
3. 汇总 N 份报告

```bash
# 用例参数 → SendRequestParams
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "{\"requestId\": $RID, \"params\": {\"method\": \"GET\", \"url\": \"$URL\", \"queryParams\": $PARAMS, \"headers\": $HEADERS, \"bodyType\": \"$BT\", \"body\": \"$BODY\", \"pathParams\": [], \"authType\": \"none\", \"authConfig\": \"{}\"}, \"concurrent\": 10, \"mode\": \"count\", \"value\": 100}" \
  "$BASE/start_stress"
```

参数：`concurrent 1~500`、`mode: count|duration`、`value`（count=总请求数≤10000，duration=秒数）。

压测阻塞到结束，返回 `StressStats`（total/success/failed/success_rate/avg_ms/p50/p90/p95/p99/max_ms/tps/...）。

## Step 9：输出摘要

报告：项目名、新建/复用接口数、写入/跳过用例数、各类型用例列表。压测时追加各用例的统计摘要。

## 已知坑（必须遵守）

- params 必须 `{key, value, enabled}` 形状，**禁止** OpenAPI 的 `{name, in}`（前端按 `p.key` 读，会静默丢参）
- GET 参数进 `params`；POST/PUT 必须写 `body_type` + `body`
- 用例 `headers`/`params`/`assertions` 都是 **JSON 字符串**（不是数组对象）
- `collection_id` 必填（用例必须挂在某目录下）
