---
name: apicat-test-gen
description: 为 ApiCat 本地库接口生成测试用例（7 种类型）并可选压测。支持从接口定义/文档/源码推断参数。触发词：生成测试用例、生成用例、测试用例、压测、apicat test、apicat-test-gen
allowed-tools: Bash, Read, Glob, Grep, AskUserQuestion
---

# ApiCat 测试用例生成

为 ApiCat 本地库接口生成 7 种类型用例，写入后可跑断言、可压测。
先按 `apicat-lib` 的 Step 1 定位 Bridge。**参数格式、选项目方式等公共约定见 `apicat-lib` 的「共享约束」，本文不重复。**

## Step 1：选项目

用 `AskUserQuestion` 选项卡让用户选项目。

## Step 2：选接口

- 用户 prompt 里点名了接口（URL 或名称）→ `method + url` 匹配，不问
- 没点名 → 列接口让用户多选

```bash
# 先取目录列表，从返回里读出各目录 id
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_collections?project_id=$PID"
# 再逐个目录取接口
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_requests?collection_id=$CID"
```

目录多、要连续读十几条时，改用 `<skills>/apicat-lib/scripts/bridge_client.py` 循环（见 Step 6），比手工拼 shell 稳。

匹配不到 → 先问「要不要新建接口」再建（建接口见 `apicat-edit`）。

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

用例参数以接口定义为基础。读接口详情：`GET /get_request?id=$RID`。

参数形状沿用接口的 `params` JSON 字符串，例如 `[{"key": "userId", "value": "test-user-001", "enabled": true}]`；POST 则 `body_type="raw_json"` + body 为 JSON 字符串。

**核心原则「参数要全」**：Happy Path 须含全部 optional 业务参数（用户与设备标识等），不只 required。

**参数只能从接口定义推断不出时**，再去读源码/文档补（这也是本技能保留 Grep 工具的原因）；仍推断不出就别编，留空并在汇报里标注。

## Step 5：生成用例 JSON

每接口 × 每类型一条。组装 `create_test_case` 请求体：

```jsonc
{
  "requestId": 5,
  "collectionId": 2,
  "name": "[缺必填] 缺少 userId",
  "method": "GET",
  "url": "{{base_url}}/api/user/info",
  "headers": "[{\"key\":\"Authorization\",\"value\":\"Bearer x\",\"enabled\":true}]",
  "params": "[{\"key\":\"page\",\"value\":\"1\",\"enabled\":true}]",
  "bodyType": null,
  "body": null,
  "caseType": "missing_required",
  "assertions": "[{\"type\":\"status_code\",\"operator\":\"eq\",\"expected\":\"400\"}]"
}
```

## Step 6：写入

每接口 × 每类型一条，用 **Step 5 的 camelCase 载荷**逐条写入。用例挂在接口所在目录（`collectionId` 取 Step 5 里的值）。

**同名检查**：先查已有用例（`GET /list_test_cases?request_id=$RID`），同 `name` 则跳过。

```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d @case.json "$BASE/create_test_case"
```

**⚠️ `caseType` 必填**：后端对缺省值的兜底是 `happy_path`，漏传会把 7 种类型全标成 happy_path，**而且不报错**。

**用例多（≥3 条）时**用 `<skills>/apicat-lib/scripts/bridge_client.py` 循环写入（可选，需 Python；`<skills>` = 技能安装目录），比逐条拼 curl 稳：

```python
import sys; sys.path.insert(0, "<skills>/apicat-lib/scripts")
from bridge_client import ApiCatBridge
b = ApiCatBridge()
existing = {c["name"] for c in b.get("/list_test_cases", {"request_id": rid})}
for case in cases:                  # cases = Step 5 的 camelCase 载荷数组
    if case["name"] in existing:
        continue                    # 幂等：已存在的同名用例跳过
    b.post("/create_test_case", case)
    existing.add(case["name"])
```

## Step 7：断言规则

- **status_code 必写**
- 业务码从接口描述/文档/样例推断；推断不出则**不写业务码断言**（避免模板假定的 `$.code eq 0` 落库误导）
- 断言格式见 `apicat-lib/references/apicat_schema.md`

## Step 8：压测

用户要求压测时：

1. 选 N 个用例（`AskUserQuestion` 多选，或用户指定）
2. **逐个压**：对每个用例，取其参数快照组装 `SendRequestParams`，调 `POST /start_stress`
3. 汇总 N 份报告

```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d "{\"requestId\": $RID, \"params\": {\"method\": \"GET\", \"url\": \"$FULL_URL\", \"query_params\": $PARAMS, \"headers\": $HEADERS, \"body_type\": \"$BT\", \"body\": \"$BODY\", \"path_params\": [], \"auth_type\": \"none\", \"auth_config\": \"{}\"}, \"concurrent\": 10, \"mode\": \"count\", \"value\": 100}" \
  "$BASE/start_stress"
```

**⚠️ 压测的两个特殊点**（与普通写接口不同，容易踩）：
- `params` 内部字段必须 **snake_case**（`query_params`/`body_type`/`path_params`/`auth_type`/`auth_config`），camelCase 会 400
- **压测引擎不做 `{{base_url}}` 替换**：用例 URL 是 `{{base_url}}/api/x` 时必须先查环境拿 base_url 拼成完整地址再传

参数：`concurrent 1~500`、`mode: count|duration`、`value`（count=总请求数≤10000，duration=秒数）。
压测阻塞到结束，返回 `StressStats`（total/success/failed/success_rate/biz_success_rate/avg_ms/p50/p90/p95/p99/max_ms/tps/...）。

> `success_rate` 是**响应率**，`biz_success_rate` 才是**业务成功率**——两者含义不同，别混。详见 `apicat-lib/references/bridge-api.md`。

## Step 9：输出摘要

报告：项目名、新建/复用接口数、写入/跳过用例数、各类型用例列表。压测时追加各用例的统计摘要。

## 本技能特有约束

- `collection_id` 必填（用例必须挂在某目录下）

（`params` 形状、GET/POST 参数位置、JSON 字符串等公共约定见 `apicat-lib` 共享约束）
