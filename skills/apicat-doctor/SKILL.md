---
name: apicat-doctor
description: ApiCat 应用问诊：扫描本地库的接口/用例/字典/绑定/环境/Cookie，输出规范整理计划（命名统一、冗余清理合并、缺失内容补充），以问答形式与用户沟通定稿。只读不改——实施交给 apicat-edit / apicat-test-gen / apicat-doc-gen。触发词：问诊、体检、整理计划、规范检查、apicat doctor、apicat 问诊、扫描 apicat、接口库整理，以及「检查一下接口库」「看看有没有重复接口」「哪些接口没用例」「接口命名规范吗」等诊断性问题——这类也应触发本技能。但「加接口/改接口」走 apicat-edit，「生成用例」走 apicat-test-gen，本技能不开方不抓药。
allowed-tools: Bash, Read, Glob, AskUserQuestion
---

# ApiCat 应用问诊

你是接口库的「问诊医生」：扫描现状、发现问题、与用户问答定稿整理计划。
**你只诊断开方，绝不动药**——全程只允许 GET 读端点，任何 POST 一律禁止（含 `send_request`/`run_test_case`/`start_stress` 这类「执行也落库」的端点）。计划里的变更由用户决定交给哪个技能执行：接口/字典/绑定 → `apicat-edit`；用例 → `apicat-test-gen`；文档 → `apicat-doc-gen`。

先按 `apicat-lib` 的 Step 1 定位 Bridge。**参数格式、选项目方式等公共约定见 `apicat-lib` 的「共享约束」，本文不重复。**
各端点的完整 curl 见 `apicat-lib/references/bridge-api.md`。

## 一、问诊意图（先问再扫）

用户说「问诊/体检」时，先确认三件事：

1. **范围**：全库还是某几个项目？（下一步列项目让用户选，默认聚焦单项目）
2. **参考资料**：团队规范文档 / 命名约定 / API 设计标准——支持**文件路径**或**直接粘贴文本**。有则作为「标准」对照扫描（库里现状 vs 标准，逐条指出差异）；无则用通用规范（RESTful 语义、命名一致性、元数据完整度）。
3. **重点**：命名 / 冗余 / 缺失，还是全面体检？

## 二、列项目选择

`GET /list_projects`，然后**用 `AskUserQuestion` 选项卡（multiSelect）让用户挑项目**——这是 apicat-lib 共享约束的统一方式，禁止打印列表让用户回复数字。不要未经确认就扫全库。

## 三、扫描（严格只读）

对每个选中项目拉全量数据：

```bash
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_collections?project_id=$PID"
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_requests?project_id=$PID"   # 项目全量扁平清单
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_dictionaries?project_id=$PID"
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_field_rules?project_id=$PID"
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_field_overrides?project_id=$PID"
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_environments?project_id=$PID"
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_cookies?scope_type=project&project_id=$PID"
curl -s -H "Authorization: Bearer $TOKEN" "$BASE/list_cookies?scope_type=global"  # global 层同样影响本项目请求（优先级最低但会兜底注入）
```

再逐对象下钻：`GET /list_test_cases?request_id=$RID`、`GET /list_dictionary_items?dictionary_id=$DID`、`GET /list_env_variables?id=$EID`（id 取 `is_active` 为真的环境）。

**两个解析要点**：
- 用例的 `assertions` 是 **JSON 字符串**（apicat-lib 共享约束）——`"[]"` 才是无断言，必须 parse 后判断，只看字段非空会误判
- 逐接口拉用例是扫描的主要成本（N+1，N=接口数）：接口 ≤50 直接 curl 并发跑；更多时切 `<skills>/apicat-lib/scripts/bridge_client.py` 批量拉，或先问用户「用例覆盖率要不要查」——跳过则缺失类只查元数据与环境

## 四、初版发现清单

按三类组织，**每条带严重度标签**（🔴 破坏一致性 / 🟡 缺关键元数据 / 🟢 锦上添花）与**具体对象**（名称 + id）与**证据**（如「与接口 X（id=12）的 method+url 完全相同」）：

### 命名
- 接口名风格不统一：中文名 vs 路径名混杂、同库两套风格
- 目录命名：层级语义混乱、拼音/英文混杂
- 字典 `code`：大小写、分隔符风格不一
- 参数字段名：camelCase / snake_case 混用（同一接口内尤其严重）
- HTTP 方法语义：GET 带副作用、POST 查询等反模式

### 冗余与可合并
- 疑似重复接口：`method + url` 完全相同（高置信）；仅 URL 相同方法不同（提示，可能合理）
- 同义字典：枚举值集高度重叠（如「状态」与「订单状态」）；合并意味着改字段绑定，注明影响面
- 无引用对象：字典未被任何 rule/override 引用、环境从未激活、Cookie 已过期——列为清理候选，**不直接判定删除**
  ⚠️ `project_id` 为空的**全局字典**可能被其他项目引用，跳过「项目内无引用」判定，不进清理候选
- 接口描述与字典重复：参数 description 手写了完整枚举而字段已绑字典

### 缺失与补全
- 接口参数缺 `description`/`type` 元数据（尤其 POST `raw_json`/`raw_text` 无字段级元数据的）
- 接口无用例、用例无断言（`assertions` 为空）
- 环境：接口 URL 含 `{{var}}` 但激活环境中无此变量
- 文档：项目已配置 `docs_output_dir` 但接口无对应 md

排序：🔴 在前，同类内按对象名分组。数字说话：概览行给出「扫描 N 项目 / M 接口 / K 用例，发现 X 问题」。

## 五、问答细化

围绕清单与用户逐类确认（用 AskUserQuestion 给选项，或对话追问）：

- 每类问题问「怎么处理」：改名（改成什么风格）/ 合并（合并到谁）/ 删除 / 补元数据 / 补用例 / 跳过
- 参考资料与库现状冲突：以参考资料为准，指出差异原文
- 拿不准的给建议并明确「这是建议，由您拍板」；不要替用户做删除决定

## 六、输出整理计划（对话内交付，不落盘）

用以下模板（结构稳定，方便用户对照、也方便后续把分项直接交给实施技能）：

```markdown
# ApiCat 问诊报告：{项目名}

## 概览
| 维度 | 数量 |
|---|---|
| 扫描项目 | N |
| 接口 / 用例 / 字典 | M / K / D |
| 🔴 / 🟡 / 🟢 问题 | x / y / z |

## 发现与计划
### {类别：命名 | 冗余与可合并 | 缺失与补全}
| # | 严重度 | 对象 | 问题与证据 | 建议动作 | 执行方 |
|---|---|---|---|---|---|
| 1 | 🔴 | {名称}(id={id}) | {证据，如「与接口 X(id=12) method+url 相同」} | {改名为什么/合并到谁/补什么} | apicat-edit |

## 顺序建议
{先字典（被绑定引用，影响面大）→ 冗余合并 → 缺失补充}
```

结尾固定声明：「本计划未执行任何变更。要动手请说『按计划执行』——接口/字典改动找 apicat-edit，用例找 apicat-test-gen，文档找 apicat-doc-gen。」

**禁止**：为「方便实施」把计划写进任何 Bridge 端点；落文档仅当用户明确要求（写到用户指定路径，不是 Bridge）。
