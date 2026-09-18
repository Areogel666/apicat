# ApiCat SQLite 表结构速查

> 同步自 `src-tauri/migrations/`，1.0.5 版本。Bridge 上线后一般不需要直接读 DB，此文档供理解数据模型。

## projects
| 列 | 类型 | 说明 |
|---|---|---|
| id | INTEGER PK | |
| name | TEXT NOT NULL | |
| description | TEXT | |
| docs_output_dir | TEXT | 1.0.5：doc-gen 输出目录（null=默认） |
| created_at / updated_at | DATETIME | |

## collections（目录树）
| 列 | 类型 | 说明 |
|---|---|---|
| id | INTEGER PK | |
| project_id | INTEGER FK→projects CASCADE | |
| parent_id | INTEGER FK→collections CASCADE | null=顶层 |
| name | TEXT NOT NULL | |
| sort_order | INTEGER | |

## api_requests（接口）
| 列 | 类型 | 说明 |
|---|---|---|
| id | INTEGER PK | |
| collection_id | INTEGER FK→collections CASCADE | |
| name | TEXT NOT NULL | UNIQUE(collection_id, name) |
| method | TEXT | GET/POST/... |
| url | TEXT | |
| params | TEXT | JSON 数组 `[{key,value,enabled,type?,description?}]` |
| headers | TEXT | 同上 |
| body_type | TEXT | none/raw_json/raw_text/form_urlencoded/form_data |
| body | TEXT | |
| auth_type | TEXT | none/bearer/basic/api_key |
| auth_config | TEXT | JSON |
| description | TEXT | 1.0.4 加列 |

## test_cases（用例）
| 列 | 类型 | 说明 |
|---|---|---|
| id | INTEGER PK | |
| request_id | INTEGER FK→api_requests SET NULL | 可空 |
| collection_id | INTEGER FK→collections CASCADE | 必填 |
| name | TEXT NOT NULL | |
| source | TEXT | manual/ai_generated |
| case_type | TEXT | 1.0.5：happy_path/missing_required/unauthorized/boundary/empty_list/type_error/invalid_chars |
| method/url/headers/params/body_type/body | | 用例自身参数快照（可偏离接口定义） |
| assertions | TEXT | JSON 数组，见下方断言格式 |
| last_run_at | DATETIME | |
| last_status | TEXT | pending/passed/failed/error |
| last_duration_ms | INTEGER | |
| last_response | TEXT | ≤1KB 摘要 |
| starred | INTEGER 0/1 | 首个用例自动 starred=1 |
| enabled | INTEGER 0/1 | |

## 断言 JSON 格式
```jsonc
[
  { "type": "status_code", "operator": "eq", "expected": "200" },
  { "type": "json_path", "path": "$.code", "operator": "eq", "expected": "0" },
  { "type": "json_path", "path": "$.data", "operator": "not_null", "expected": "" },
  { "type": "json_path", "path": "$.msg", "operator": "contains", "expected": "success" }
]
```
- `status_code`: eq / ne
- `json_path`: eq / ne / not_null / contains
- 路径支持 `$.a.b`、`$.list[0].field`

## data_dictionaries
| 列 | 类型 | 说明 |
|---|---|---|
| id | INTEGER PK | |
| code | TEXT UNIQUE | |
| name | TEXT | |
| project_id | INTEGER FK→projects CASCADE | null=全局共享 |
| builtin | INTEGER 0/1 | |

## dictionary_items
| 列 | 类型 | 说明 |
|---|---|---|
| id | INTEGER PK | |
| dictionary_id | INTEGER FK→data_dictionaries CASCADE | |
| label / value / description | TEXT | UNIQUE(dictionary_id, value) |

## field_dictionary_rules（项目级字段绑定）
UNIQUE(project_id, field_name)。字段名↔字典，一对一。

## field_dictionary_overrides（接口级例外，优先于规则）
UNIQUE(project_id, request_id, field_name)。dictionary_id=null 表示解绑。

## stress_runs
| 列 | 类型 | |
|---|---|---|
| request_id | INTEGER FK | |
| config_json | TEXT | {concurrent, mode, value} |
| stats_json | TEXT | StressStats |
